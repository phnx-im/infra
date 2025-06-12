// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::convert::identity;

use futures_util::{FutureExt, future::BoxFuture};
use phnxcommon::{
    LibraryError,
    credentials::{
        ClientCredentialPayload,
        keys::{ClientSigningKey, HandleSigningKey},
    },
    crypto::{
        RatchetEncryptionKey, indexed_aead::keys::UserProfileKeyIndex, kdf::keys::RatchetSecret,
        signatures::signable::Signable,
    },
    identifiers::{UserHandle, UserHandleHash, UserId},
    messages::{
        QueueMessage,
        client_as::{ConnectionPackage, EncryptedConnectionOffer, UserConnectionPackagesParams},
        client_as_out::{
            AsCredentialsResponseIn, ConnectionPackageIn, EncryptedUserProfile,
            GetUserProfileResponse, RegisterUserResponseIn, UserConnectionPackagesResponseIn,
        },
    },
};
use phnxprotos::auth_service::v1::{
    AckListenHandleRequest, AckListenRequest, AsCredentialsRequest, ConnectRequest,
    ConnectResponse, CreateHandlePayload, DeleteHandlePayload, DeleteUserPayload,
    EnqueueConnectionOfferStep, EnqueueMessagesRequest, FetchConnectionPackageStep,
    GetUserConnectionPackagesRequest, GetUserProfileRequest, HandleQueueMessage,
    InitListenHandlePayload, InitListenPayload, ListenHandleRequest, ListenRequest,
    MergeUserProfilePayload, PublishConnectionPackagesPayload, RegisterUserRequest,
    StageUserProfilePayload, connect_request, connect_response, listen_handle_request,
    listen_request, publish_connection_packages_payload,
};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::Request;
use tracing::error;
use uuid::Uuid;

pub mod grpc;

use crate::ApiClient;

#[derive(Error, Debug)]
pub enum AsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Received an unexpected response type")]
    UnexpectedResponse,
    #[error(transparent)]
    Tonic(#[from] tonic::Status),
}

impl From<LibraryError> for AsRequestError {
    fn from(_: LibraryError) -> Self {
        AsRequestError::LibraryError
    }
}

impl ApiClient {
    pub async fn as_register_user(
        &self,
        client_payload: ClientCredentialPayload,
        queue_encryption_key: RatchetEncryptionKey,
        initial_ratchet_secret: RatchetSecret,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Result<RegisterUserResponseIn, AsRequestError> {
        let request = RegisterUserRequest {
            client_credential_payload: Some(client_payload.into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            initial_ratchet_secret: Some(initial_ratchet_secret.into()),
            encrypted_user_profile: Some(encrypted_user_profile.into()),
        };
        let response = self
            .as_grpc_client
            .client()
            .register_user(Request::new(request))
            .await?
            .into_inner();
        Ok(RegisterUserResponseIn {
            client_credential: response
                .client_credential
                .ok_or_else(|| {
                    error!("missing `client_credential` in response");
                    AsRequestError::UnexpectedResponse
                })?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid client_credential in response");
                    AsRequestError::UnexpectedResponse
                })?,
        })
    }

