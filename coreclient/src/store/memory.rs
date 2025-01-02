use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use openmls_rust_crypto::RustCrypto;
use phnxtypes::{identifiers::QualifiedUserName, time::TimeStamp};
use tokio::sync::{Mutex, MutexGuard};
use tokio_stream::Stream;
use uuid::Uuid;

use crate::{
    Contact, Conversation, ConversationAttributes, ConversationId, ConversationMessage,
    ConversationMessageId, MimiContent, PartialContact, UserProfile,
};

use super::{Store, StoreNotification, StoreNotificationsSender, StoreNotifier, StoreResult};

thread_local! {
    pub static RNG: RefCell<RustCrypto> = Default::default();
}

pub(crate) struct InMemoryStore {
    user_name: QualifiedUserName,
    inner: Arc<Mutex<InMemoryStoreInner>>,
    store_notifications_tx: StoreNotificationsSender,
}

#[derive(Default)]
struct InMemoryStoreInner {
    user_profile: Option<UserProfile>,
    conversations: Vec<Conversation>,
    members: HashMap<ConversationId, HashSet<QualifiedUserName>>,
    partial_contacts: HashMap<QualifiedUserName, PartialContact>,
    contacts: HashMap<QualifiedUserName, Contact>,
    user_profiles: HashMap<QualifiedUserName, UserProfile>,
    messages: HashMap<ConversationId, Vec<ConversationMessage>>,
}

impl InMemoryStore {
    pub(crate) fn new(user_name: QualifiedUserName) -> Self {
        Self {
            user_name,
            inner: Default::default(),
            store_notifications_tx: Default::default(),
        }
    }

    fn store_notifier(&self) -> StoreNotifier {
        StoreNotifier::new(self.store_notifications_tx.clone())
    }

    async fn lock(&self) -> MutexGuard<InMemoryStoreInner> {
        self.inner.lock().await
    }
}

impl Store for InMemoryStore {
    fn user_name(&self) -> QualifiedUserName {
        self.user_name.clone()
    }

    async fn own_user_profile(&self) -> StoreResult<UserProfile> {
        self.inner
            .lock()
            .await
            .user_profile
            .clone()
            .context("user profile not found")
    }

    async fn set_own_user_profile(&self, user_profile: UserProfile) -> StoreResult<()> {
        self.inner.lock().await.user_profile.replace(user_profile);
        Ok(())
    }

    async fn create_conversation(
        &self,
        title: &str,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<ConversationId> {
        let group_id = RNG.with_borrow_mut(|rng| GroupId::random(rng));
        let attributes = ConversationAttributes::new(title.to_string(), picture);
        let conversation = Conversation::new_group_conversation(group_id, attributes);
        let id = conversation.id();
        self.inner.lock().await.conversations.push(conversation);
        Ok(id)
    }

    async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<()> {
        let mut inner = self.inner.lock().await;
        let conversation = inner
            .conversations
            .iter_mut()
            .find(|c| c.id() == conversation_id)
            .context("conversation not found")?;
        conversation
            .attributes_mut()
            .set_conversation_picture_option(picture);
        Ok(())
    }

    async fn conversations(&self) -> StoreResult<Vec<Conversation>> {
        Ok(self.inner.lock().await.conversations.clone())
    }

    async fn conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<HashSet<QualifiedUserName>>> {
        Ok(self
            .inner
            .lock()
            .await
            .members
            .get(&conversation_id)
            .cloned())
    }

    async fn mark_conversation_as_read<I>(&self, until: I) -> StoreResult<()>
    where
        I: IntoIterator<Item = (ConversationId, DateTime<Utc>)> + Send,
        I::IntoIter: Send,
    {
        let mut inner = self.inner.lock().await;
        for (conversation_id, timestamp) in until {
            if let Some(conversation) = inner
                .conversations
                .iter_mut()
                .find(|c| c.id() == conversation_id)
            {
                conversation.set_last_read(timestamp);
            }
        }
        Ok(())
    }

    async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.inner
            .lock()
            .await
            .conversations
            .retain(|c| c.id() != conversation_id);
        Ok(vec![])
    }

