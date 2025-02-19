// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use api_migrations::migrate_qs_process_response;
use http::StatusCode;
use mls_assist::openmls::prelude::KeyPackage;
use phnxtypes::{
    crypto::{
        ear::keys::KeyPackageEarKey,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientSigningKey, QsClientVerifyingKey, QsUserSigningKey},
            signable::Signable,
            traits::SigningKeyBehaviour,
        },
        RatchetEncryptionKey,
    },
    endpoint_paths::ENDPOINT_QS,
    identifiers::{QsClientId, QsUserId},
    messages::{
        client_qs::{
            ClientKeyPackageParams, ClientKeyPackageResponse, ClientToQsMessageTbs,
            CreateClientRecordResponse, CreateUserRecordResponse, DeleteClientRecordParams,
            DeleteUserRecordParams, DequeueMessagesParams, DequeueMessagesResponse,
            EncryptionKeyResponse, KeyPackageParams, KeyPackageResponseIn, QsProcessResponseIn,
            QsVersionedProcessResponseIn, UpdateClientRecordParams, UpdateUserRecordParams,
            VersionError,
        },
        client_qs_out::{
            ClientToQsMessageOut, ClientToQsMessageTbsOut, CreateClientRecordParamsOut,
            CreateUserRecordParamsOut, PublishKeyPackagesParamsOut, QsRequestParamsOut,
        },
        push_token::EncryptedPushToken,
        ApiVersion, FriendshipToken,
    },
};
use thiserror::Error;
use tls_codec::{DeserializeBytes, Serialize};

use crate::{
    version::{extract_api_version_negotiation, negotiate_api_version},
    ApiClient, Protocol,
};

pub mod ws;

mod api_migrations;
#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum QsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Couldn't deserialize TLS response body: {0}")]
    Tls(#[from] tls_codec::Error),
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("Unsuccessful response: status = {status}, error = {error}")]
    RequestFailed { status: StatusCode, error: String },
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error(transparent)]
    Version(#[from] VersionError),
}

// TODO: This is a workaround that allows us to use the Signable trait.
enum AuthenticationMethod<'a, T: SigningKeyBehaviour> {
    Token(FriendshipToken),
    SigningKey(&'a T),
    None,
}

impl ApiClient {
    async fn prepare_and_send_qs_message<T: SigningKeyBehaviour>(
        &self,
        request_params: QsRequestParamsOut,
        token_or_signing_key: AuthenticationMethod<'_, T>,
    ) -> Result<QsProcessResponseIn, QsRequestError> {
        let api_version = self.negotiated_versions().qs_api_version();

        let message = sign_params(request_params, &token_or_signing_key, api_version)?;
        let endpoint = self.build_url(Protocol::Http, ENDPOINT_QS);
        let response = send_qs_message(&self.client, &endpoint, &message).await?;

        // check if we need to negotiate a new API version
        let Some(accepted_versions) = extract_api_version_negotiation(&response) else {
            return process_response(response).await;
        };

        let supported_versions = ClientToQsMessageTbs::SUPPORTED_API_VERSIONS;
        let accepted_version = negotiate_api_version(accepted_versions, supported_versions)
            .ok_or_else(|| VersionError::new(api_version, supported_versions.to_vec()))?;
        self.negotiated_versions()
            .set_qs_api_version(accepted_version);

        let request_params = message.into_payload().into_unversioned_params();
        let message = sign_params(request_params, &token_or_signing_key, accepted_version)?;

        let response = send_qs_message(&self.client, &endpoint, &message).await?;
        process_response(response).await
    }

