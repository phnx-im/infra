// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{auth_service_server, *},
    validation::MissingFieldExt,
};

use phnxtypes::{
    crypto::{
        indexed_aead::keys::UserProfileKeyIndex,
        signatures::{
            private_keys::SignatureVerificationError,
            signable::{Verifiable, VerifiedStruct},
        },
    },
    identifiers,
    messages::{
        client_as::{
            AsCredentialsParams, DeleteUserParamsTbs, DequeueMessagesParamsTbs,
            EnqueueMessageParams, UserConnectionPackagesParams,
        },
        client_as_out::{
            GetUserProfileParams, MergeUserProfileParamsTbs, RegisterUserParamsIn,
            StageUserProfileParamsTbs,
        },
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
        let response = self.inner.as_init_user_registration(params).await?;
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
        self.inner.as_delete_user(params).await?;
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
        let connection_packages = payload
            .connection_packages
            .into_iter()
            .map(|package| package.try_into())
            .collect::<Result<Vec<_>, _>>()?;
        self.inner
            .as_publish_connection_packages(client_id, connection_packages)
            .await?;
        Ok(Response::new(PublishConnectionPackagesResponse {}))
    }

    async fn get_user_connection_packages(
        &self,
        request: Request<GetUserConnectionPackagesRequest>,
    ) -> Result<Response<GetUserConnectionPackagesResponse>, Status> {
        let request = request.into_inner();
        let client_id = request
            .client_id
            .ok_or_missing_field("client_id")?
            .try_into()?;
        let params = UserConnectionPackagesParams { client_id };
        let connection_packages = self
            .inner
            .as_user_connection_packages(params)
            .await?
            .key_packages;
        Ok(Response::new(GetUserConnectionPackagesResponse {
            connection_packages: connection_packages.into_iter().map(Into::into).collect(),
        }))
    }

    async fn as_credentials(
        &self,
        _request: Request<AsCredentialsRequest>,
    ) -> Result<Response<AsCredentialsResponse>, Status> {
        let response = self.inner.as_credentials(AsCredentialsParams {}).await?;
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

    async fn stage_user_profile(
        &self,
        request: Request<StageUserProfileRequest>,
    ) -> Result<Response<StageUserProfileResponse>, Status> {
        let request = request.into_inner();
        let (client_id, payload) = self
            .verify_client_auth::<_, StageUserProfilePayload>(request)
            .await?;
        let params = StageUserProfileParamsTbs {
            client_id,
            user_profile: payload
                .encrypted_user_profile
                .ok_or_missing_field("encrypted_user_profile")?
                .try_into()?,
        };
        self.inner.as_stage_user_profile(params).await?;
        Ok(Response::new(StageUserProfileResponse {}))
    }

    async fn merge_user_profile(
        &self,
        request: Request<MergeUserProfileRequest>,
    ) -> Result<Response<MergeUserProfileResponse>, Status> {
        let request = request.into_inner();
        let (client_id, _payload) = self
            .verify_client_auth::<_, MergeUserProfilePayload>(request)
            .await?;
        let params = MergeUserProfileParamsTbs { client_id };
        self.inner.as_merge_user_profile(params).await?;
        Ok(Response::new(MergeUserProfileResponse {}))
    }

    async fn get_user_profile(
        &self,
        request: Request<GetUserProfileRequest>,
    ) -> Result<Response<GetUserProfileResponse>, Status> {
        let request = request.into_inner();
        let client_id = request
            .client_id
            .ok_or_missing_field("client_id")?
            .try_into()?;
        let key_index = UserProfileKeyIndex::from_bytes(request.key_index.try_into().map_err(
            |bytes: Vec<u8>| {
                Status::invalid_argument(format!("invalid key index length: {}", bytes.len()))
            },
        )?);
        let params = GetUserProfileParams {
            client_id,
            key_index,
        };
        let response = self.inner.as_get_user_profile(params).await?;
        Ok(Response::new(GetUserProfileResponse {
            encrypted_user_profile: Some(response.encrypted_user_profile.into()),
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
        request: Request<EnqueueMessagesRequest>,
    ) -> Result<Response<EnqueueMessagesResponse>, Status> {
        let request = request.into_inner();
        let params = EnqueueMessageParams {
            client_id: request
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()?,
            connection_establishment_ctxt: request
                .connection_establishment_package
                .ok_or_missing_field("connection_establishment_package")?
                .try_into()?,
        };
        self.inner.as_enqueue_message(params).await?;
        Ok(Response::new(EnqueueMessagesResponse {}))
    }

    async fn dequeue_messages(
        &self,
        request: Request<DequeueMessagesRequest>,
    ) -> Result<Response<DequeueMessagesResponse>, Status> {
        let request = request.into_inner();
        let (sender, payload) = self
            .verify_client_auth::<_, DequeueMessagesPayload>(request)
            .await?;
        let params = DequeueMessagesParamsTbs {
            sender,
            sequence_number_start: payload.sequence_number_start,
            max_message_number: payload.max_message_number,
        };
        let response = self.inner.as_dequeue_messages(params).await?;
        Ok(Response::new(DequeueMessagesResponse {
            messages: response.messages.into_iter().map(Into::into).collect(),
            remaining_messages_number: response.remaining_messages_number,
        }))
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

impl WithAsClientId for DequeueMessagesRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.sender.clone()
    }
}

impl WithAsClientId for StageUserProfileRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.client_id.clone()
    }
}

impl WithAsClientId for MergeUserProfileRequest {
    fn client_id_proto(&self) -> Option<AsClientId> {
        self.payload.as_ref()?.client_id.clone()
    }
}