    pub async fn as_get_user_profile(
        &self,
        user_id: UserId,
        key_index: UserProfileKeyIndex,
    ) -> Result<GetUserProfileResponse, AsRequestError> {
        let request = GetUserProfileRequest {
            user_id: Some(user_id.into()),
            key_index: key_index.into_bytes().to_vec(),
        };
        let response = self
            .as_grpc_client
            .client()
            .get_user_profile(request)
            .await?
            .into_inner();
        Ok(GetUserProfileResponse {
            encrypted_user_profile: response
                .encrypted_user_profile
                .ok_or_else(|| {
                    error!("missing `encrypted_user_profile` in response");
                    AsRequestError::UnexpectedResponse
                })?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid encrypted_user_profile in response");
                    AsRequestError::UnexpectedResponse
                })?,
        })
    }

    pub async fn as_stage_user_profile(
        &self,
        user_id: UserId,
        signing_key: &ClientSigningKey,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Result<(), AsRequestError> {
        let payload = StageUserProfilePayload {
            user_id: Some(user_id.into()),
            encrypted_user_profile: Some(encrypted_user_profile.into()),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client
            .client()
            .stage_user_profile(request)
            .await?;
        Ok(())
    }

    pub async fn as_merge_user_profile(
        &self,
        user_id: UserId,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = MergeUserProfilePayload {
            user_id: Some(user_id.into()),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client
            .client()
            .merge_user_profile(request)
            .await?;
        Ok(())
    }

    pub async fn as_delete_user(
        &self,
        user_id: UserId,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = DeleteUserPayload {
            user_id: Some(user_id.into()),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client.client().delete_user(request).await?;
        Ok(())
    }

    pub async fn as_listen(
        &self,
        sequence_number_start: u64,
        signing_key: &ClientSigningKey,
    ) -> Result<
        (
            impl Stream<Item = Option<QueueMessage>> + Send,
            ListenResponder,
        ),
        AsRequestError,
    > {
        let init_payload = InitListenPayload {
            user_id: Some(signing_key.credential().identity().clone().into()),
            sequence_number_start,
        };
        let init_request = init_payload.sign(signing_key)?;

        const ACK_CHANNEL_BUFFER_SIZE: usize = 16; // not too big for applying backpressure
        let (requests_tx, requests_rx) = mpsc::channel(ACK_CHANNEL_BUFFER_SIZE);
        requests_tx
            .send(ListenRequest {
                request: Some(listen_request::Request::Init(init_request)),
            })
            .await
            .map_err(|_| {
                error!("logic error: channel closed");
                tonic::Status::internal("logic error")
            })?;

        let mut client = self.as_grpc_client.client();

        let responses = client
            .listen(ReceiverStream::new(requests_rx))
            .await?
            .into_inner();

        let messages = responses.filter_map(|response| -> Option<Option<QueueMessage>> {
            let response = response
                .inspect_err(|error| {
                    error!(%error, "error receiving response");
                })
                .ok()?;
            let Some(message) = response.message else {
                return Some(None); // sentinel value
            };
            let message = message
                .try_into()
                .inspect_err(|error| {
                    error!(%error, "invalid message in response");
                })
                .ok()?;
            Some(Some(message))
        });

        let responder = ListenResponder { tx: requests_tx };

        Ok((messages, responder))
    }

    pub async fn as_publish_connection_packages(
        &self,
        user_id: UserId,
        connection_packages: Vec<ConnectionPackage>,
        signing_key: &ClientSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = PublishConnectionPackagesPayload {
            owner: Some(publish_connection_packages_payload::Owner::UserId(
                user_id.into(),
            )),
            connection_packages: connection_packages.into_iter().map(From::from).collect(),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client
            .client()
            .publish_connection_packages(request)
            .await?;
        Ok(())
    }

    pub async fn as_publish_connection_packages_for_handle(
        &self,
        hash: UserHandleHash,
        connection_packages: Vec<ConnectionPackage>,
        signing_key: &HandleSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = PublishConnectionPackagesPayload {
            owner: Some(publish_connection_packages_payload::Owner::Hash(
                hash.into(),
            )),
            connection_packages: connection_packages.into_iter().map(From::from).collect(),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client
            .client()
            .publish_connection_packages(request)
            .await?;
        Ok(())
    }

    pub async fn as_user_connection_packages(
        &self,
        payload: UserConnectionPackagesParams,
    ) -> Result<UserConnectionPackagesResponseIn, AsRequestError> {
        let request = GetUserConnectionPackagesRequest {
            user_id: Some(payload.user_id.into()),
        };
        let response = self
            .as_grpc_client
            .client()
            .get_user_connection_packages(request)
            .await?
            .into_inner();
        let connection_packages = response
            .connection_packages
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()
            .map_err(|error| {
                error!(%error, "failed to convert connection package");
                AsRequestError::UnexpectedResponse
            })?;
        Ok(UserConnectionPackagesResponseIn {
            connection_packages,
        })
    }

    pub async fn as_enqueue_message(
        &self,
        user_id: UserId,
        connection_offer: EncryptedConnectionOffer,
    ) -> Result<(), AsRequestError> {
        let request = EnqueueMessagesRequest {
            user_id: Some(user_id.into()),
            connection_offer: Some(connection_offer.into()),
        };
        self.as_grpc_client
            .client()
            .enqueue_messages(request)
            .await?;
        Ok(())
    }

    pub async fn as_connect_handle(
        &self,
        handle: &UserHandle,
    ) -> Result<(ConnectionPackageIn, ConnectionOfferResponder), AsRequestError> {
        let hash = handle.hash().map_err(|error| {
            error!(%error, "failed to hash user handle");
            AsRequestError::LibraryError
        })?;

        // Step 1: Fetch connection package
        let fetch_request = ConnectRequest {
            step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                hash: Some(hash.into()),
            })),
        };

        // Step 2: Enqueue connection offer
        let (connection_offer_tx, connection_offer_rx) =
            oneshot::channel::<EncryptedConnectionOffer>();
        let connection_offer_fut = async move {
            let connection_offer = connection_offer_rx.await.ok()?;
            Some(ConnectRequest {
                step: Some(connect_request::Step::Enqueue(EnqueueConnectionOfferStep {
                    connection_offer: Some(connection_offer.into()),
                })),
            })
        };

        let requests = tokio_stream::once(Some(fetch_request))
            .chain(connection_offer_fut.into_stream())
            .filter_map(identity);
        let mut responses = self
            .as_grpc_client
            .client()
            .connect_handle(requests)
            .await?
            .into_inner();

        let response = responses.next().await.ok_or_else(|| {
            error!("protocol violation: missing response");
            AsRequestError::UnexpectedResponse
        })??;

        let connection_package: ConnectionPackageIn = match response {
            ConnectResponse {
                step: Some(connect_response::Step::FetchResponse(fetch)),
            } => fetch
                .connection_package
                .ok_or_else(|| {
                    error!("protocol violation: missing connection package");
                    AsRequestError::UnexpectedResponse
                })?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid connection package");
                    AsRequestError::UnexpectedResponse
                })?,
            _ => {
                error!("protocol violation: expected fetch response");
                return Err(AsRequestError::UnexpectedResponse);
            }
        };

        let connection_offer_response_fut = async move {
            let response = responses.next().await.ok_or_else(|| {
                error!("protocol violation: missing connection offer response");
                AsRequestError::UnexpectedResponse
            })??;
            match response {
                ConnectResponse {
                    step: Some(connect_response::Step::EnqueueResponse(_)),
                } => Ok(()),
                _ => {
                    error!("protocol violation: expected connection offer response");
                    Err(AsRequestError::UnexpectedResponse)
                }
            }
        };

        let responder =
            ConnectionOfferResponder::new(connection_offer_tx, connection_offer_response_fut);
        Ok((connection_package, responder))
    }

    pub async fn as_listen_handle(
        &self,
        hash: UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> Result<
        (
            impl Stream<Item = Option<HandleQueueMessage>> + Send + use<>,
            ListenHandleResponder,
        ),
        AsRequestError,
    > {
        let init_payload = InitListenHandlePayload {
            hash: Some(hash.into()),
        };
        let init_request = init_payload.sign(signing_key)?;

        const ACK_CHANNEL_BUFFER_SIZE: usize = 16; // not too big for applying backpressure
        let (ack_tx, ack_rx) = mpsc::channel::<Uuid>(ACK_CHANNEL_BUFFER_SIZE);

        let requests = tokio_stream::once(ListenHandleRequest {
            request: Some(listen_handle_request::Request::Init(init_request)),
        })
        .chain(
            ReceiverStream::new(ack_rx).map(|message_id| ListenHandleRequest {
                request: Some(listen_handle_request::Request::Ack(
                    AckListenHandleRequest {
                        message_id: Some(message_id.into()),
                    },
                )),
            }),
        );

        let responses = self
            .as_grpc_client
            .client()
            .listen_handle(requests)
            .await?
            .into_inner();

        let responses = responses.map_while(move |response| {
            let response = response
                .inspect_err(|error| {
                    error!(%error, "stop handle listen stream");
                })
                .ok()?;
            Some(response.message)
        });

        let responder = ListenHandleResponder { tx: ack_tx };

        Ok((responses, responder))
    }

    pub async fn as_as_credentials(&self) -> Result<AsCredentialsResponseIn, AsRequestError> {
        let request = AsCredentialsRequest {};
        let response = self
            .as_grpc_client
            .client()
            .as_credentials(request)
            .await?
            .into_inner();
        Ok(AsCredentialsResponseIn {
            as_credentials: response
                .as_credentials
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    error!(%error, "invalid AS credential");
                    AsRequestError::UnexpectedResponse
                })?,
            as_intermediate_credentials: response
                .as_intermediate_credentials
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    error!(%error, "invalid AS intermediate credential");
                    AsRequestError::UnexpectedResponse
                })?,
            revoked_credentials: response
                .revoked_credentials
                .into_iter()
                .map(From::from)
                .collect(),
        })
    }

    pub async fn as_create_handle(
        &self,
        user_handle: &UserHandle,
        hash: UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> Result<bool, AsRequestError> {
        let payload = CreateHandlePayload {
            verifying_key: Some(signing_key.verifying_key().clone().into()),
            plaintext: user_handle.plaintext().into(),
            hash: Some(hash.into()),
        };
        let request = payload.sign(signing_key)?;
        match self.as_grpc_client.client().create_handle(request).await {
            Ok(_) => Ok(true),
            Err(e) if e.code() == tonic::Code::AlreadyExists => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn as_delete_handle(
        &self,
        hash: UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> Result<(), AsRequestError> {
        let payload = DeleteHandlePayload {
            hash: Some(hash.into()),
        };
        let request = payload.sign(signing_key)?;
        self.as_grpc_client.client().delete_handle(request).await?;
        Ok(())
    }
}

pub struct ListenResponder {
    tx: mpsc::Sender<ListenRequest>,
}

impl ListenResponder {
    pub async fn ack(&self, up_to_sequence_number: u64) {
        let ack_request = listen_request::Request::Ack(AckListenRequest {
            up_to_sequence_number,
        });
        let request = ListenRequest {
            request: Some(ack_request),
        };
        let _ = self.tx.send(request).await;
    }
}

#[derive(Debug)]
pub struct ListenHandleResponder {
    tx: mpsc::Sender<Uuid>,
}

impl ListenHandleResponder {
    pub async fn ack(&self, message_id: Uuid) {
        let _ = self.tx.send(message_id).await;
    }
}

pub struct ConnectionOfferResponder {
    tx: oneshot::Sender<EncryptedConnectionOffer>,
    response: BoxFuture<'static, Result<(), AsRequestError>>,
}

impl ConnectionOfferResponder {
    pub fn new(
        tx: oneshot::Sender<EncryptedConnectionOffer>,
        response: impl Future<Output = Result<(), AsRequestError>> + Send + 'static,
    ) -> Self {
        Self {
            tx,
            response: Box::pin(response),
        }
    }

    pub async fn send(self, offer: EncryptedConnectionOffer) -> Result<(), AsRequestError> {
        self.tx.send(offer).map_err(|_| {
            error!("failed to send connection offer: connection closed");
            AsRequestError::UnexpectedResponse
        })?;
        self.response.await
    }
}