    pub async fn qs_create_user(
        &self,
        friendship_token: FriendshipToken,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        initial_ratchet_key: RatchetSecret,
        signing_key: &QsUserSigningKey,
    ) -> Result<CreateUserRecordResponse, QsRequestError> {
        let payload = CreateUserRecordParamsOut {
            user_record_auth_key: signing_key.verifying_key().clone(),
            friendship_token,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
            initial_ratchet_secret: initial_ratchet_key,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::CreateUser(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::CreateUser(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_update_user(
        &self,
        sender: QsUserId,
        friendship_token: FriendshipToken,
        signing_key: &QsUserSigningKey,
    ) -> Result<(), QsRequestError> {
        let payload = UpdateUserRecordParams {
            user_record_auth_key: signing_key.verifying_key().clone(),
            friendship_token,
            sender,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::UpdateUser(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, QsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_delete_user(
        &self,
        sender: QsUserId,
        signing_key: &QsUserSigningKey,
    ) -> Result<(), QsRequestError> {
        let payload = DeleteUserRecordParams { sender };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::DeleteUser(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, QsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_create_client(
        &self,
        sender: QsUserId,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        initial_ratchet_key: RatchetSecret,
        signing_key: &QsUserSigningKey,
    ) -> Result<CreateClientRecordResponse, QsRequestError> {
        let payload = CreateClientRecordParamsOut {
            sender,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
            initial_ratchet_secret: initial_ratchet_key,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::CreateClient(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::CreateClient(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_update_client(
        &self,
        sender: QsClientId,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let payload = UpdateClientRecordParams {
            sender,
            client_record_auth_key: signing_key.verifying_key().clone(),
            queue_encryption_key,
            encrypted_push_token,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::UpdateClient(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, QsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_delete_client(
        &self,
        sender: QsClientId,
        signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let payload = DeleteClientRecordParams { sender };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::DeleteClient(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, QsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_publish_key_packages(
        &self,
        sender: QsClientId,
        key_packages: Vec<KeyPackage>,
        friendship_ear_key: KeyPackageEarKey,
        signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let payload = PublishKeyPackagesParamsOut {
            sender,
            key_packages,
            friendship_ear_key,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::PublishKeyPackages(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, QsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_client_key_package(
        &self,
        sender: QsUserId,
        client_id: QsClientId,
        signing_key: &QsUserSigningKey,
    ) -> Result<ClientKeyPackageResponse, QsRequestError> {
        let payload = ClientKeyPackageParams { sender, client_id };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::ClientKeyPackage(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::ClientKeyPackage(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_dequeue_messages(
        &self,
        sender: &QsClientId,
        sequence_number_start: u64,
        max_message_number: u64,
        signing_key: &QsClientSigningKey,
    ) -> Result<DequeueMessagesResponse, QsRequestError> {
        let payload = DequeueMessagesParams {
            sender: *sender,
            sequence_number_start,
            max_message_number,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::DequeueMessages(payload),
            AuthenticationMethod::SigningKey(signing_key),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::DequeueMessages(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_key_package(
        &self,
        sender: FriendshipToken,
        friendship_ear_key: KeyPackageEarKey,
    ) -> Result<KeyPackageResponseIn, QsRequestError> {
        let payload = KeyPackageParams {
            sender: sender.clone(),
            friendship_ear_key,
        };
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::KeyPackage(payload),
            AuthenticationMethod::<QsUserSigningKey>::Token(sender),
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::KeyPackage(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }

    pub async fn qs_encryption_key(&self) -> Result<EncryptionKeyResponse, QsRequestError> {
        self.prepare_and_send_qs_message(
            QsRequestParamsOut::QsEncryptionKey,
            AuthenticationMethod::<QsUserSigningKey>::None,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let QsProcessResponseIn::EncryptionKey(resp) = response {
                Ok(resp)
            } else {
                Err(QsRequestError::UnexpectedResponse)
            }
        })
    }
}

async fn process_response(
    response: reqwest::Response,
) -> Result<QsProcessResponseIn, QsRequestError> {
    let status = response.status();
    if status.is_success() {
        let bytes = response.bytes().await.map_err(QsRequestError::Reqwest)?;
        let qs_response = QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&bytes)
            .map_err(QsRequestError::Tls)?;
        migrate_qs_process_response(qs_response)
    } else {
        let error = response
            .text()
            .await
            .unwrap_or_else(|error| format!("unprocessable response body due to: {error}"));
        Err(QsRequestError::RequestFailed { status, error })
    }
}

async fn send_qs_message(
    client: &reqwest::Client,
    endpoint: &str,
    message: &ClientToQsMessageOut,
) -> Result<reqwest::Response, QsRequestError> {
    client
        .post(endpoint)
        .body(
            message
                .tls_serialize_detached()
                .map_err(|_| QsRequestError::LibraryError)?,
        )
        .send()
        .await
        .map_err(From::from)
}

fn sign_params<T: SigningKeyBehaviour>(
    request_params: QsRequestParamsOut,
    token_or_signing_key: &AuthenticationMethod<'_, T>,
    api_version: ApiVersion,
) -> Result<ClientToQsMessageOut, QsRequestError> {
    let tbs = ClientToQsMessageTbsOut::with_api_version(api_version, request_params).ok_or_else(
        || {
            VersionError::new(
                api_version,
                ClientToQsMessageTbs::SUPPORTED_API_VERSIONS.to_vec(),
            )
        },
    )?;
    let message = match token_or_signing_key {
        AuthenticationMethod::Token(token) => ClientToQsMessageOut::from_token(tbs, token.clone()),
        AuthenticationMethod::SigningKey(signing_key) => tbs
            .sign(*signing_key)
            .map_err(|_| QsRequestError::LibraryError)?,
        AuthenticationMethod::None => ClientToQsMessageOut::without_signature(tbs),
    };
    Ok(message)
}
