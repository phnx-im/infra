// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{auth_service_server, *},
    validation::MissingFieldExt,
};

use phnxtypes::{errors, messages::client_as_out::RegisterUserParamsIn};
use tonic::{Request, Response, Status, async_trait};

use super::AuthService;

pub struct GrpcAs {
    inner: AuthService,
}

impl GrpcAs {
    pub fn new(inner: AuthService) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl auth_service_server::AuthService for GrpcAs {
    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status> {
        let request = request.into_inner();
        let params = RegisterUserParamsIn {
            client_payload: request
                .client_credential_payload
                .ok_or_missing_field("client_payload")?
                .try_into()?,
            queue_encryption_key: request
                .queue_encryption_key
                .ok_or_missing_field("queue_encryption_key")?
                .into(),
            initial_ratchet_secret: request
                .initial_ratchet_secret
                .ok_or_missing_field("initial_ratchet_secret")?
                .try_into()?,
            encrypted_user_profile: request
                .encrypted_user_profile
                .ok_or_missing_field("encrypted_user_profile")?
                .try_into()?,
        };
        let response = self
            .inner
            .as_init_user_registration(params)
            .await
            .map_err(RegisterUserError)?;
        Ok(Response::new(RegisterUserResponse {
            client_credential: Some(response.client_credential.into()),
        }))
    }

    async fn delete_user(
        &self,
        _request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        todo!()
    }

    async fn publish_connection_package(
        &self,
        _request: Request<PublishConnectionPackageRequest>,
    ) -> Result<Response<PublishConnectionPackageResponse>, Status> {
        todo!()
    }

    async fn get_connection_package(
        &self,
        _request: Request<GetConnectionPackageRequest>,
    ) -> Result<Response<GetConnectionPackageResponse>, Status> {
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

struct RegisterUserError(errors::auth_service::RegisterUserError);

impl From<RegisterUserError> for Status {
    fn from(e: RegisterUserError) -> Self {
        match e.0 {
            errors::auth_service::RegisterUserError::LibraryError
            | errors::auth_service::RegisterUserError::StorageError => {
                Status::internal(e.0.to_string())
            }
            errors::auth_service::RegisterUserError::SigningKeyNotFound => {
                Status::not_found(e.0.to_string())
            }
            errors::auth_service::RegisterUserError::UserAlreadyExists => {
                Status::already_exists(e.0.to_string())
            }
            errors::auth_service::RegisterUserError::InvalidCsr(..) => {
                Status::invalid_argument(e.0.to_string())
            }
        }
    }
}
