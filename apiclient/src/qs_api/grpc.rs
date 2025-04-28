// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::queue_service::v1::{
    CreateUserRequest, ListenRequest, QueueEvent, queue_service_client::QueueServiceClient,
};
use phnxtypes::{
    crypto::{
        RatchetEncryptionKey,
        kdf::keys::RatchetSecret,
        signatures::keys::{QsClientVerifyingKey, QsUserSigningKey},
    },
    identifiers::QsClientId,
    messages::{
        FriendshipToken, client_qs::CreateUserRecordResponse, push_token::EncryptedPushToken,
    },
};
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;
use tracing::error;

use super::QsRequestError;

#[derive(Debug, Clone)]
pub(crate) struct QsGrpcClient {
    client: QueueServiceClient<Channel>,
}

impl QsGrpcClient {
    pub(crate) fn new(client: QueueServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn create_user(
        &self,
        friendship_token: FriendshipToken,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
        initial_ratchet_key: RatchetSecret,
        signing_key: &QsUserSigningKey,
    ) -> Result<CreateUserRecordResponse, QsRequestError> {
        let request = CreateUserRequest {
            user_record_auth_key: Some(signing_key.verifying_key().clone().into()),
            friendship_token: Some(friendship_token.into()),
            client_record_auth_key: Some(client_record_auth_key.into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            encrypted_push_token: encrypted_push_token.map(From::from),
            initial_ratched_secret: Some(initial_ratchet_key.into()),
        };
        let response = self.client.clone().create_user(request).await?.into_inner();
        Ok(CreateUserRecordResponse {
            user_id: response
                .user_id
                .ok_or_else(|| {
                    error!("missing user_id in response");
                    QsRequestError::UnexpectedResponse
                })?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid user_id in response");
                    QsRequestError::UnexpectedResponse
                })?,
            client_id: response
                .client_id
                .ok_or_else(|| {
                    error!("missing client_id in response");
                    QsRequestError::UnexpectedResponse
                })?
                .try_into()
                .map_err(|error| {
                    error!(%error, "invalid client_id in response");
                    QsRequestError::UnexpectedResponse
                })?,
        })
    }

    pub(crate) async fn listen(
        &self,
        queue_id: QsClientId,
    ) -> Result<impl Stream<Item = QueueEvent> + use<>, QsRequestError> {
        let request = ListenRequest {
            client_id: Some(queue_id.into()),
        };
        let response = self.client.clone().listen(request).await?;
        let stream = response.into_inner().map_while(|response| {
            response
                .inspect_err(|status| error!(?status, "terminating listen stream due to an error"))
                .ok()
        });
        Ok(stream)
    }
}
