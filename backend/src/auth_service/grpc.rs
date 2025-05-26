// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use futures_util::stream::BoxStream;
use phnxprotos::{
    auth_service::v1::{auth_service_server, *},
    validation::MissingFieldExt,
};

use phnxcommon::{
    credentials::keys,
    crypto::{
        indexed_aead::keys::UserProfileKeyIndex,
        signatures::{
            private_keys::{SignatureVerificationError, VerifyingKeyBehaviour},
            signable::{Verifiable, VerifiedStruct},
        },
    },
    identifiers,
    messages::{
        client_as::{AsCredentialsParams, EnqueueMessageParams, UserConnectionPackagesParams},
        client_as_out::{
            GetUserProfileParams, MergeUserProfileParamsTbs, RegisterUserParamsIn,
            StageUserProfileParamsTbs,
        },
    },
};
use privacypass::{amortized_tokens::AmortizedBatchTokenRequest, private_tokens::Ristretto255};
use tls_codec::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming, async_trait};
use tracing::error;

use super::{
    AuthService,
    client_record::ClientRecord,
    queue::Queues,
    user_handles::{ConnectHandleProtocol, UserHandleRecord},
};

pub struct GrpcAs {
    inner: AuthService,
}

impl GrpcAs {
    pub fn new(inner: AuthService) -> Self {
        Self { inner }
    }

    async fn verify_user_auth<R, P>(&self, request: R) -> Result<(identifiers::UserId, P), Status>
    where
        R: WithUserId + Verifiable,
        P: VerifiedStruct<R>,
    {
        let user_id = request.user_id()?;
        let client_verifying_key = self.load_client_verifying_key(&user_id).await?;
        let payload = self.verify_request(request, &client_verifying_key)?;
        Ok((user_id, payload))
    }

    async fn load_client_verifying_key(
        &self,
        user_id: &identifiers::UserId,
    ) -> Result<keys::ClientVerifyingKey, Status> {
        let client_record = ClientRecord::load(&self.inner.db_pool, user_id)
            .await
            .map_err(|error| {
                error!(%error, ?user_id, "failed to load client");
                Status::internal("database error")
            })?
            .ok_or_else(|| Status::not_found("unknown client"))?;
        Ok(client_record.credential.verifying_key().clone())
    }

    async fn verify_handle_auth<R, P>(
        &self,
        request: R,
    ) -> Result<(identifiers::UserHandleHash, P), Status>
    where
        R: WithUserHandleHash + Verifiable,
        P: VerifiedStruct<R>,
    {
        let hash = request.user_handle_hash()?;
        let verifying_key = self.load_handle_verifying_key(hash).await?;
        let payload = self.verify_request(request, &verifying_key)?;
        Ok((hash, payload))
    }

    async fn load_handle_verifying_key(
        &self,
        hash: identifiers::UserHandleHash,
    ) -> Result<keys::HandleVerifyingKey, Status> {
        UserHandleRecord::load_verifying_key(&self.inner.db_pool, &hash)
            .await
            .map_err(|error| {
                error!(%error, "failed to load verifying key");
                Status::internal("database error")
            })?
            .ok_or_else(|| Status::not_found("unknown handle"))
    }

    #[expect(clippy::result_large_err)]
    fn verify_request<R, P>(
        &self,
        request: R,
        verifying_key: impl VerifyingKeyBehaviour,
    ) -> Result<P, Status>
    where
        R: Verifiable,
        P: VerifiedStruct<R>,
    {
        request.verify(verifying_key).map_err(|error| match error {
            SignatureVerificationError::VerificationFailure => {
                Status::unauthenticated("invalid signature")
            }
            SignatureVerificationError::LibraryError(_) => Status::internal("unrecoverable error"),
        })
    }

    async fn process_listen_requests_task(
        queues: Queues,
        user_id: identifiers::UserId,
        mut requests: Streaming<ListenRequest>,
    ) {
        while let Some(request) = requests.next().await {
            if let Err(error) = Self::process_listen_request(&queues, &user_id, request).await {
                // We report the error, but don't stop processing requests.
                // TODO(#466): Send this to the client.
                error!(%error, "error processing listen request");
            }
        }
    }

