// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnx_types::{
    credentials::{keys::ClientSigningKey, ClientCredentialPayload},
    crypto::{
        kdf::keys::RatchetSecret,
        opaque::{
            OpaqueLoginFinish, OpaqueLoginRequest, OpaqueRegistrationRecord,
            OpaqueRegistrationRequest,
        },
        signatures::signable::Signable,
        RatchetEncryptionKey,
    },
    identifiers::{AsClientId, UserName},
    messages::{
        client_as::{
            AsCredentialsParams, AsPublishConnectionPackagesParamsTbs, AsRequestParams,
            ClientConnectionPackageParamsTbs, ClientToAsMessage, ConnectionPackage,
            DeleteClientParamsTbs, DeleteUserParamsTbs, DequeueMessagesParamsTbs,
            EncryptedConnectionEstablishmentPackage, EnqueueMessageParams,
            FinishClientAdditionParams, FinishClientAdditionParamsTbs,
            FinishUserRegistrationParamsTbs, Init2FactorAuthParamsTbs, Init2FactorAuthResponse,
            InitUserRegistrationParams, InitiateClientAdditionParams, IssueTokensParamsTbs,
            IssueTokensResponse, UserClientsParams, UserConnectionPackagesParams,
        },
        client_as_out::{
            AsClientConnectionPackageResponseIn, AsCredentialsResponseIn, AsProcessResponseIn,
            ConnectionPackageIn, InitClientAdditionResponseIn, InitUserRegistrationResponseIn,
            UserClientsResponseIn, UserConnectionPackagesResponseIn,
        },
        client_qs::DequeueMessagesResponse,
        AsTokenType,
    },
};
use phnxbackend::auth_service::errors::AsProcessingError;
use phnxserver::endpoints::ENDPOINT_AS;
use privacypass::batched_tokens::TokenRequest;
use thiserror::Error;
use tls_codec::{DeserializeBytes, Serialize};

use crate::{ApiClient, Protocol};

#[derive(Error, Debug)]
pub enum AsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Couldn't deserialize response body.")]
    BadResponse,
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error(transparent)]
    AsError(#[from] AsProcessingError),
}

