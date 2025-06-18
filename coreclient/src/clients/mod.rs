// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Duration, Utc};
use exif::{Reader, Tag};
use openmls::prelude::Ciphersuite;
use own_client_info::OwnClientInfo;
pub use phnxapiclient::as_api::ListenHandleResponder;
use phnxapiclient::{ApiClient, ApiClientInitError};
use phnxcommon::{
    DEFAULT_PORT_GRPC,
    credentials::{
        ClientCredential, ClientCredentialCsr, ClientCredentialPayload,
        keys::{ClientSigningKey, HandleSigningKey},
    },
    crypto::{
        ConnectionDecryptionKey, RatchetDecryptionKey,
        ear::{
            EarEncryptable,
            keys::{PushTokenEarKey, WelcomeAttributionInfoEarKey},
        },
        hpke::HpkeEncryptable,
        kdf::keys::RatchetSecret,
        signatures::keys::{QsClientSigningKey, QsUserSigningKey},
    },
    identifiers::{ClientConfig, QsClientId, QsReference, QsUserId, UserHandleHash, UserId},
    messages::{FriendshipToken, QueueMessage, push_token::PushToken},
};
pub use phnxprotos::auth_service::v1::{HandleQueueMessage, handle_queue_message};
pub use phnxprotos::queue_service::v1::{
    QueueEvent, QueueEventPayload, QueueEventUpdate, queue_event,
};

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use store::ClientRecord;
use thiserror::Error;
use tokio_stream::{Stream, StreamExt};
use tracing::{error, info};
use url::Url;

use crate::{
    Asset, contacts::HandleContact, groups::Group, store::Store,
    utils::persistence::delete_client_database,
};
use crate::{ConversationId, key_stores::as_credentials::AsCredentials};
use crate::{
    ConversationMessageId,
    clients::connection_offer::FriendshipPackage,
    contacts::Contact,
    conversations::{
        Conversation, ConversationAttributes,
        messages::{ConversationMessage, TimestampedMessage},
    },
    groups::openmls_provider::PhnxOpenMlsProvider,
    key_stores::{MemoryUserKeyStore, queue_ratchets::QueueType},
    store::{StoreNotification, StoreNotifier},
    user_profiles::IndexedUserProfile,
    utils::persistence::{open_client_db, open_db_in_memory, open_phnx_db},
};
use crate::{store::StoreNotificationsSender, user_profiles::UserProfile};

use self::{api_clients::ApiClients, create_user::InitialUserState, store::UserCreationState};

mod add_contact;
pub(crate) mod api_clients;
pub(crate) mod connection_offer;
pub mod conversations;
mod create_user;
mod invite_users;
mod message;
pub(crate) mod own_client_info;
mod persistence;
pub mod process;
mod remove_users;
pub mod store;
#[cfg(test)]
mod tests;
mod update_key;
mod user_profile;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const KEY_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION: Duration = Duration::days(30);

#[derive(Debug, Clone)]
pub struct CoreUser {
    inner: Arc<CoreUserInner>,
}

#[derive(Debug)]
struct CoreUserInner {
    pool: SqlitePool,
    api_clients: ApiClients,
    _qs_user_id: QsUserId,
    qs_client_id: QsClientId,
    key_store: MemoryUserKeyStore,
    store_notifications_tx: StoreNotificationsSender,
}

impl CoreUser {
    /// Create a new user with the given `user_id`.
    ///
    /// If a user with this name already exists, this will overwrite that user.
    pub async fn new(
        user_id: UserId,
        server_url: Url,
        grpc_port: u16,
        db_path: &str,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        info!(?user_id, "creating new user");

        // Open the phnx db to store the client record
        let phnx_db = open_phnx_db(db_path).await?;

        // Open client specific db
        let client_db = open_client_db(&user_id, db_path).await?;

        Self::new_with_connections(
            user_id, server_url, grpc_port, push_token, phnx_db, client_db,
        )
        .await
    }

