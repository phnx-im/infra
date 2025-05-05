// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use http::StatusCode;
use phnxtypes::{
    LibraryError,
    credentials::{ClientCredentialPayload, keys::ClientSigningKey},
    crypto::{
        RatchetEncryptionKey, indexed_aead::keys::UserProfileKeyIndex, kdf::keys::RatchetSecret,
        signatures::signable::Signable,
    },
    endpoint_paths::ENDPOINT_AS,
    errors::version::VersionError,
    identifiers::{AsClientId, QualifiedUserName},
    messages::{
        AsTokenType,
        client_as::{
            AsCredentialsParams, AsPublishConnectionPackagesParamsTbs, AsRequestParamsOut,
            AsVersionedRequestParamsOut, ClientConnectionPackageParamsTbs, ClientToAsMessageOut,
            ConnectionPackage, DeleteUserParamsTbs, DequeueMessagesParamsTbs,
            EncryptedConnectionEstablishmentPackage, EnqueueMessageParams,
            InitUserRegistrationParams, IssueTokensParamsTbs, IssueTokensResponse,
            SUPPORTED_AS_API_VERSIONS, UserClientsParams, UserConnectionPackagesParams,
        },
        client_as_out::{
            AsClientConnectionPackageResponseIn, AsCredentialsResponseIn, AsProcessResponseIn,
            AsVersionedProcessResponseIn, EncryptedUserProfile, GetUserProfileParams,
            GetUserProfileResponse, InitUserRegistrationResponseIn, MergeUserProfileParamsTbs,
            StageUserProfileParamsTbs, UserClientsResponseIn, UserConnectionPackagesResponseIn,
        },
        client_qs::DequeueMessagesResponse,
    },
};
use privacypass::batched_tokens_ristretto255::TokenRequest;
use thiserror::Error;
use tls_codec::{DeserializeBytes, Serialize};
use tracing::error;

use crate::{
    ApiClient, Protocol,
    version::{extract_api_version_negotiation, negotiate_api_version},
};

