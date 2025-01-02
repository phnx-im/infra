// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use phnxtypes::identifiers::QualifiedUserName;

use crate::clients::CoreUser;

use super::{Store, StoreResult};

impl Store for CoreUser {
    fn user_name(&self) -> QualifiedUserName {
        self.user_name()
    }

    async fn own_user_profile(&self) -> StoreResult<crate::UserProfile> {
        Ok(self.own_user_profile().await?)
    }

    async fn set_own_user_profile(&self, user_profile: crate::UserProfile) -> StoreResult<()> {
        self.set_own_user_profile(user_profile).await
    }

    async fn create_conversation(
        &self,
        title: &str,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<crate::ConversationId> {
        self.create_conversation(title, picture).await
    }

    async fn set_conversation_picture(
        &self,
        conversation_id: crate::ConversationId,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<()> {
        self.set_conversation_picture(conversation_id, picture)
            .await
    }

    async fn conversations(&self) -> StoreResult<Vec<crate::Conversation>> {
        Ok(self.conversations().await?)
    }

    async fn conversation_participants(
        &self,
        conversation_id: crate::ConversationId,
    ) -> StoreResult<Option<HashSet<QualifiedUserName>>> {
        self.try_conversation_participants(conversation_id).await
    }

    async fn delete_conversation(
        &self,
        conversation_id: crate::ConversationId,
    ) -> StoreResult<Vec<crate::ConversationMessage>> {
        self.delete_conversation(conversation_id).await
    }

    async fn leave_conversation(&self, conversation_id: crate::ConversationId) -> StoreResult<()> {
        self.leave_conversation(conversation_id).await
    }

    async fn add_contact(
        &self,
        user_name: &QualifiedUserName,
    ) -> StoreResult<crate::ConversationId> {
        self.add_contact(user_name.clone()).await
    }

    async fn contacts(&self) -> StoreResult<Vec<crate::Contact>> {
        Ok(self.contacts().await?)
    }

    async fn contact(&self, user_name: &QualifiedUserName) -> StoreResult<Option<crate::Contact>> {
        Ok(self.try_contact(user_name).await?)
    }

    async fn partial_contacts(&self) -> StoreResult<Vec<crate::PartialContact>> {
        Ok(self.partial_contacts().await?)
    }

    async fn user_profile(
        &self,
        user_name: &QualifiedUserName,
    ) -> StoreResult<Option<crate::UserProfile>> {
        self.user_profile(user_name).await
    }

    async fn messages(
        &self,
        conversation_id: crate::ConversationId,
        limit: usize,
    ) -> StoreResult<Vec<crate::ConversationMessage>> {
        self.get_messages(conversation_id, limit).await
    }

    async fn message(
        &self,
        message_id: crate::ConversationMessageId,
    ) -> StoreResult<Option<crate::ConversationMessage>> {
        Ok(self.message(message_id).await?)
    }

    async fn last_message(
        &self,
        conversation_id: crate::ConversationId,
    ) -> StoreResult<Option<crate::ConversationMessage>> {
        Ok(self.try_last_message(conversation_id).await?)
    }

    async fn unread_messages_count(
        &self,
        conversation_id: crate::ConversationId,
    ) -> StoreResult<usize> {
        Ok(self.try_unread_messages_count(conversation_id).await?)
    }

    async fn global_unread_messages_count(&self) -> StoreResult<usize> {
        let count = self.global_unread_messages_count().await?;
        Ok(usize::try_from(count).expect("usize overflow"))
    }

    async fn mark_conversation_as_read<I>(&self, until: I) -> StoreResult<()>
    where
        I: IntoIterator<Item = (crate::ConversationId, chrono::DateTime<chrono::Utc>)> + Send,
        I::IntoIter: Send,
    {
        Ok(self.mark_as_read(until).await?)
    }

    async fn send_message(
        &self,
        conversation_id: crate::ConversationId,
        content: crate::MimiContent,
    ) -> StoreResult<crate::ConversationMessage> {
        self.send_message(conversation_id, content).await
    }

    async fn resend_message(&self, local_message_id: uuid::Uuid) -> StoreResult<()> {
        self.re_send_message(local_message_id).await
    }

    fn subscribe(
        &self,
    ) -> impl tokio_stream::Stream<Item = std::sync::Arc<super::StoreNotification>> + Send + 'static
    {
        self.inner.store_notifications_tx.subscribe()
    }
}
