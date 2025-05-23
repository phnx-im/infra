// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::auth_service::v1::{
    AckListenRequest, AsCredentialsRequest, DeleteUserPayload, EnqueueMessagesRequest,
    GetUserConnectionPackagesRequest, GetUserProfileRequest, InitListenPayload, ListenRequest,
    MergeUserProfilePayload, PublishConnectionPackagesPayload, RegisterUserRequest,
    StageUserProfilePayload, listen_request, publish_connection_packages_payload,
};
use phnxtypes::{
    LibraryError,
    credentials::{ClientCredentialPayload, keys::ClientSigningKey},
    crypto::{
        RatchetEncryptionKey, indexed_aead::keys::UserProfileKeyIndex, kdf::keys::RatchetSecret,
        signatures::signable::Signable,
    },
    identifiers::UserId,
    messages::{
        QueueMessage,
        client_as::{
            ConnectionPackage, EncryptedConnectionEstablishmentPackage,
            UserConnectionPackagesParams,
        },
        client_as_out::{
            AsCredentialsResponseIn, EncryptedUserProfile, GetUserProfileResponse,
            RegisterUserResponseIn, UserConnectionPackagesResponseIn,
        },
    },
};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::Request;
use tracing::error;

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
        connection_establishment_ctxt: EncryptedConnectionEstablishmentPackage,
    ) -> Result<(), AsRequestError> {
        let request = EnqueueMessagesRequest {
            user_id: Some(user_id.into()),
            connection_establishment_package: Some(connection_establishment_ctxt.into()),
        };
        self.as_grpc_client
            .client()
            .enqueue_messages(request)
            .await?;
        Ok(())
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
