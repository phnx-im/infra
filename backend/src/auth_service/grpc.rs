// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{auth_service_server, *},
    convert::TryRefInto,
    validation::{InvalidTlsExt, MissingFieldExt},
};
use phnxtypes::{
    errors::auth_service::{AsVerificationError, Init2FactorAuthError},
    messages::client_as::Init2FactorAuthParamsTbs,
};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use super::AuthService;

pub struct GrpcAs {
    auth_service: AuthService,
}

impl GrpcAs {
    pub fn new(auth_service: AuthService) -> Self {
        Self { auth_service }
    }
}

#[async_trait]
impl auth_service_server::AuthService for GrpcAs {
    async fn init2_fa_authentication(
        &self,
        request: Request<Init2FaAuthenticationRequest>,
    ) -> Result<Response<Init2FaAuthenticationResponse>, Status> {
        let request = request.into_inner();

        let payload = self
            .auth_service
            .verify(request.into_auth_method()?)
            .await
            .map_err(AuthError)?;

        let params = Init2FactorAuthParamsTbs {
            client_id: payload
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()?,
            opaque_ke1: payload
                .opaque_ke1
                .ok_or_missing_field("opaque_ke1")?
                .try_ref_into()
                .invalid_tls("opaque_ke1")?,
        };
        let response = self
            .auth_service
            .as_init_two_factor_auth(params)
            .await
            .map_err(Init2FaAuthError)?;
        let opaque_ke2 = response
            .opaque_ke2
            .try_ref_into()
            .invalid_tls("opaque_ke2")?;
        Ok(Response::new(Init2FaAuthenticationResponse {
            opaque_ke2: Some(opaque_ke2),
        }))
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

struct AuthError(AsVerificationError);

impl From<AuthError> for Status {
    fn from(e: AuthError) -> Self {
        match e.0 {
            AsVerificationError::StorageError | AsVerificationError::Api(_) => {
                error!(error =% e.0, "failed to authenticate request");
                Status::internal("failed to authenticate request")
            }
            AsVerificationError::UnknownClient => Status::unauthenticated("unknown client"),
            AsVerificationError::UnknownUser => Status::unauthenticated("unknown user"),
            AsVerificationError::AuthenticationFailed => {
                Status::unauthenticated("authentication failed")
            }
        }
    }
}

struct Init2FaAuthError(Init2FactorAuthError);

impl From<Init2FaAuthError> for Status {
    fn from(e: Init2FaAuthError) -> Self {
        error!(error =% e.0, "init 2fa auth failed");
        Status::internal(e.0.to_string())
    }
}
