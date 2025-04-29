// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::pin::Pin;

use phnxprotos::{
    queue_service::v1::{queue_service_server::QueueService, *},
    validation::MissingFieldExt,
};
use phnxtypes::identifiers;
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio_stream::{Stream, StreamExt, wrappers::UnboundedReceiverStream};
use tonic::{Request, Response, Status, async_trait};

use super::Qs;

pub struct GrpcQs<L: GrpcListen> {
    _qs: Qs,
    listen: L,
}

impl<L: GrpcListen> GrpcQs<L> {
    pub fn new(qs: Qs, listen: L) -> Self {
        Self { _qs: qs, listen }
    }
}

pub trait GrpcListen: Send + Sync + 'static {
    fn register_connection(
        &self,
        queue_id: identifiers::QsClientId,
        tx: mpsc::UnboundedSender<QueueEvent>,
    ) -> impl Future<Output = ()> + Send + Sync;
}

#[async_trait]
impl<L: GrpcListen> QueueService for GrpcQs<L> {
    async fn create_user(
        &self,
        _request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        todo!()
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