impl ApiClient {
    async fn prepare_and_send_as_message(
        &self,
        message: ClientToAsMessage,
    ) -> Result<AsProcessResponseIn, AsRequestError> {
        let message_bytes = message
            .tls_serialize_detached()
            .map_err(|_| AsRequestError::LibraryError)?;
        let url = self.build_url(Protocol::Http, ENDPOINT_AS);
        let res = self
            .client
            .post(url.clone())
            .body(message_bytes)
            .send()
            .await;
        match res {
            Ok(res) => {
                match res.status().as_u16() {
                    // Success!
                    x if (200..=299).contains(&x) => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| AsRequestError::BadResponse)?;
                        let ds_proc_res =
                            AsProcessResponseIn::tls_deserialize_exact(&ds_proc_res_bytes)
                                .map_err(|_| AsRequestError::BadResponse)?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes =
                            res.bytes().await.map_err(|_| AsRequestError::BadResponse)?;
                        let ds_proc_err =
                            AsProcessingError::tls_deserialize_exact(&ds_proc_err_bytes)
                                .map_err(|_| AsRequestError::BadResponse)?;
                        Err(AsRequestError::AsError(ds_proc_err))
                    }
                    // All other errors
                    other_status => {
                        let error_text =
                            res.text().await.map_err(|_| AsRequestError::BadResponse)?
                                + &format!(" (status code {})", other_status);
                        Err(AsRequestError::NetworkError(error_text))
                    }
                }
            }
            // A network error occurred.
            Err(err) => {
                let error_message = format!(
                    "Got a POST message error while contacting the URL {}: {:?}",
                    url, err
                );
                log::error!("{}", error_message);
                Err(AsRequestError::NetworkError(err.to_string()))
            }
        }
    }

    pub async fn as_initiate_create_user(
        &self,
        client_payload: ClientCredentialPayload,
        opaque_registration_request: OpaqueRegistrationRequest,
    ) -> Result<InitUserRegistrationResponseIn, AsRequestError> {
        let payload = InitUserRegistrationParams {
            client_payload,
            opaque_registration_request,
        };
        let params = AsRequestParams::InitUserRegistration(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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

    pub async fn as_initiate_2fa_auth(
        &self,
        client_id: AsClientId,
        opaque_ke1: OpaqueLoginRequest,
        signing_key: &ClientSigningKey,
    ) -> Result<Init2FactorAuthResponse, AsRequestError> {
        let tbs = Init2FactorAuthParamsTbs {
            client_id,
            opaque_ke1,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::Initiate2FaAuthentication(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::Init2FactorAuth(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_finish_user_registration(
        &self,
        queue_encryption_key: RatchetEncryptionKey,
        initial_ratchet_secret: RatchetSecret,
        connection_packages: Vec<ConnectionPackage>,
        opaque_registration_record: OpaqueRegistrationRecord,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let tbs = FinishUserRegistrationParamsTbs {
            client_id: signing_key.credential().identity(),
            queue_encryption_key,
            initial_ratchet_secret,
            connection_packages,
            opaque_registration_record,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::FinishUserRegistration(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        user_name: UserName,
        client_id: AsClientId,
        opaque_finish: OpaqueLoginFinish,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let tbs = DeleteUserParamsTbs {
            client_id,
            user_name,
            opaque_finish,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::DeleteUser(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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

    pub async fn as_initiate_client_addition(
        &self,
        client_credential_payload: ClientCredentialPayload,
        opaque_login_request: OpaqueLoginRequest,
    ) -> Result<InitClientAdditionResponseIn, AsRequestError> {
        let payload = InitiateClientAdditionParams {
            client_credential_payload,
            opaque_login_request,
        };
        let params = AsRequestParams::InitiateClientAddition(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let AsProcessResponseIn::InitiateClientAddition(response) = response {
                    Ok(response)
                } else {
                    Err(AsRequestError::UnexpectedResponse)
                }
            })
    }

    pub async fn as_finish_client_addition(
        &self,
        client_id: AsClientId,
        queue_encryption_key: RatchetEncryptionKey,
        initial_ratchet_key: RatchetSecret,
        connection_package: ConnectionPackageIn,
        opaque_login_finish: OpaqueLoginFinish,
    ) -> Result<(), AsRequestError> {
        // This is called TBS, but isn't signed yet. It will be signed by the
        // client as soon as we support client cross-signing.
        let tbs = FinishClientAdditionParamsTbs {
            client_id,
            queue_encryption_key,
            initial_ratchet_secret: initial_ratchet_key,
            connection_package,
        };
        let payload = FinishClientAdditionParams {
            opaque_login_finish,
            payload: tbs,
        };
        let params = AsRequestParams::FinishClientAddition(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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

    pub async fn as_delete_client(
        &self,
        client_id: AsClientId,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        // TODO: This means that clients can only ever delete themselves. Is
        // that what we want here?
        let tbs = DeleteClientParamsTbs(client_id);
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::DeleteClient(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
            sender: signing_key.credential().identity(),
            sequence_number_start,
            max_message_number,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::DequeueMessages(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        connection_packages: Vec<ConnectionPackageIn>,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let tbs = AsPublishConnectionPackagesParamsTbs {
            client_id,
            connection_packages,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::PublishConnectionPackages(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::ClientConnectionPackage(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
            client_id: signing_key.credential().identity(),
            token_type,
            token_request,
        };
        let payload = tbs
            .sign(signing_key)
            .map_err(|_| AsRequestError::LibraryError)?;
        let params = AsRequestParams::IssueTokens(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        user_name: UserName,
    ) -> Result<UserClientsResponseIn, AsRequestError> {
        let payload = UserClientsParams { user_name };
        let params = AsRequestParams::UserClients(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        let params = AsRequestParams::UserConnectionPackages(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        let params = AsRequestParams::EnqueueMessage(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
        let params = AsRequestParams::AsCredentials(payload);
        let message = ClientToAsMessage::new(params);
        self.prepare_and_send_as_message(message)
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
}