#[derive(Error, Debug)]
pub enum AsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Tls(#[from] tls_codec::Error),
    #[error("Received an unexpected response type")]
    UnexpectedResponse,
    #[error("API Error: {0}")]
    Api(#[from] VersionError),
    #[error("AS Error: {0}")]
    AsError(String),
    #[error("Unsuccessful response: status = {status}, error = {error}")]
    RequestFailed { status: StatusCode, error: String },
}

impl From<LibraryError> for AsRequestError {
    fn from(_: LibraryError) -> Self {
        AsRequestError::LibraryError
    }
}

impl ApiClient {
    async fn prepare_and_send_as_message(
        &self,
        request_params: AsRequestParamsOut,
    ) -> Result<AsProcessResponseIn, AsRequestError> {
        let api_version = self.negotiated_versions().as_api_version();

        let request_params =
            AsVersionedRequestParamsOut::with_version(request_params, api_version)?;
        let message = ClientToAsMessageOut::new(request_params);

        let response = self.send_as_http_request(&message).await?;

        // check if we need to negotiate a new API version
        let Some(accepted_versions) = extract_api_version_negotiation(&response) else {
            return handle_as_response(response).await;
        };

        let supported_versions = SUPPORTED_AS_API_VERSIONS;
        let accepted_version = negotiate_api_version(accepted_versions, supported_versions)
            .ok_or_else(|| VersionError::new(api_version, supported_versions))?;
        self.negotiated_versions()
            .set_as_api_version(accepted_version);

        let (request_params, _) = message.into_body().change_version(accepted_version)?;
        let message = ClientToAsMessageOut::new(request_params);

        let response = self.send_as_http_request(&message).await?;
        handle_as_response(response).await
    }

    pub async fn as_initiate_create_user(
        &self,
        client_payload: ClientCredentialPayload,
        queue_encryption_key: RatchetEncryptionKey,
        initial_ratchet_secret: RatchetSecret,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Result<InitUserRegistrationResponseIn, AsRequestError> {
        let payload = InitUserRegistrationParams {
            client_payload,
            queue_encryption_key,
            initial_ratchet_secret,
            encrypted_user_profile,
        };
        let params = AsRequestParamsOut::InitUserRegistration(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::InitUserRegistration(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_get_user_profile(
        &self,
        client_id: AsClientId,
        key_index: UserProfileKeyIndex,
    ) -> Result<GetUserProfileResponse, AsRequestError> {
        let payload = GetUserProfileParams {
            client_id,
            key_index,
        };
        let params = AsRequestParamsOut::GetUserProfile(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::GetUserProfile(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_stage_user_profile(
        &self,
        client_id: AsClientId,
        signing_key: &ClientSigningKey,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Result<(), AsRequestError> {
        let payload = StageUserProfileParamsTbs {
            client_id,
            user_profile: encrypted_user_profile,
        }
        .sign(signing_key)?;
        let params = AsRequestParamsOut::StageUserProfile(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if matches!(response, AsProcessResponseIn::Ok) {
                    Ok(())
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_merge_user_profile(
        &self,
        client_id: AsClientId,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = MergeUserProfileParamsTbs { client_id }.sign(signing_key)?;
        let params = AsRequestParamsOut::MergeUserProfile(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if matches!(response, AsProcessResponseIn::Ok) {
                    Ok(())
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_delete_user(
        &self,
        user_name: QualifiedUserName,
        client_id: AsClientId,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let tbs = DeleteUserParamsTbs {
            client_id,
            user_name,
        };
        let payload = tbs.sign(signing_key)?;
        let params = AsRequestParamsOut::DeleteUser(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if matches!(response, AsProcessResponseIn::Ok) {
                    Ok(())
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_dequeue_messages(
        &self,
        sequence_number_start: u64,
        max_message_number: u64,
        signing_key: &ClientSigningKey,
    ) -> Result<DequeueMessagesResponse, AsRequestError> {
        let tbs = DequeueMessagesParamsTbs {
            sender: signing_key.credential().identity().clone(),
            sequence_number_start,
            max_message_number,
        };
        let payload = tbs.sign(signing_key)?;
        let params = AsRequestParamsOut::DequeueMessages(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::DequeueMessages(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_publish_connection_packages(
        &self,
        client_id: AsClientId,
        connection_packages: Vec<ConnectionPackage>,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let tbs = AsPublishConnectionPackagesParamsTbs {
            client_id,
            connection_packages,
        };
        let payload = tbs.sign(signing_key)?;
        let params = AsRequestParamsOut::PublishConnectionPackages(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if matches!(response, AsProcessResponseIn::Ok) {
                    Ok(())
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    // TODO: Verify that this fetches the correct key packages. I believe right
    // now it expects the signature to be from the client with the given client
    // id, which doesn't make a lot of sense.
    pub async fn as_client_connection_packages(
        &self,
        client_id: AsClientId,
        signing_key: &ClientSigningKey,
    ) -> Result<AsClientConnectionPackageResponseIn, AsRequestError> {
        let tbs = ClientConnectionPackageParamsTbs(client_id);
        let payload = tbs.sign(signing_key)?;
        let params = AsRequestParamsOut::ClientConnectionPackage(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::ClientConnectionPackage(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_issue_tokens(
        &self,
        token_type: AsTokenType,
        token_request: TokenRequest,
        signing_key: &ClientSigningKey,
    ) -> Result<IssueTokensResponse, AsRequestError> {
        let tbs = IssueTokensParamsTbs {
            client_id: signing_key.credential().identity().clone(),
            token_type,
            token_request,
        };
        let payload = tbs.sign(signing_key)?;
        let params = AsRequestParamsOut::IssueTokens(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::IssueTokens(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_user_clients(
        &self,
        user_name: QualifiedUserName,
    ) -> Result<UserClientsResponseIn, AsRequestError> {
        let payload = UserClientsParams { user_name };
        let params = AsRequestParamsOut::UserClients(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::UserClients(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_user_connection_packages(
        &self,
        payload: UserConnectionPackagesParams,
    ) -> Result<UserConnectionPackagesResponseIn, AsRequestError> {
        let params = AsRequestParamsOut::UserConnectionPackages(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::UserConnectionPackages(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_enqueue_message(
        &self,
        client_id: AsClientId,
        connection_establishment_ctxt: EncryptedConnectionEstablishmentPackage,
    ) -> Result<(), AsRequestError> {
        let payload = EnqueueMessageParams {
            client_id,
            connection_establishment_ctxt,
        };
        let params = AsRequestParamsOut::EnqueueMessage(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if matches!(response, AsProcessResponseIn::Ok) {
                    Ok(())
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_as_credentials(&self) -> Result<AsCredentialsResponseIn, AsRequestError> {
        let payload = AsCredentialsParams {};
        let params = AsRequestParamsOut::AsCredentials(payload);
        self.prepare_and_send_as_message(params)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::AsCredentials(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    async fn send_as_http_request(
        &self,
        message: &ClientToAsMessageOut,
    ) -> Result<reqwest::Response, AsRequestError> {
        let message_bytes = message.tls_serialize_detached()?;
        let endpoint = self.build_url(Protocol::Http, ENDPOINT_AS);
        let response = self
            .client
            .post(endpoint)
            .body(message_bytes)
            .send()
            .await?;
        Ok(response)
    }
}

async fn handle_as_response(res: reqwest::Response) -> Result<AsProcessResponseIn, AsRequestError> {
    let status = res.status();
    match status.as_u16() {
        // Success!
        _ if res.status().is_success() => {
            let bytes = res.bytes().await?;
            let response = AsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&bytes)?
                .into_unversioned()?;
            Ok(response)
        }
        // AS Specific Error
        418 => {
            let error = res
                .text()
                .await
                .unwrap_or_else(|error| format!("unprocessable response body due to: {error}"));
            Err(AsRequestError::AsError(error))
        }
        // All other errors
        _ => {
            let error = res
                .text()
                .await
                .unwrap_or_else(|error| format!("unprocessable response body due to: {error}"));
            Err(AsRequestError::RequestFailed { status, error })
        }
    }
}
