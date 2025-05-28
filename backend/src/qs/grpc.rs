// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::pin::Pin;

use phnxprotos::{
    queue_service::v1::{queue_service_server::QueueService, *},
    validation::{InvalidTlsExt, MissingFieldExt},
};

use phnxcommon::{
    identifiers,
    messages::client_qs::{
        CreateClientRecordParams, CreateUserRecordParams, DeleteClientRecordParams,
        DeleteUserRecordParams, DequeueMessagesParams, KeyPackageParams, PublishKeyPackagesParams,
        UpdateClientRecordParams, UpdateUserRecordParams,
    },
};
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio_stream::{Stream, StreamExt, wrappers::UnboundedReceiverStream};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use super::Qs;

pub struct GrpcQs<L: GrpcListen> {
    qs: Qs,
    listen: L,
}

impl<L: GrpcListen> GrpcQs<L> {
    pub fn new(qs: Qs, listen: L) -> Self {
        Self { qs, listen }
    }
}

pub trait GrpcListen: Send + Sync + 'static {
    fn register_connection(
        &self,
        queue_id: identifiers::QsClientId,
        tx: mpsc::UnboundedSender<QueueEvent>,
    ) -> impl Future<Output = ()> + Send + Sync;
}

// Note: currently, *no* authentication is done
#[async_trait]
impl<L: GrpcListen> QueueService for GrpcQs<L> {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let request = request.into_inner();
        let params = CreateUserRecordParams {
            user_record_auth_key: request
                .user_record_auth_key
                .ok_or_missing_field("user_record_auth_key")?
                .into(),
            friendship_token: request
                .friendship_token
                .ok_or_missing_field("friendship_token")?
                .into(),
            client_record_auth_key: request
                .client_record_auth_key
                .ok_or_missing_field("client_record_auth_key")?
                .into(),
            queue_encryption_key: request
                .queue_encryption_key
                .ok_or_missing_field("queue_encryption_key")?
                .into(),
            encrypted_push_token: request
                .encrypted_push_token
                .map(|token| token.try_into())
                .transpose()?,
            initial_ratchet_secret: request
                .initial_ratched_secret
                .ok_or_missing_field("initial_ratched_secret")?
                .try_into()?,
        };
        let response = self
            .qs
            .qs_create_user_record(params)
            .await
            .map_err(|error| {
                error!(%error, "failed to create user record");
                Status::internal("failed to create user record")
            })?;
        let response = CreateUserResponse {
            user_id: Some(response.user_id.into()),
            client_id: Some(response.qs_client_id.into()),
        };
        Ok(Response::new(response))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        let request = request.into_inner();
        let params = UpdateUserRecordParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
            user_record_auth_key: request
                .user_record_auth_key
                .ok_or_missing_field("user_record_auth_key")?
                .into(),
            friendship_token: request
                .friendship_token
                .ok_or_missing_field("friendship_token")?
                .into(),
        };
        self.qs
            .qs_update_user_record(params)
            .await
            .map_err(|error| {
                error!(%error, "failed to update user record");
                Status::internal("failed to update user record")
            })?;
        Ok(Response::new(UpdateUserResponse {}))
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        let request = request.into_inner();
        let params = DeleteUserRecordParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
        };
        self.qs
            .qs_delete_user_record(params)
            .await
            .map_err(|error| {
                error!(%error, "failed to delete user record");
                Status::internal("failed to delete user record")
            })?;
        Ok(Response::new(DeleteUserResponse {}))
    }

    async fn create_client(
        &self,
        request: Request<CreateClientRequest>,
    ) -> Result<Response<CreateClientResponse>, Status> {
        let request = request.into_inner();
        let params = CreateClientRecordParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
            client_record_auth_key: request
                .client_record_auth_key
                .ok_or_missing_field("client_record_auth_key")?
                .into(),
            queue_encryption_key: request
                .queue_encryption_key
                .ok_or_missing_field("queue_encryption_key")?
                .into(),
            encrypted_push_token: request
                .encrypted_push_token
                .map(|token| token.try_into())
                .transpose()?,
            initial_ratchet_secret: request
                .initial_ratched_secret
                .ok_or_missing_field("initial_ratched_secret")?
                .try_into()?,
        };
        let response = self.qs.qs_create_client_record(params).await?;
        Ok(Response::new(CreateClientResponse {
            client_id: Some(response.qs_client_id.into()),
        }))
    }

    async fn update_client(
        &self,
        request: Request<UpdateClientRequest>,
    ) -> Result<Response<UpdateClientResponse>, Status> {
        let request = request.into_inner();
        let params = UpdateClientRecordParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
            client_record_auth_key: request
                .client_record_auth_key
                .ok_or_missing_field("client_record_auth_key")?
                .into(),
            queue_encryption_key: request
                .queue_encryption_key
                .ok_or_missing_field("queue_encryption_key")?
                .into(),
            encrypted_push_token: request
                .encrypted_push_token
                .map(|token| token.try_into())
                .transpose()?,
        };
        self.qs.qs_update_client_record(params).await?;
        Ok(Response::new(UpdateClientResponse {}))
    }

    async fn delete_client(
        &self,
        request: Request<DeleteClientRequest>,
    ) -> Result<Response<DeleteClientResponse>, Status> {
        let request = request.into_inner();
        let params = DeleteClientRecordParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
        };
        self.qs.qs_delete_client_record(params).await?;
        Ok(Response::new(DeleteClientResponse {}))
    }

    async fn publish_key_packages(
        &self,
        request: Request<PublishKeyPackagesRequest>,
    ) -> Result<Response<PublishKeyPackagesResponse>, Status> {
        let request = request.into_inner();
        let params = PublishKeyPackagesParams {
            sender: request
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()?,
            key_packages: request
                .key_packages
                .into_iter()
                .map(|key_package| key_package.try_into())
                .collect::<Result<Vec<_>, _>>()
                .invalid_tls("key_packages")?,
        };
        self.qs.qs_publish_key_packages(params).await?;
        Ok(Response::new(PublishKeyPackagesResponse {}))
    }

    async fn key_package(
        &self,
        request: Request<KeyPackageRequest>,
    ) -> Result<Response<KeyPackageResponse>, Status> {
        let request = request.into_inner();
        let params = KeyPackageParams {
            sender: request.sender.ok_or_missing_field("sender")?.into(),
        };
        let response = self.qs.qs_key_package(params).await?;
        Ok(Response::new(KeyPackageResponse {
            key_package: Some(response.key_package.try_into().tls_failed("key_package")?),
        }))
    }

    async fn dequeue_messages(
        &self,
        request: Request<DequeueMessagesRequest>,
    ) -> Result<Response<DequeueMessagesResponse>, Status> {
        let request = request.into_inner();
        let params = DequeueMessagesParams {
            sender: request.sender.ok_or_missing_field("sender")?.try_into()?,
            sequence_number_start: request.sequence_number_start,
            max_message_number: request.max_message_number,
        };
        let response = self.qs.qs_dequeue_messages(params).await?;
        Ok(Response::new(DequeueMessagesResponse {
            messages: response
                .messages
                .into_iter()
                .map(|message| message.into())
                .collect(),
            remaining_messages_number: response.remaining_messages_number,
        }))
    }

    async fn qs_encryption_key(
        &self,
        _request: Request<QsEncryptionKeyRequest>,
    ) -> Result<Response<QsEncryptionKeyResponse>, Status> {
        let response = self.qs.qs_encryption_key().await?;
        Ok(Response::new(QsEncryptionKeyResponse {
            encryption_key: Some(response.encryption_key.into()),
        }))
    }

    type ListenStream = Pin<Box<dyn Stream<Item = Result<QueueEvent, Status>> + Send + 'static>>;

    async fn listen(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        let request = request.into_inner();
        let client_id = request
            .client_id
            .ok_or_missing_field("client_id")?
            .try_into()?;

        let (tx, rx) = unbounded_channel();

        self.listen.register_connection(client_id, tx).await;

        Ok(Response::new(Box::pin(
            UnboundedReceiverStream::new(rx).map(Ok),
        )))
    }
}
