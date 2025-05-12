// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackage;
use phnxprotos::queue_service::v1::QueueEvent;
use phnxprotos::{
    queue_service::v1::{
        CreateClientRequest, CreateUserRequest, DeleteClientRequest, DeleteUserRequest,
        DequeueMessagesRequest, KeyPackageRequest, ListenRequest, PublishKeyPackagesRequest,
        QsEncryptionKeyRequest, UpdateClientRequest, UpdateUserRequest,
    },
    validation::{MissingFieldError, MissingFieldExt},
};
use phnxtypes::{
    crypto::{
        RatchetEncryptionKey,
        kdf::keys::RatchetSecret,
        signatures::keys::{QsClientSigningKey, QsClientVerifyingKey, QsUserSigningKey},
    },
    identifiers::{QsClientId, QsUserId},
    messages::{
        FriendshipToken, QueueMessage,
        client_qs::{
            CreateClientRecordResponse, CreateUserRecordResponse, DequeueMessagesResponse,
            EncryptionKeyResponse, KeyPackageResponseIn,
        },
        push_token::EncryptedPushToken,
    },
};
use thiserror::Error;
use tokio_stream::{Stream, StreamExt};
use tracing::error;

use crate::ApiClient;

pub mod grpc;

#[derive(Error, Debug)]
pub enum QsRequestError {
    #[error(transparent)]
    Tls(#[from] tls_codec::Error),
    #[error("received an unexpected response")]
    UnexpectedResponse,
    #[error(transparent)]
    Tonic(#[from] tonic::Status),
    #[error("missing field in response: {0}")]
    MissingField(#[from] MissingFieldError<&'static str>),
}

impl ApiClient {
    pub async fn qs_create_user(
        &self,
        friendship_token: FriendshipToken,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        initial_ratchet_key: RatchetSecret,
        signing_key: &QsUserSigningKey,
    ) -> Result<CreateUserRecordResponse, QsRequestError> {
        let request = CreateUserRequest {
            user_record_auth_key: Some(signing_key.verifying_key().clone().into()),
            friendship_token: Some(friendship_token.into()),
            client_record_auth_key: Some(client_record_auth_key.into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            encrypted_push_token: encrypted_push_token.map(From::from),
            initial_ratched_secret: Some(initial_ratchet_key.into()),
        };
        let response = self
            .qs_grpc_client
            .client()
            .create_user(request)
            .await?
            .into_inner();
        Ok(CreateUserRecordResponse {
            user_id: response
                .user_id
                .ok_or_missing_field("user_id")?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid user_id in response");
                    QsRequestError::UnexpectedResponse
                })?,
            client_id: response
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid client_id in response");
                    QsRequestError::UnexpectedResponse
                })?,
        })
    }

    pub async fn qs_update_user(
        &self,
        sender: QsUserId,
        friendship_token: FriendshipToken,
        signing_key: &QsUserSigningKey,
    ) -> Result<(), QsRequestError> {
        let request = UpdateUserRequest {
            sender: Some(sender.into()),
            user_record_auth_key: Some(signing_key.verifying_key().clone().into()),
            friendship_token: Some(friendship_token.into()),
        };
        self.qs_grpc_client.client().update_user(request).await?;
        Ok(())
    }

    pub async fn qs_delete_user(
        &self,
        sender: QsUserId,
        _signing_key: &QsUserSigningKey,
    ) -> Result<(), QsRequestError> {
        let request = DeleteUserRequest {
            sender: Some(sender.into()),
        };
        self.qs_grpc_client.client().delete_user(request).await?;
        Ok(())
    }

