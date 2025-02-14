// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::signable::Verifiable,
    errors::qs::{QsDequeueError, QsProcessError},
    messages::client_qs::{
        DequeueMessagesParams, DequeueMessagesResponse, QsProcessResponse, QsRequestParams,
        QsSender, QsVersionedProcessResponse, VerifiableClientToQsMessage,
    },
};

use super::{client_record::QsClientRecord, queue::Queue, user_record::UserRecord, Qs};

mod api_migrations;
pub(crate) mod client_records;
pub(crate) mod key_packages;
pub(crate) mod user_records;

impl Qs {
    pub async fn process(
        &self,
        message: VerifiableClientToQsMessage,
    ) -> Result<QsVersionedProcessResponse, QsProcessError> {
        let request_params = match message.sender()? {
            QsSender::User(user_id) => {
                let Some(user) = UserRecord::load(&self.db_pool, &user_id)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Failed to load user record: {:?}", e);
                        QsProcessError::StorageError
                    })?
                else {
                    tracing::warn!("User not found: {:?}", user_id);
                    return Err(QsProcessError::AuthenticationError);
                };
                let signature_public_key = user.verifying_key;
                message.verify(&signature_public_key).map_err(|e| {
                    tracing::warn!("Failed to verify message: {:?}", e);
                    QsProcessError::AuthenticationError
                })?
            }
            QsSender::Client(client_id) => {
                let Some(client) = QsClientRecord::load(&self.db_pool, &client_id)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Failed to load client record: {:?}", e);
                        QsProcessError::StorageError
                    })?
                else {
                    tracing::warn!("Client not found: {:?}", client_id);
                    return Err(QsProcessError::AuthenticationError);
                };
                let signature_public_key = client.auth_key;
                message.verify(&signature_public_key).map_err(|e| {
                    tracing::warn!("Failed to verify message: {:?}", e);
                    QsProcessError::AuthenticationError
                })?
            }
            QsSender::FriendshipToken(token) => message.verify_with_token(token).map_err(|e| {
                tracing::warn!("Failed to verify message: {:?}", e);
                QsProcessError::AuthenticationError
            })?,
            QsSender::QsUserVerifyingKey(key) => message.verify(&key).map_err(|e| {
                tracing::warn!("Failed to verify message: {:?}", e);
                QsProcessError::AuthenticationError
            })?,
            QsSender::Anonymous => message.extract_without_verification().map_err(|e| {
                tracing::warn!("Failed to verify message: {:?}", e);
                QsProcessError::AuthenticationError
            })?,
        };

        let request_params = api_migrations::migrate_qs_request_params(request_params)?;

        let response = match request_params {
            QsRequestParams::CreateUser(params) => {
                QsProcessResponse::CreateUser(self.qs_create_user_record(params).await?)
            }
            QsRequestParams::UpdateUser(params) => {
                self.qs_update_user_record(params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::DeleteUser(params) => {
                self.qs_delete_user_record(params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::CreateClient(params) => {
                QsProcessResponse::CreateClient(self.qs_create_client_record(params).await?)
            }
            QsRequestParams::UpdateClient(params) => {
                self.qs_update_client_record(params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::DeleteClient(params) => {
                self.qs_delete_client_record(params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::PublishKeyPackages(params) => {
                self.qs_publish_key_packages(params).await?;
                QsProcessResponse::Ok
            }
            QsRequestParams::ClientKeyPackage(params) => {
                QsProcessResponse::ClientKeyPackage(self.qs_client_key_package(params).await?)
            }
            QsRequestParams::KeyPackage(params) => {
                QsProcessResponse::KeyPackage(self.qs_key_package(params).await?)
            }
            QsRequestParams::DequeueMessages(params) => {
                QsProcessResponse::DequeueMessages(self.qs_dequeue_messages(params).await?)
            }
            QsRequestParams::EncryptionKey => {
                QsProcessResponse::EncryptionKey(self.qs_encryption_key().await?)
            }
        };

        Ok(QsVersionedProcessResponse::Alpha(response))
    }

    /// Retrieve messages the given number of messages, starting with
    /// `sequence_number_start` from the queue with the given id and delete any
    /// messages older than the given sequence number start.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_dequeue_messages(
        &self,
        params: DequeueMessagesParams,
    ) -> Result<DequeueMessagesResponse, QsDequeueError> {
        let DequeueMessagesParams {
            sender,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsDequeueError::StorageError
        })?;
        let (messages, remaining_messages_number) = Queue::read_and_delete(
            &mut connection,
            &sender,
            sequence_number_start,
            max_message_number,
        )
        .await
        .map_err(|e| {
            tracing::warn!("Storage provider error: {:?}", e);
            QsDequeueError::StorageError
        })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
