// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    errors::qs::QsProcessError,
    identifiers::{QsClientId, QsUserId},
    messages::{
        client_qs::{
            ClientKeyPackageParams, ClientKeyPackageResponse, CreateClientRecordResponse,
            CreateUserRecordResponse, DeleteClientRecordParams, DeleteUserRecordParams,
            DequeueMessagesParams, DequeueMessagesResponse, EncryptionKeyResponse,
            KeyPackageParams, KeyPackageResponseIn, QsProcessResponseIn, UpdateClientRecordParams,
            UpdateUserRecordParams,
        },
        client_qs_out::{
            ClientToQsMessageOut, ClientToQsMessageTbsOut, CreateClientRecordParamsOut,
            CreateUserRecordParamsOut, PublishKeyPackagesParamsOut, QsRequestParamsOut,
        },
        push_token::EncryptedPushToken,
        FriendshipToken,
    },
};
use thiserror::Error;
use tls_codec::{DeserializeBytes, Serialize};

use crate::{ApiClient, Protocol};

pub mod ws;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum QsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Couldn't deserialize response body.")]
    BadResponse,
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error(transparent)]
    QsError(#[from] QsProcessError),
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
        let tbs = ClientToQsMessageTbsOut::new(request_params);
        let message = match token_or_signing_key {
            AuthenticationMethod::Token(token) => ClientToQsMessageOut::from_token(tbs, token),
            AuthenticationMethod::SigningKey(signing_key) => tbs
                .sign(signing_key)
                .map_err(|_| QsRequestError::LibraryError)?,
            AuthenticationMethod::None => ClientToQsMessageOut::without_signature(tbs),
        };
        let message_bytes = message
            .tls_serialize_detached()
            .map_err(|_| QsRequestError::LibraryError)?;
        match self
            .client
            .post(self.build_url(Protocol::Http, ENDPOINT_QS))
            .body(message_bytes)
            .send()
            .await
        {
            Ok(res) => {
                match res.status().as_u16() {
                    // Success!
                    x if (200..=299).contains(&x) => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| QsRequestError::BadResponse)?;
                        let ds_proc_res =
                            QsProcessResponseIn::tls_deserialize_exact_bytes(&ds_proc_res_bytes)
                                .map_err(|_| QsRequestError::BadResponse)?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes =
                            res.bytes().await.map_err(|_| QsRequestError::BadResponse)?;
                        let ds_proc_err =
                            QsProcessError::tls_deserialize_exact_bytes(&ds_proc_err_bytes)
                                .map_err(|_| QsRequestError::BadResponse)?;
                        Err(QsRequestError::QsError(ds_proc_err))
                    }
                    // All other errors
                    _ => {
                        let error_text =
                            res.text().await.map_err(|_| QsRequestError::BadResponse)?;
                        Err(QsRequestError::NetworkError(error_text))
                    }
                }
            }
            // A network error occurred.
            Err(err) => Err(QsRequestError::NetworkError(err.to_string())),
        }
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
            sender: sender.clone(),
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