    pub async fn qs_create_client(
        &self,
        sender: QsUserId,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        initial_ratchet_key: RatchetSecret,
        _signing_key: &QsUserSigningKey,
    ) -> Result<CreateClientRecordResponse, QsRequestError> {
        let request = CreateClientRequest {
            sender: Some(sender.into()),
            client_record_auth_key: Some(client_record_auth_key.into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            encrypted_push_token: encrypted_push_token.map(|token| token.into()),
            initial_ratched_secret: Some(initial_ratchet_key.into()),
        };
        let response = self
            .qs_grpc_client
            .client()
            .create_client(request)
            .await?
            .into_inner();
        Ok(CreateClientRecordResponse {
            client_id: response
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid client_id in response");
                    QsRequestError::UnexpectedResponse
                })?,
        })
    }

    pub async fn qs_update_client(
        &self,
        sender: QsClientId,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let request = UpdateClientRequest {
            sender: Some(sender.into()),
            client_record_auth_key: Some(signing_key.verifying_key().clone().into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            encrypted_push_token: encrypted_push_token.map(|token| token.into()),
        };
        self.qs_grpc_client.client().update_client(request).await?;
        Ok(())
    }

    pub async fn qs_delete_client(
        &self,
        sender: QsClientId,
        _signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let request = DeleteClientRequest {
            sender: Some(sender.into()),
        };
        self.qs_grpc_client.client().delete_client(request).await?;
        Ok(())
    }

    pub async fn qs_publish_key_packages(
        &self,
        sender: QsClientId,
        key_packages: Vec<KeyPackage>,
        _signing_key: &QsClientSigningKey,
    ) -> Result<(), QsRequestError> {
        let request = PublishKeyPackagesRequest {
            client_id: Some(sender.into()),
            key_packages: key_packages
                .into_iter()
                .map(|key_package| key_package.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        };
        self.qs_grpc_client
            .client()
            .publish_key_packages(request)
            .await?;
        Ok(())
    }

    pub async fn qs_dequeue_messages(
        &self,
        sender: &QsClientId,
        sequence_number_start: u64,
        max_message_number: u64,
        _signing_key: &QsClientSigningKey,
    ) -> Result<DequeueMessagesResponse, QsRequestError> {
        let request = DequeueMessagesRequest {
            sender: Some((*sender).into()),
            sequence_number_start,
            max_message_number,
        };
        let response = self
            .qs_grpc_client
            .client()
            .dequeue_messages(request)
            .await?
            .into_inner();
        let messages: Result<Vec<QueueMessage>, _> = response
            .messages
            .into_iter()
            .map(|message| message.try_into())
            .collect();
        let messages = messages.map_err(|error| {
            error!(%error, "failed to dequeue messages");
            QsRequestError::UnexpectedResponse
        })?;
        Ok(DequeueMessagesResponse {
            messages,
            remaining_messages_number: response.remaining_messages_number,
        })
    }

    pub async fn qs_key_package(
        &self,
        sender: FriendshipToken,
    ) -> Result<KeyPackageResponseIn, QsRequestError> {
        let request = KeyPackageRequest {
            sender: Some(sender.into()),
        };
        let response = self
            .qs_grpc_client
            .client()
            .key_package(request)
            .await?
            .into_inner();
        let key_package = response
            .key_package
            .ok_or_missing_field("key_package")?
            .try_into()
            .map_err(|error| {
                error!(%error, "invalid key_package in response");
                QsRequestError::UnexpectedResponse
            })?;
        Ok(KeyPackageResponseIn { key_package })
    }

    pub async fn qs_encryption_key(&self) -> Result<EncryptionKeyResponse, QsRequestError> {
        let request = QsEncryptionKeyRequest {};
        let response = self
            .qs_grpc_client
            .client()
            .qs_encryption_key(request)
            .await?
            .into_inner();
        let encryption_key = response
            .encryption_key
            .ok_or_missing_field("encryption_key")?
            .into();
        Ok(EncryptionKeyResponse { encryption_key })
    }

    pub async fn listen_queue(
        &self,
        queue_id: QsClientId,
    ) -> Result<impl Stream<Item = QueueEvent> + use<>, QsRequestError> {
        let request = ListenRequest {
            client_id: Some(queue_id.into()),
        };
        let response = self.qs_grpc_client.client().listen(request).await?;
        let stream = response.into_inner().map_while(|response| {
            response
                .inspect_err(|status| error!(?status, "terminating listen stream due to an error"))
                .ok()
        });
        Ok(stream)
    }
}
