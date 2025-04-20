// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::auth_service::v1::{auth_service_server, *};
use tonic::{Request, Response, Status, async_trait};

use super::AuthService;

pub struct GrpcAs {
    _auth_service: AuthService,
}

impl GrpcAs {
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            _auth_service: auth_service,
        }
    }
}

#[async_trait]
impl auth_service_server::AuthService for GrpcAs {
    async fn init2_fa_authentication(
        &self,
        _request: Request<Init2FaAuthenticationRequest>,
    ) -> Result<Response<Init2FaAuthenticationResponse>, Status> {
        todo!()
    }

    async fn init_user_registration(
        &self,
        _request: Request<InitUserRegistrationRequest>,
    ) -> Result<Response<InitUserRegistrationResponse>, Status> {
        todo!()
    }

    async fn finish_user_registration(
        &self,
        _request: Request<FinishUserRegistrationRequest>,
    ) -> Result<Response<FinishUserRegistrationResponse>, Status> {
        todo!()
    }

    async fn delete_user(
        &self,
        _request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        todo!()
    }

    async fn init_client_addition(
        &self,
        _request: Request<InitClientAdditionRequest>,
    ) -> Result<Response<InitClientAdditionResponse>, Status> {
        todo!()
    }

    async fn finish_client_addition(
        &self,
        _request: Request<FinishClientAdditionRequest>,
    ) -> Result<Response<FinishClientAdditionResponse>, Status> {
        todo!()
    }

    async fn publish_connection_package(
        &self,
        _request: Request<PublishConnectionPackageRequest>,
    ) -> Result<Response<PublishConnectionPackageResponse>, Status> {
        todo!()
    }

    async fn client_connection_package(
        &self,
        _request: Request<ClientConnectionPackageRequest>,
    ) -> Result<Response<ClientConnectionPackageResponse>, Status> {
        todo!()
    }

    async fn user_connection_packages(
        &self,
        _request: Request<UserConnectionPackagesRequest>,
    ) -> Result<Response<UserConnectionPackagesResponse>, Status> {
        todo!()
    }

    async fn user_clients(
        &self,
        _request: Request<UserClientsRequest>,
    ) -> Result<Response<UserClientsResponse>, Status> {
        todo!()
    }

    async fn as_credentials(
        &self,
        _request: Request<AsCredentialsRequest>,
    ) -> Result<Response<AsCredentialsResponse>, Status> {
        todo!()
    }

    async fn issue_tokens(
        &self,
        _request: Request<IssueTokensRequest>,
    ) -> Result<Response<IssueTokensResponse>, Status> {
        todo!()
    }

    async fn enqueue_messages(
        &self,
        _request: Request<EnqueueMessagesRequest>,
    ) -> Result<Response<EnqueueMessagesResponse>, Status> {
        todo!()
    }

    async fn dequeue_messages(
        &self,
        _request: Request<DequeueMessagesRequest>,
    ) -> Result<Response<DequeueMessagesResponse>, Status> {
        todo!()
    }
}
