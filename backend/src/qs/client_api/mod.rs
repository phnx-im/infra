// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    crypto::signatures::signable::Verifiable,
    messages::client_qs::{
        DequeueMessagesParams, DequeueMessagesResponse, QsProcessResponse, QsRequestParams,
        QsSender, VerifiableClientToQsMessage,
    },
    qs::errors::QsDequeueError,
};

use super::{errors::QsProcessError, storage_provider_trait::QsStorageProvider, Qs};

pub(crate) mod client_records;
pub(crate) mod key_packages;
pub(crate) mod user_records;

impl Qs {
    pub async fn process<S: QsStorageProvider>(
        storage_provider: &S,
        message: VerifiableClientToQsMessage,
    ) -> Result<QsProcessResponse, QsProcessError> {
        let request_params = match message.sender() {
            QsSender::User(user_id) => {
                let user = storage_provider
                    .load_user(&user_id)
                    .await
                    .ok_or(QsProcessError::AuthenticationError)?;
                let signature_public_key = user.auth_key;
                message
                    .verify(&signature_public_key)
                    .map_err(|_| QsProcessError::AuthenticationError)?
            }
            QsSender::Client(client_id) => {
                let client = storage_provider
                    .load_client(&client_id)
                    .await
                    .ok_or(QsProcessError::AuthenticationError)?;
                let signature_public_key = client.owner_signature_key;
                message.verify(&signature_public_key).unwrap()
            }
            QsSender::FriendshipToken(token) => message
                .verify_with_token(token)
                .map_err(|_| QsProcessError::AuthenticationError)?,
            QsSender::QsUserVerifyingKey(key) => message
                .verify(&key)
                .map_err(|_| QsProcessError::AuthenticationError)?,
            QsSender::Anonymous => QsRequestParams::VerifyingKey,
        };

        Ok(match request_params {
            QsRequestParams::CreateUser(params) => QsProcessResponse::CreateUser(
                Self::qs_create_user_record(storage_provider, params).await?,
            ),
            QsRequestParams::UpdateUser(params) => {
                Self::qs_update_user_record(storage_provider, params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::DeleteUser(params) => {
                Self::qs_delete_user_record(storage_provider, params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::CreateClient(params) => QsProcessResponse::CreateClient(
                Self::qs_create_client_record(storage_provider, params).await?,
            ),
            QsRequestParams::UpdateClient(params) => {
                Self::qs_update_client_record(storage_provider, params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::DeleteClient(params) => {
                Self::qs_delete_client_record(storage_provider, params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::PublishKeyPackages(params) => {
                Self::qs_publish_key_packages(storage_provider, params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::ClientKeyPackage(params) => QsProcessResponse::ClientKeyPackage(
                Self::qs_client_key_package(storage_provider, params).await?,
            ),
            QsRequestParams::KeyPackageBatch(params) => QsProcessResponse::KeyPackageBatch(
                Self::qs_key_package_batch(storage_provider, params).await?,
            ),
            QsRequestParams::DequeueMessages(params) => QsProcessResponse::DequeueMessages(
                Self::qs_dequeue_messages(storage_provider, params).await?,
            ),
            QsRequestParams::VerifyingKey => {
                QsProcessResponse::VerifyingKey(Self::qs_verifying_key(storage_provider).await?)
            }
        })
    }

    /// Retrieve messages the given number of messages, starting with
    /// `sequence_number_start` from the queue with the given id and delete any
    /// messages older than the given sequence number start.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_dequeue_messages<S: QsStorageProvider>(
        storage_provider: &S,
        params: DequeueMessagesParams,
    ) -> Result<DequeueMessagesResponse, QsDequeueError> {
        let DequeueMessagesParams {
            sender,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        tracing::trace!("Reading and deleting messages from storage provider");
        let (messages, remaining_messages_number) = storage_provider
            .read_and_delete(&sender, sequence_number_start, max_message_number)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsDequeueError::StorageError
            })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