    async fn new_with_connections(
        user_id: UserId,
        server_url: Url,
        grpc_port: u16,
        push_token: Option<PushToken>,
        phnx_db: SqlitePool,
        client_db: SqlitePool,
    ) -> Result<Self> {
        let server_url = server_url.to_string();
        let api_clients = ApiClients::new(user_id.domain().clone(), server_url.clone(), grpc_port);

        let user_creation_state = UserCreationState::new(
            &client_db,
            &phnx_db,
            user_id,
            server_url.clone(),
            push_token,
        )
        .await?;

        let final_state = user_creation_state
            .complete_user_creation(&phnx_db, &client_db, &api_clients)
            .await?;

        OwnClientInfo {
            server_url,
            qs_user_id: *final_state.qs_user_id(),
            qs_client_id: *final_state.qs_client_id(),
            user_id: final_state.user_id().clone(),
        }
        .store(&client_db)
        .await?;

        let self_user = final_state.into_self_user(client_db, api_clients);

        Ok(self_user)
    }

    /// The same as [`Self::new()`], except that databases are ephemeral and are
    /// dropped together with this instance of [`CoreUser`].
    pub async fn new_ephemeral(
        user_id: UserId,
        server_url: Url,
        grpc_port: u16,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        info!(?user_id, "creating new ephemeral user");

        // Open the phnx db to store the client record
        let phnx_db = open_db_in_memory().await?;

        // Open client specific db
        let client_db = open_db_in_memory().await?;

        Self::new_with_connections(
            user_id, server_url, grpc_port, push_token, phnx_db, client_db,
        )
        .await
    }

    /// Load a user from the database.
    ///
    /// If a user creation process with a matching `UserId` was interrupted before, this will
    /// resume that process.
    pub async fn load(user_id: UserId, db_path: &str) -> Result<CoreUser> {
        let client_db = open_client_db(&user_id, db_path).await?;

        let user_creation_state = UserCreationState::load(&client_db, &user_id)
            .await?
            .context("missing user creation state")?;

        let phnx_db = open_phnx_db(db_path).await?;
        let api_clients = ApiClients::new(
            user_id.domain().clone(),
            user_creation_state.server_url(),
            DEFAULT_PORT_GRPC,
        );
        let final_state = user_creation_state
            .complete_user_creation(&phnx_db, &client_db, &api_clients)
            .await?;
        ClientRecord::set_default(&phnx_db, &user_id).await?;

        Ok(final_state.into_self_user(client_db, api_clients))
    }

    /// Delete this user on the server and locally.
    ///
    /// The user database is also deleted. The client record is removed from the phnx database.
    pub async fn delete(self, db_path: &str) -> anyhow::Result<()> {
        let user_id = self.user_id().clone();
        self.delete_ephemeral().await?;
        delete_client_database(db_path, &user_id).await?;
        Ok(())
    }

    /// Delete this user on the server.
    ///
    /// The local database and client record are not touched.
    pub async fn delete_ephemeral(self) -> anyhow::Result<()> {
        self.inner
            .api_clients
            .default_client()?
            .as_delete_user(self.user_id().clone(), &self.inner.key_store.signing_key)
            .await?;
        Ok(())
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.inner.pool
    }

    pub(crate) fn signing_key(&self) -> &ClientSigningKey {
        &self.inner.key_store.signing_key
    }

    pub(crate) fn api_client(&self) -> anyhow::Result<ApiClient> {
        Ok(self.inner.api_clients.default_client()?)
    }

    pub(crate) fn key_store(&self) -> &MemoryUserKeyStore {
        &self.inner.key_store
    }

    pub(crate) fn send_store_notification(&self, notification: StoreNotification) {
        if !notification.is_empty() {
            self.inner.store_notifications_tx.notify(notification);
        }
    }

