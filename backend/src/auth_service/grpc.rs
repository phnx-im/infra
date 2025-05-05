// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{auth_service_server, *},
    validation::MissingFieldExt,
};

use phnxtypes::{
    crypto::signatures::{
        private_keys::SignatureVerificationError,
        signable::{Verifiable, VerifiedStruct},
    },
    errors, identifiers,
    messages::{
        client_as::{AsCredentialsParams, ClientConnectionPackageParamsTbs, DeleteUserParamsTbs},
        client_as_out::{AsPublishConnectionPackagesParamsTbsIn, RegisterUserParamsIn},
    },
};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use super::{AuthService, client_record::ClientRecord};

pub struct GrpcAs {
    inner: AuthService,
}

impl GrpcAs {
    pub fn new(inner: AuthService) -> Self {
        Self { inner }
    }

    async fn verify_client_auth<R, P>(
        &self,
        request: R,
    ) -> Result<(identifiers::AsClientId, P), Status>
    where
        R: WithAsClientId + Verifiable,
        P: VerifiedStruct<R>,
    {
        let client_id = request.client_id()?;
        let client_record = ClientRecord::load(&self.inner.db_pool, &client_id)
            .await
            .map_err(|error| {
                error!(%error, %client_id, "failed to load client");
                Status::internal("database error")
            })?
            .ok_or_else(|| Status::not_found("unknown client"))?;
        let payload = request
            .verify(client_record.credential.verifying_key())
            .map_err(|error| match error {
                SignatureVerificationError::VerificationFailure => {
                    Status::unauthenticated("invalid signature")
                }
                SignatureVerificationError::LibraryError(_) => {
                    Status::internal("unrecoverable error")
                }
            })?;
        Ok((client_id, payload))
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
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        let request = request.into_inner();
        let (client_id, payload) = self
            .verify_client_auth::<_, DeleteUserPayload>(request)
            .await?;
        let params = DeleteUserParamsTbs {
            user_name: payload
                .user_name
                .ok_or_missing_field("user_name")?
                .try_into()?,
            client_id,
        };
        self.inner
            .as_delete_user(params)
            .await
            .map_err(DeleteUserError)?;
        Ok(Response::new(DeleteUserResponse {}))
    }

    async fn publish_connection_packages(
        &self,
        request: Request<PublishConnectionPackagesRequest>,
    ) -> Result<Response<PublishConnectionPackagesResponse>, Status> {
        let request = request.into_inner();
        let (client_id, payload) = self
            .verify_client_auth::<_, PublishConnectionPackagesPayload>(request)
            .await?;
        let params = AsPublishConnectionPackagesParamsTbsIn {
            client_id,
            connection_packages: payload
                .connection_packages
                .into_iter()
                .map(|package| package.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        };
        self.inner
            .as_publish_connection_packages(params)
            .await
            .map_err(PublishConnectionPackagesError)?;
        Ok(Response::new(PublishConnectionPackagesResponse {}))
    }

    async fn get_connection_package(
        &self,
        request: Request<GetConnectionPackageRequest>,
    ) -> Result<Response<GetConnectionPackageResponse>, Status> {
        let request = request.into_inner();
        let (client_id, _payload) = self
            .verify_client_auth::<_, GetConnectionPackagePayload>(request)
            .await?;
        let params = ClientConnectionPackageParamsTbs(client_id);
        let connection_package = self
            .inner
            .as_client_key_package(params)
            .await
            .map_err(GetConnectionPackageError)?
            .connection_package;
        Ok(Response::new(GetConnectionPackageResponse {
            connection_package: connection_package.map(Into::into),
        }))
    }

    async fn as_credentials(
        &self,
        _request: Request<AsCredentialsRequest>,
    ) -> Result<Response<AsCredentialsResponse>, Status> {
        let response = self
            .inner
            .as_credentials(AsCredentialsParams {})
            .await
            .map_err(AsCredentialsError)?;
        Ok(Response::new(AsCredentialsResponse {
            as_credentials: response
                .as_credentials
                .into_iter()
                .map(From::from)
                .collect(),
            as_intermediate_credentials: response
                .as_intermediate_credentials
                .into_iter()
                .map(From::from)
                .collect(),
            revoked_credentials: response
                .revoked_credentials
                .into_iter()
                .map(From::from)
                .collect(),
        }))
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

struct DeleteUserError(errors::auth_service::DeleteUserError);

impl From<DeleteUserError> for Status {
    fn from(e: DeleteUserError) -> Self {
        match e.0 {
            errors::auth_service::DeleteUserError::StorageError => {
                Status::internal(e.0.to_string())
            }
        }
    }
}

struct PublishConnectionPackagesError(errors::auth_service::PublishConnectionPackageError);

impl From<PublishConnectionPackagesError> for Status {
    fn from(e: PublishConnectionPackagesError) -> Self {
        match e.0 {
            errors::auth_service::PublishConnectionPackageError::StorageError => {
                Status::internal(e.0.to_string())
            }
            errors::auth_service::PublishConnectionPackageError::InvalidKeyPackage => {
                Status::invalid_argument(e.0.to_string())
            }
        }
    }
}

struct GetConnectionPackageError(errors::auth_service::ClientKeyPackageError);

impl From<GetConnectionPackageError> for Status {
    fn from(e: GetConnectionPackageError) -> Self {
        match e.0 {
            errors::auth_service::ClientKeyPackageError::StorageError => {
                Status::internal(e.0.to_string())
            }
        }
    }
}

struct AsCredentialsError(errors::auth_service::AsCredentialsError);

impl From<AsCredentialsError> for Status {
    fn from(e: AsCredentialsError) -> Self {
        match e.0 {
            errors::auth_service::AsCredentialsError::StorageError => {
                Status::internal(e.0.to_string())
            }
        }
    }
}

trait WithAsClientId {
    fn client_id_proto(&self) -> Option<AsClientId>;
    fn client_id(&self) -> Result<identifiers::AsClientId, Status> {
        Ok(self
            .client_id_proto()
            .ok_or_missing_field("client_id")?
            .try_into()?)
    }
}

impl WithAsClientId for DeleteUserRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.client_id.clone()
    }
}

impl WithAsClientId for PublishConnectionPackagesRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.client_id.clone()
    }
}

impl WithAsClientId for GetConnectionPackageRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.client_id.clone()
    }
}