    async fn leave_conversation(&self, _conversation_id: ConversationId) -> StoreResult<()> {
        Ok(())
    }

    async fn add_contact(&self, user_name: &QualifiedUserName) -> StoreResult<ConversationId> {
        let group_id = RNG.with_borrow_mut(|rng| GroupId::random(rng));
        let title = format!("Connection group: {} - {}", self.user_name(), user_name);
        let attributes = ConversationAttributes::new(title.to_string(), None);
        let conversation =
            Conversation::new_connection_conversation(group_id, user_name.clone(), attributes)?;
        let id = conversation.id();
        self.inner.lock().await.conversations.push(conversation);
        Ok(id)
    }

    async fn contacts(&self) -> StoreResult<Vec<Contact>> {
        Ok(self.inner.lock().await.contacts.values().cloned().collect())
    }

    async fn contact(&self, user_name: &QualifiedUserName) -> StoreResult<Option<Contact>> {
        Ok(self.inner.lock().await.contacts.get(user_name).cloned())
    }

    async fn partial_contacts(&self) -> StoreResult<Vec<PartialContact>> {
        Ok(self
            .inner
            .lock()
            .await
            .partial_contacts
            .values()
            .cloned()
            .collect())
    }

    async fn user_profile(
        &self,
        user_name: &QualifiedUserName,
    ) -> StoreResult<Option<UserProfile>> {
        Ok(self
            .inner
            .lock()
            .await
            .user_profiles
            .get(user_name)
            .cloned())
    }

    async fn messages(
        &self,
        conversation_id: ConversationId,
        limit: usize,
    ) -> StoreResult<Vec<ConversationMessage>> {
        let inner = self.inner.lock().await;
        let Some(messages) = inner.messages.get(&conversation_id) else {
            return Ok(Vec::new());
        };
        let offset = messages.len().saturating_sub(limit);
        Ok(messages[offset..].to_vec())
    }

    async fn message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>> {
        let inner = self.inner.lock().await;
        for messages in inner.messages.values() {
            if let Some(message) = messages.iter().find(|m| m.id() == message_id) {
                return Ok(Some(message.clone()));
            }
        }
        Ok(None)
    }

    async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<ConversationMessage>> {
        Ok(self
            .inner
            .lock()
            .await
            .messages
            .get(&conversation_id)
            .and_then(|messages| messages.last().cloned()))
    }

    async fn unread_messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize> {
        let inner = self.inner.lock().await;
        let Some(conversation) = inner
            .conversations
            .iter()
            .find(|c| c.id() == conversation_id)
        else {
            return Ok(0);
        };
        let Some(messages) = inner.messages.get(&conversation_id) else {
            return Ok(0);
        };

        let last_read = conversation.last_read();
        let count = messages
            .iter()
            .filter(|m| m.timestamp() > last_read)
            .count();

        Ok(count)
    }

    async fn global_unread_messages_count(&self) -> StoreResult<usize> {
        let inner = self.inner.lock().await;
        let mut count = 0;
        for conversation in inner.conversations.iter() {
            let Some(messages) = inner.messages.get(&conversation.id()) else {
                continue;
            };
            let last_read = conversation.last_read();
            count += messages
                .iter()
                .filter(|m| m.timestamp() > last_read)
                .count();
        }
        Ok(count)
    }

    async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> StoreResult<ConversationMessage> {
        let mut inner = self.inner.lock().await;
        let messages = inner
            .messages
            .get_mut(&conversation_id)
            .context("conversation not found")?;

        let mut message = ConversationMessage::new_unsent_message(
            self.user_name().to_string(),
            conversation_id,
            content,
        );
        let ds_timestamp = TimeStamp::now();
        message.timestamped_message_mut().mark_as_sent(ds_timestamp);
        messages.push(message.clone());

        Ok(message)
    }

    async fn resend_message(&self, _local_message_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> + Send + 'static {
        self.store_notifications_tx.subscribe()
    }
}