    async fn process_listen_request(
        queues: &Queues,
        user_id: &identifiers::UserId,
        request: Result<ListenRequest, Status>,
    ) -> Result<(), Status> {
        let request = request?;
        let Some(listen_request::Request::Ack(ack_request)) = request.request else {
            return Err(ListenProtocolViolation::OnlyAckRequestAllowed.into());
        };
        queues
            .ack(user_id, ack_request.up_to_sequence_number)
            .await?;
        Ok(())
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
        let (user_id, payload) = self
            .verify_user_auth::<_, DeleteUserPayload>(request)
            .await?;
        let payload_user_id: identifiers::UserId =
            payload.user_id.ok_or_missing_field("user_id")?.try_into()?;
        if payload_user_id != user_id {
            return Err(Status::invalid_argument("only possible to delete own user"));
        }
        self.inner.as_delete_user(&user_id).await?;
        Ok(Response::new(DeleteUserResponse {}))
    }

    async fn publish_connection_packages(
        &self,
        request: Request<PublishConnectionPackagesRequest>,
    ) -> Result<Response<PublishConnectionPackagesResponse>, Status> {
        let request = request.into_inner();

        match request
            .payload
            .as_ref()
            .ok_or_missing_field("payload")?
            .owner
            .clone()
            .ok_or_missing_field("owner")?
        {
            // publishing connection packages for a user
            publish_connection_packages_payload::Owner::UserId(user_id) => {
                let user_id: identifiers::UserId = user_id.try_into()?;
                let client_verifying_key = self.load_client_verifying_key(&user_id).await?;
                let payload = self.verify_request::<_, PublishConnectionPackagesPayload>(
                    request,
                    &client_verifying_key,
                )?;
                let connection_packages = payload
                    .connection_packages
                    .into_iter()
                    .map(|package| package.try_into())
                    .collect::<Result<Vec<_>, _>>()?;
                self.inner
                    .as_publish_connection_packages(user_id, connection_packages)
                    .await?;
            }
            // publishing connection packages for a user handle
            publish_connection_packages_payload::Owner::Hash(hash) => {
                let hash: identifiers::UserHandleHash = hash.try_into()?;
                let handle_verifying_key = self.load_handle_verifying_key(hash).await?;
                let payload = self.verify_request::<_, PublishConnectionPackagesPayload>(
                    request,
                    &handle_verifying_key,
                )?;
                let connection_packages = payload
                    .connection_packages
                    .into_iter()
                    .map(|package| package.try_into())
                    .collect::<Result<Vec<_>, _>>()?;
                self.inner
                    .as_publish_connection_packages_for_handle(&hash, connection_packages)
                    .await?;
            }
        };

        Ok(Response::new(PublishConnectionPackagesResponse {}))
    }