    /// Subscribes to store notifications.
    ///
    /// All notifications sent after this function was called are observed as items of the returned
    /// stream.
    pub(crate) fn subscribe_to_store_notifications(
        &self,
    ) -> impl Stream<Item = Arc<StoreNotification>> + Send + 'static {
        self.inner.store_notifications_tx.subscribe()
    }

    /// Subcribes to pending store notifications.
    ///
    /// Unlike `subscribe_to_store_notifications`, this function does not remove stored
    /// notifications from the persisted queue.
    pub(crate) fn subscribe_iter_to_store_notifications(
        &self,
    ) -> impl Iterator<Item = Arc<StoreNotification>> + Send + 'static {
        self.inner.store_notifications_tx.subscribe_iter()
    }

    pub(crate) fn store_notifier(&self) -> StoreNotifier {
        StoreNotifier::new(self.inner.store_notifications_tx.clone())
    }

    pub(crate) async fn enqueue_store_notification(
        &self,
        notification: &StoreNotification,
    ) -> Result<()> {
        notification
            .enqueue(self.pool().acquire().await?.as_mut())
            .await?;
        Ok(())
    }

    pub(crate) async fn dequeue_store_notification(&self) -> Result<StoreNotification> {
        Ok(StoreNotification::dequeue(self.pool()).await?)
    }

    pub async fn set_own_user_profile(&self, mut user_profile: UserProfile) -> Result<UserProfile> {
        if &user_profile.user_id != self.user_id() {
            bail!("Can't set user profile for users other than the current user.",);
        }
        if let Some(profile_picture) = user_profile.profile_picture {
            let new_image = match profile_picture {
                Asset::Value(image_bytes) => self.resize_image(&image_bytes)?,
            };
            user_profile.profile_picture = Some(Asset::Value(new_image));
        }
        self.update_user_profile(user_profile.clone()).await?;
        Ok(user_profile)
    }

    fn resize_image(&self, mut image_bytes: &[u8]) -> Result<Vec<u8>> {
        let image = image::load_from_memory(image_bytes)?;

        // Read EXIF data
        let exif_reader = Reader::new();
        let mut image_bytes_cursor = std::io::Cursor::new(&mut image_bytes);
        let exif = exif_reader
            .read_from_container(&mut image_bytes_cursor)
            .ok();

        // Resize the image
        let image = image.resize(256, 256, image::imageops::FilterType::Nearest);

        // Rotate/flip the image according to the orientation if necessary
        let image = if let Some(exif) = exif {
            let orientation = exif
                .get_field(Tag::Orientation, exif::In::PRIMARY)
                .and_then(|field| field.value.get_uint(0))
                .unwrap_or(1);
            match orientation {
                1 => image,
                2 => image.fliph(),
                3 => image.rotate180(),
                4 => image.flipv(),
                5 => image.rotate90().fliph(),
                6 => image.rotate90(),
                7 => image.rotate270().fliph(),
                8 => image.rotate270(),
                _ => image,
            }
        } else {
            image
        };

        // Save the resized image
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90);
        encoder.encode_image(&image)?;
        info!(
            from_bytes = image_bytes.len(),
            to_bytes = buf.len(),
            "Resized profile picture",
        );
        Ok(buf)
    }

    /// Get the user profile of the user with the given [`AsClientId`].
    ///
    /// In case of an error, or if the user profile is not found, the client id is used as a
    /// fallback.
    pub async fn user_profile(&self, user_id: &UserId) -> UserProfile {
        match self.pool().acquire().await {
            Ok(mut connection) => self.user_profile_internal(&mut connection, user_id).await,
            Err(error) => {
                error!(%error, "Error loading user profile; fallback to user_id");
                UserProfile::from_user_id(user_id)
            }
        }
    }

    // Helper to use when we already hold a connection
    async fn user_profile_internal(
        &self,
        connection: &mut SqliteConnection,
        user_id: &UserId,
    ) -> UserProfile {
        IndexedUserProfile::load(connection, user_id)
            .await
            .inspect_err(|error| {
                error!(%error, "Error loading user profile; fallback to user_id");
            })
            .ok()
            .flatten()
            .map(UserProfile::from)
            .unwrap_or_else(|| UserProfile::from_user_id(user_id))
    }

    async fn fetch_messages_from_queue(&self, queue_type: QueueType) -> Result<Vec<QueueMessage>> {
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        let mut sequence_number = queue_type.load_sequence_number(self.pool()).await?;

        while remaining_messages > 0 {
            let api_client = self.inner.api_clients.default_client()?;
            let mut response = match &queue_type {
                QueueType::Qs => {
                    api_client
                        .qs_dequeue_messages(
                            &self.inner.qs_client_id,
                            sequence_number,
                            1_000_000,
                            &self.inner.key_store.qs_client_signing_key,
                        )
                        .await?
                }
            };

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);

            if let Some(message) = messages.last() {
                sequence_number = message.sequence_number + 1;
                queue_type
                    .update_sequence_number(self.pool(), sequence_number)
                    .await?;
            }
        }
        Ok(messages)
    }

    /// Fetch and process AS messages
    ///
    /// Returns the list of [`ConversationId`]s of any newly created conversations.
    pub async fn fetch_and_process_as_messages(&self) -> Result<Vec<ConversationId>> {
        let records = self.user_handle_records().await?;
        let api_client = self.api_client()?;
        let mut conversation_ids = Vec::new();
        for record in records {
            let (mut stream, responder) = api_client
                .as_listen_handle(record.hash, &record.signing_key)
                .await?;
            while let Some(Some(message)) = stream.next().await {
                let Some(message_id) = message.message_id else {
                    error!("no message id in handle queue message");
                    continue;
                };
                match self
                    .process_handle_queue_message(&record.handle, message)
                    .await
                {
                    Ok(conversation_id) => {
                        conversation_ids.push(conversation_id);
                    }
                    Err(error) => {
                        error!(%error, "failed to process handle queue message");
                    }
                }
                // ack the message independently of the result of processing the message
                responder.ack(message_id.into()).await;
            }
        }
        Ok(conversation_ids)
    }

    pub async fn qs_fetch_messages(&self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::Qs).await
    }

    pub async fn contacts(&self) -> sqlx::Result<Vec<Contact>> {
        let contacts = Contact::load_all(self.pool()).await?;
        Ok(contacts)
    }

    pub async fn contact(&self, user_id: &UserId) -> Option<Contact> {
        self.try_contact(user_id).await.ok().flatten()
    }

    pub async fn try_contact(&self, user_id: &UserId) -> sqlx::Result<Option<Contact>> {
        Contact::load(self.pool(), user_id).await
    }

    pub async fn handle_contacts(&self) -> sqlx::Result<Vec<HandleContact>> {
        HandleContact::load_all(self.pool()).await
    }

    fn create_own_client_reference(&self) -> QsReference {
        let sealed_reference = ClientConfig {
            client_id: self.inner.qs_client_id,
            push_token_ear_key: Some(self.inner.key_store.push_token_ear_key.clone()),
        }
        .encrypt(&self.inner.key_store.qs_client_id_encryption_key, &[], &[]);
        QsReference {
            client_homeserver_domain: self.user_id().domain().clone(),
            sealed_reference,
        }
    }

    /// Returns None if there is no conversation with the given id.
    pub async fn conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> Option<HashSet<UserId>> {
        self.try_conversation_participants(conversation_id)
            .await
            .ok()?
    }

    pub(crate) async fn try_conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<HashSet<UserId>>> {
        let mut connection = self.pool().acquire().await?;
        let Some(conversation) = Conversation::load(&mut connection, &conversation_id).await?
        else {
            return Ok(None);
        };
        let Some(group) = Group::load(&mut connection, conversation.group_id()).await? else {
            return Ok(None);
        };
        Ok(Some(group.members(&mut *connection).await))
    }

    pub async fn pending_removes(&self, conversation_id: ConversationId) -> Option<Vec<UserId>> {
        let mut connection = self.pool().acquire().await.ok()?;
        let conversation = Conversation::load(&mut connection, &conversation_id)
            .await
            .ok()??;
        let group = Group::load(&mut connection, conversation.group_id())
            .await
            .ok()??;
        Some(group.pending_removes(&mut connection).await)
    }

    pub async fn listen_queue(&self) -> Result<impl Stream<Item = QueueEvent> + use<>> {
        let api_client = self.inner.api_clients.default_client()?;
        Ok(api_client.listen_queue(self.inner.qs_client_id).await?)
    }

    pub async fn listen_handle(
        &self,
        hash: UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> Result<(
        impl Stream<Item = Option<HandleQueueMessage>> + use<>,
        ListenHandleResponder,
    )> {
        let api_client = self.inner.api_clients.default_client()?;
        Ok(api_client.as_listen_handle(hash, signing_key).await?)
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub async fn mark_as_read<T: IntoIterator<Item = (ConversationId, DateTime<Utc>)>>(
        &self,
        mark_as_read_data: T,
    ) -> anyhow::Result<()> {
        let mut notifier = self.store_notifier();
        Conversation::mark_as_read(
            self.pool().acquire().await?.as_mut(),
            &mut notifier,
            mark_as_read_data,
        )
        .await?;
        notifier.notify();
        Ok(())
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub async fn mark_conversation_as_read(
        &self,
        conversation_id: ConversationId,
        until: ConversationMessageId,
    ) -> sqlx::Result<bool> {
        let mut notifier = self.store_notifier();
        let marked_as_read = Conversation::mark_as_read_until_message_id(
            self.pool().acquire().await?.as_mut(),
            &mut notifier,
            conversation_id,
            until,
        )
        .await?;
        notifier.notify();
        Ok(marked_as_read)
    }

    /// Returns how many messages are marked as unread across all conversations.
    pub async fn global_unread_messages_count(&self) -> sqlx::Result<usize> {
        Conversation::global_unread_message_count(self.pool()).await
    }

    /// Returns how many messages in the conversation with the given ID are
    /// marked as unread.
    pub async fn unread_messages_count(&self, conversation_id: ConversationId) -> usize {
        Conversation::unread_messages_count(self.pool(), conversation_id)
            .await
            .inspect_err(|error| error!(%error, "Error while fetching unread messages count"))
            .unwrap_or(0)
    }

    pub(crate) async fn try_messages_count(
        &self,
        conversation_id: ConversationId,
    ) -> sqlx::Result<usize> {
        Conversation::messages_count(self.pool(), conversation_id).await
    }

    pub(crate) async fn try_unread_messages_count(
        &self,
        conversation_id: ConversationId,
    ) -> sqlx::Result<usize> {
        Conversation::unread_messages_count(self.pool(), conversation_id).await
    }

    /// Updates the client's push token on the QS.
    pub async fn update_push_token(&self, push_token: Option<PushToken>) -> Result<()> {
        let client_id = self.inner.qs_client_id;
        // Ratchet encryption key
        let queue_encryption_key = self
            .inner
            .key_store
            .qs_queue_decryption_key
            .encryption_key();
        // Signung key
        let signing_key = self.inner.key_store.qs_client_signing_key.clone();

        // Encrypt the push token, if there is one.
        let encrypted_push_token = match push_token {
            Some(push_token) => {
                let encrypted_push_token =
                    push_token.encrypt(&self.inner.key_store.push_token_ear_key)?;
                Some(encrypted_push_token)
            }
            None => None,
        };

        self.inner
            .api_clients
            .default_client()?
            .qs_update_client(
                client_id,
                queue_encryption_key.clone(),
                encrypted_push_token,
                &signing_key,
            )
            .await?;
        Ok(())
    }

    pub fn user_id(&self) -> &UserId {
        self.inner.key_store.signing_key.credential().identity()
    }

    async fn store_messages(
        connection: &mut sqlx::SqliteConnection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        group_messages: Vec<TimestampedMessage>,
    ) -> Result<Vec<ConversationMessage>> {
        let mut stored_messages = Vec::with_capacity(group_messages.len());
        for timestamped_message in group_messages.into_iter() {
            let message =
                ConversationMessage::from_timestamped_message(conversation_id, timestamped_message);
            message.store(&mut *connection, notifier).await?;
            stored_messages.push(message);
        }
        Ok(stored_messages)
    }

    /// Returns the user profile of this [`CoreUser`].
    pub async fn own_user_profile(&self) -> sqlx::Result<UserProfile> {
        IndexedUserProfile::load(self.pool(), self.user_id())
            .await
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap().into())
    }

    /// Executes a function with a transaction.
    ///
    /// The transaction is committed if the function returns `Ok`, and rolled
    /// back if the function returns `Err`.
    pub(crate) async fn with_transaction<T: Send>(
        &self,
        f: impl AsyncFnOnce(&mut sqlx::SqliteTransaction<'_>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut txn = self.pool().begin_with("BEGIN IMMEDIATE").await?;
        let value = f(&mut txn).await?;
        txn.commit().await?;
        Ok(value)
    }

    /// Executes a function with a transaction and a [`StoreNotifier`].
    ///
    /// The transaction is committed if the function returns `Ok`, and rolled
    /// back if the function returns `Err`. The [`StoreNotifier`] is notified
    /// after the transaction is committed successfully.
    pub(crate) async fn with_transaction_and_notifier<T: Send>(
        &self,
        f: impl AsyncFnOnce(&mut sqlx::SqliteTransaction<'_>, &mut StoreNotifier) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut txn = self.pool().begin_with("BEGIN IMMEDIATE").await?;
        let mut notifier = self.store_notifier();
        let value = f(&mut txn, &mut notifier).await?;
        txn.commit().await?;
        notifier.notify();
        Ok(value)
    }

    pub(crate) async fn with_notifier<T: Send>(
        &self,
        f: impl AsyncFnOnce(&mut StoreNotifier) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut notifier = self.store_notifier();
        let value = f(&mut notifier).await?;
        notifier.notify();
        Ok(value)
    }
}
