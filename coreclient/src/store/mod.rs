// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use phnxtypes::identifiers::QualifiedUserName;
use tokio_stream::Stream;
use uuid::Uuid;

use crate::{
    Contact, Conversation, ConversationId, ConversationMessage, ConversationMessageId, MimiContent,
    PartialContact, UserProfile,
};

pub use notification::{StoreEntityId, StoreNotification};
pub(crate) use notification::{StoreNotificationsSender, StoreNotifier};

mod r#impl;
mod notification;

pub type StoreResult<T> = anyhow::Result<T>;

#[allow(async_fn_in_trait, reason = "trait is only used in the workspace")]
pub trait Store {
    // user

    fn user_name(&self) -> QualifiedUserName;

    async fn own_user_profile(&self) -> StoreResult<UserProfile>;

    async fn set_own_user_profile(&self, user_profile: UserProfile) -> StoreResult<()>;

    // conversations

    async fn create_conversation(
        &self,
        title: &str,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<ConversationId>;

    async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<()>;

    async fn conversations(&self) -> StoreResult<Vec<Conversation>>;

    async fn conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<HashSet<QualifiedUserName>>>;

    async fn mark_conversation_as_read(
        &self,
        until: impl IntoIterator<Item = (ConversationId, DateTime<Utc>)>,
    ) -> StoreResult<()>;

    async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>>;

    async fn leave_conversation(&self, conversation_id: ConversationId) -> StoreResult<()>;

    // contacts

    async fn add_contact(&self, user_name: &QualifiedUserName) -> StoreResult<ConversationId>;

    async fn contacts(&self) -> StoreResult<Vec<Contact>>;

    async fn contact(&self, user_name: &QualifiedUserName) -> StoreResult<Option<Contact>>;

    async fn partial_contacts(&self) -> StoreResult<Vec<PartialContact>>;

    async fn user_profile(&self, user_name: &QualifiedUserName)
        -> StoreResult<Option<UserProfile>>;

    // messages

    async fn messages(
        &self,
        conversation_id: ConversationId,
        limit: usize,
    ) -> StoreResult<Vec<ConversationMessage>>;

    async fn message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>>;

    async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<ConversationMessage>>;

    async fn unread_messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize>;

    async fn global_unread_messages_count(&self) -> StoreResult<usize>;

    async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> StoreResult<ConversationMessage>;

    async fn resend_message(&self, local_message_id: Uuid) -> StoreResult<()>;

    // observability

    fn subcribe(&self) -> impl Stream<Item = Arc<StoreNotification>>;
}