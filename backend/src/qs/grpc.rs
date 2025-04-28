// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::pin::Pin;

use phnxprotos::{
    queue_service::v1::{queue_service_server::QueueService, *},
    validation::MissingFieldExt,
};
use phnxtypes::{identifiers, messages::client_qs::CreateUserRecordParams};
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
            client_id: Some(response.client_id.into()),
        };
        Ok(Response::new(response))
    }

    async fn update_user(
        &self,
        _request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        todo!()
    }

    async fn delete_user(
        &self,
        _request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        todo!()
    }

    async fn create_client(
        &self,
        _request: Request<CreateClientRequest>,
    ) -> Result<Response<CreateClientResponse>, Status> {
        todo!()
    }

    async fn update_client(
        &self,
        _request: Request<UpdateClientRequest>,
    ) -> Result<Response<UpdateClientResponse>, Status> {
        todo!()
    }

    async fn delete_client(
        &self,
        _request: Request<DeleteClientRequest>,
    ) -> Result<Response<DeleteClientResponse>, Status> {
        todo!()
    }

    async fn publish_key_packages(
        &self,
        _request: Request<PublishKeyPackagesRequest>,
    ) -> Result<Response<PublishKeyPackagesResponse>, Status> {
        todo!()
    }

    async fn client_key_packages(
        &self,
        _request: Request<ClientKeyPackagesRequest>,
    ) -> Result<Response<ClientKeyPackagesResponse>, Status> {
        todo!()
    }

    async fn key_package(
        &self,
        _request: Request<KeyPackageRequest>,
    ) -> Result<Response<KeyPackageResponse>, Status> {
        todo!()
    }

    async fn dequeue_messages(
        &self,
        _request: Request<DequeueMessagesRequest>,
    ) -> Result<Response<DequeueMessagesResponse>, Status> {
        todo!()
    }

    async fn qs_encryption_key(
        &self,
        _request: Request<QsEncryptionKeyRequest>,
    ) -> Result<Response<QsEncryptionKeyResponse>, Status> {
        todo!()
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