    async fn get_user_connection_packages(
        &self,
        request: Request<GetUserConnectionPackagesRequest>,
    ) -> Result<Response<GetUserConnectionPackagesResponse>, Status> {
        let request = request.into_inner();
        let user_id = request.user_id.ok_or_missing_field("user_id")?.try_into()?;
        let params = UserConnectionPackagesParams { user_id };
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
        let (user_id, payload) = self
            .verify_user_auth::<_, StageUserProfilePayload>(request)
            .await?;
        let params = StageUserProfileParamsTbs {
            user_id,
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
        let (user_id, _payload) = self
            .verify_user_auth::<_, MergeUserProfilePayload>(request)
            .await?;
        let params = MergeUserProfileParamsTbs { user_id };
        self.inner.as_merge_user_profile(params).await?;
        Ok(Response::new(MergeUserProfileResponse {}))
    }

    async fn get_user_profile(
        &self,
        request: Request<GetUserProfileRequest>,
    ) -> Result<Response<GetUserProfileResponse>, Status> {
        let request = request.into_inner();
        let user_id = request.user_id.ok_or_missing_field("user_id")?.try_into()?;
        let key_index = UserProfileKeyIndex::from_bytes(request.key_index.try_into().map_err(
            |bytes: Vec<u8>| {
                Status::invalid_argument(format!("invalid key index length: {}", bytes.len()))
            },
        )?);
        let params = GetUserProfileParams { user_id, key_index };
        let response = self.inner.as_get_user_profile(params).await?;
        Ok(Response::new(GetUserProfileResponse {
            encrypted_user_profile: Some(response.encrypted_user_profile.into()),
        }))
    }

    async fn issue_tokens(
        &self,
        request: Request<IssueTokensRequest>,
    ) -> Result<Response<IssueTokensResponse>, Status> {
        let request = request.into_inner();
        let (user_id, payload) = self
            .verify_user_auth::<_, IssueTokensPayload>(request)
            .await?;

        let token_request: AmortizedBatchTokenRequest<Ristretto255> =
            AmortizedBatchTokenRequest::tls_deserialize_exact(payload.token_request.as_slice())
                .map_err(|_| Status::invalid_argument("invalid token request"))?;

        let token_response = self
            .inner
            .as_issue_tokens(&user_id, token_request)
            .await?
            .tls_serialize_detached()
            .map_err(|_| Status::internal("failed to serialize token response"))?;

        Ok(Response::new(IssueTokensResponse { token_response }))
    }

    type ListenStream = BoxStream<'static, Result<ListenResponse, Status>>;

    async fn listen(
        &self,
        request: Request<Streaming<ListenRequest>>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        let mut requests = request.into_inner();

        let request = requests
            .next()
            .await
            .ok_or(ListenProtocolViolation::MissingInitRequest)??;
        let Some(listen_request::Request::Init(init_request)) = request.request else {
            return Err(Status::failed_precondition("missing initial request"));
        };

        let (user_id, payload) = self
            .verify_user_auth::<_, InitListenPayload>(init_request)
            .await?;

        let messages = self
            .inner
            .queues
            .listen(&user_id, payload.sequence_number_start)
            .await?;

        tokio::spawn(Self::process_listen_requests_task(
            self.inner.queues.clone(),
            user_id.clone(),
            requests,
        ));

        let responses = Box::pin(messages.map(|message| {
            Ok(ListenResponse {
                message: message.map(From::from),
            })
        }));

        Ok(Response::new(responses))
    }

    async fn enqueue_messages(
        &self,
        request: Request<EnqueueMessagesRequest>,
    ) -> Result<Response<EnqueueMessagesResponse>, Status> {
        let request = request.into_inner();
        let params = EnqueueMessageParams {
            user_id: request.user_id.ok_or_missing_field("user_id")?.try_into()?,
            connection_establishment_ctxt: request
                .connection_establishment_package
                .ok_or_missing_field("connection_establishment_package")?
                .try_into()?,
        };
        self.inner.as_enqueue_message(params).await?;
        Ok(Response::new(EnqueueMessagesResponse {}))
    }

    async fn create_handle(
        &self,
        request: Request<CreateHandleRequest>,
    ) -> Result<Response<CreateHandleResponse>, Status> {
        let request = request.into_inner();

        let verifying_key = request
            .payload
            .as_ref()
            .ok_or_missing_field("payload")?
            .verifying_key
            .clone()
            .ok_or_missing_field("verifying_key")?
            .into();
        let payload = self.verify_request::<_, CreateHandlePayload>(request, &verifying_key)?;

        let hash = payload.hash.ok_or_missing_field("hash")?.try_into()?;

        self.inner
            .as_create_handle(verifying_key, payload.plaintext, hash)
            .await?;

        Ok(Response::new(CreateHandleResponse {}))
    }

    async fn delete_handle(
        &self,
        request: Request<DeleteHandleRequest>,
    ) -> Result<Response<DeleteHandleResponse>, Status> {
        let request = request.into_inner();

        let (hash, _payload) = self
            .verify_handle_auth::<_, DeleteHandlePayload>(request)
            .await?;

        self.inner.as_delete_handle(hash).await?;

        Ok(Response::new(DeleteHandleResponse {}))
    }

    async fn refresh_handle(
        &self,
        request: Request<RefreshHandleRequest>,
    ) -> Result<Response<RefreshHandleResponse>, Status> {
        let request = request.into_inner();

        let (hash, _payload) = self
            .verify_handle_auth::<_, RefreshHandlePayload>(request)
            .await?;

        self.inner.as_refresh_handle(hash).await?;

        Ok(Response::new(RefreshHandleResponse {}))
    }

    type ConnectHandleStream = BoxStream<'static, Result<ConnectResponse, Status>>;

    async fn connect_handle(
        &self,
        request: Request<Streaming<ConnectRequest>>,
    ) -> Result<Response<Self::ConnectHandleStream>, Status> {
        let incoming = request.into_inner();
        let (outgoing_tx, outgoing_rx) = mpsc::channel(1);

        // protocol
        tokio::spawn(
            self.inner
                .clone()
                .connect_handle_protocol(incoming, outgoing_tx),
        );

        let outgoing = tokio_stream::wrappers::ReceiverStream::new(outgoing_rx);
        Ok(Response::new(Box::pin(outgoing)))
    }

    type ListenHandleStream = BoxStream<'static, Result<ListenHandleResponse, Status>>;

    async fn listen_handle(
        &self,
        _request: Request<Streaming<ListenHandleRequest>>,
    ) -> Result<Response<Self::ListenHandleStream>, Status> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
enum ListenProtocolViolation {
    #[error("missing initial request")]
    MissingInitRequest,
    #[error("only ack request allowed")]
    OnlyAckRequestAllowed,
}

impl From<ListenProtocolViolation> for Status {
    fn from(error: ListenProtocolViolation) -> Self {
        Status::failed_precondition(error.to_string())
    }
}

trait WithUserId {
    fn user_id_proto(&self) -> Option<UserId>;

    #[expect(clippy::result_large_err)]
    fn user_id(&self) -> Result<identifiers::UserId, Status> {
        Ok(self
            .user_id_proto()
            .ok_or_missing_field("user_id")?
            .try_into()?)
    }
}

impl WithUserId for DeleteUserRequest {
    fn user_id_proto(&self) -> Option<UserId> {
        self.payload.as_ref()?.user_id.clone()
    }
}

impl WithUserId for StageUserProfileRequest {
    fn user_id_proto(&self) -> Option<UserId> {
        self.payload.as_ref()?.user_id.clone()
    }
}

impl WithUserId for MergeUserProfileRequest {
    fn user_id_proto(&self) -> Option<UserId> {
        self.payload.as_ref()?.user_id.clone()
    }
}

impl WithUserId for InitListenRequest {
    fn user_id_proto(&self) -> Option<UserId> {
        self.payload.as_ref()?.user_id.clone()
    }
}

impl WithUserId for IssueTokensRequest {
    fn user_id_proto(&self) -> Option<UserId> {
        self.payload.as_ref()?.user_id.clone()
    }
}

trait WithUserHandleHash {
    fn user_handle_hash_proto(&self) -> Option<UserHandleHash>;

    #[expect(clippy::result_large_err)]
    fn user_handle_hash(&self) -> Result<identifiers::UserHandleHash, Status> {
        Ok(self
            .user_handle_hash_proto()
            .ok_or_missing_field("user_handle_hash")?
            .try_into()?)
    }
}

impl WithUserHandleHash for CreateHandleRequest {
    fn user_handle_hash_proto(&self) -> Option<UserHandleHash> {
        self.payload.as_ref()?.hash.clone()
    }
}

impl WithUserHandleHash for DeleteHandleRequest {
    fn user_handle_hash_proto(&self) -> Option<UserHandleHash> {
        self.payload.as_ref()?.hash.clone()
    }
}

impl WithUserHandleHash for RefreshHandleRequest {
    fn user_handle_hash_proto(&self) -> Option<UserHandleHash> {
        self.payload.as_ref()?.hash.clone()
    }
}
