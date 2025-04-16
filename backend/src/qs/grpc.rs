// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::pin::Pin;

use phnxprotos::queue_service::v1::{queue_service_server::QueueService, *};
use tokio_stream::Stream;
use tonic::{Request, Response, Status, async_trait};

use super::Qs;

#[derive(Debug, Clone)]
pub struct GrpcQs {
    qs: Qs,
}

impl GrpcQs {
    pub fn new(qs: Qs) -> Self {
        Self { qs }
    }
}

#[async_trait]
impl QueueService for GrpcQs {
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

    type ListenStream =
        Pin<Box<dyn Stream<Item = Result<ListenResponse, Status>> + Send + 'static>>;

    async fn listen(
        &self,
        _request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        todo!()
    }
}
