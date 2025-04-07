#![allow(unused_variables)]

use protos::auth_service::v1::*;
use tonic::{Status, async_trait};

#[derive(Clone)]
pub(crate) struct AuthService {}

impl AuthService {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl protos::auth_service::v1::auth_service_server::AuthService for AuthService {
    async fn init2_fa_authentication(
        &self,
        request: tonic::Request<Init2FaAuthenticationRequest>,
    ) -> Result<tonic::Response<Init2FaAuthenticationResponse>, Status> {
        todo!()
    }
    async fn init_user_registration(
        &self,
        request: tonic::Request<InitUserRegistrationRequest>,
    ) -> Result<tonic::Response<InitUserRegistrationResponse>, Status> {
        todo!()
    }
    async fn finish_user_registration(
        &self,
        request: tonic::Request<FinishUserRegistrationRequest>,
    ) -> Result<tonic::Response<FinishUserRegistrationResponse>, Status> {
        todo!()
    }
    async fn delete_user(
        &self,
        request: tonic::Request<DeleteUserRequest>,
    ) -> Result<tonic::Response<DeleteUserResponse>, Status> {
        todo!()
    }
    async fn init_client_addition(
        &self,
        request: tonic::Request<InitClientAdditionRequest>,
    ) -> Result<tonic::Response<InitClientAdditionResponse>, Status> {
        todo!()
    }
    async fn finish_client_addition(
        &self,
        request: tonic::Request<FinishClientAdditionRequest>,
    ) -> Result<tonic::Response<FinishClientAdditionResponse>, Status> {
        todo!()
    }
    async fn publish_connection_package(
        &self,
        request: tonic::Request<PublishConnectionPackageRequest>,
    ) -> Result<tonic::Response<PublishConnectionPackageResponse>, Status> {
        todo!()
    }
    async fn client_connection_package(
        &self,
        request: tonic::Request<ClientConnectionPackageRequest>,
    ) -> Result<tonic::Response<ClientConnectionPackageResponse>, Status> {
        todo!()
    }
    async fn user_connection_packages(
        &self,
        request: tonic::Request<UserConnectionPackagesRequest>,
    ) -> Result<tonic::Response<UserConnectionPackagesResponse>, Status> {
        todo!()
    }
    async fn user_clients(
        &self,
        request: tonic::Request<UserClientsRequest>,
    ) -> Result<tonic::Response<UserClientsResponse>, Status> {
        todo!()
    }
    async fn as_credentials(
        &self,
        request: tonic::Request<AsCredentialsRequest>,
    ) -> Result<tonic::Response<AsCredentialsResponse>, Status> {
        todo!()
    }
    async fn issue_tokens(
        &self,
        request: tonic::Request<IssueTokensRequest>,
    ) -> Result<tonic::Response<IssueTokensResponse>, Status> {
        todo!()
    }
    async fn enqueue_messages(
        &self,
        request: tonic::Request<EnqueueMessagesRequest>,
    ) -> Result<tonic::Response<EnqueueMessagesResponse>, Status> {
        todo!()
    }
    async fn dequeue_messages(
        &self,
        request: tonic::Request<DequeueMessagesRequest>,
    ) -> Result<tonic::Response<DequeueMessagesResponse>, Status> {
        todo!()
    }
}
