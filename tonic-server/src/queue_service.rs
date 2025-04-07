#![allow(unused_variables)]

use protos::queue_service::v1::*;
use tonic::{Request, Response, Status, async_trait};

#[derive(Clone)]
pub(crate) struct QueueService {}

impl QueueService {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl protos::queue_service::v1::queue_service_server::QueueService for QueueService {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        todo!()
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        todo!()
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        todo!()
    }

    async fn create_client(
        &self,
        request: Request<CreateClientRequest>,
    ) -> Result<Response<CreateClientResponse>, Status> {
        todo!()
    }

    async fn update_client(
        &self,
        request: Request<UpdateClientRequest>,
    ) -> Result<Response<UpdateClientResponse>, Status> {
        todo!()
    }

    async fn delete_client(
        &self,
        request: Request<DeleteClientRequest>,
    ) -> Result<Response<DeleteClientResponse>, Status> {
        todo!()
    }

    async fn publish_key_packages(
        &self,
        request: Request<PublishKeyPackagesRequest>,
    ) -> Result<Response<PublishKeyPackagesResponse>, Status> {
        todo!()
    }

    async fn client_key_packages(
        &self,
        request: Request<ClientKeyPackagesRequest>,
    ) -> Result<Response<ClientKeyPackagesResponse>, Status> {
        todo!()
    }

    async fn key_package(
        &self,
        request: Request<KeyPackageRequest>,
    ) -> Result<Response<KeyPackageResponse>, Status> {
        todo!()
    }

    async fn dequeue_messages(
        &self,
        request: Request<DequeueMessagesRequest>,
    ) -> Result<Response<DequeueMessagesResponse>, Status> {
        todo!()
    }

    async fn qs_encryption_key(
        &self,
        request: Request<QsEncryptionKeyRequest>,
    ) -> Result<Response<QsEncryptionKeyResponse>, Status> {
        todo!()
    }
}
