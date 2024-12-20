// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use phnxtypes::identifiers::QualifiedUserName;

use crate::clients::CoreUser;
use crate::ConversationMessageId;

use super::{Store, StoreEntityId, StoreNotification, StoreResult};

impl Store for CoreUser {
    fn user_name(&self) -> QualifiedUserName {
        self.user_name()
    }

    async fn own_user_profile(&self) -> StoreResult<crate::UserProfile> {
        Ok(self.own_user_profile().await?)
    }

    async fn set_own_user_profile(&self, user_profile: crate::UserProfile) -> StoreResult<()> {
        self.set_own_user_profile(user_profile).await?;
        self.notify(StoreEntityId::OwnUser.updated());
        Ok(())
    }

    async fn create_conversation(
        &self,
        title: &str,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<crate::ConversationId> {
        let id = self.create_conversation(title, picture).await?;
        self.notify(StoreEntityId::from(id).added());
        Ok(id)
    }

    async fn set_conversation_picture(
        &self,
        conversation_id: crate::ConversationId,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<()> {
        self.set_conversation_picture(conversation_id, picture)
            .await?;
        self.notify(StoreEntityId::Conversation(conversation_id).updated());
        Ok(())
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
        let messages = self.delete_conversation(conversation_id).await?;
        self.notify(StoreEntityId::from(conversation_id).removed());
        Ok(messages)
    }

    async fn leave_conversation(&self, conversation_id: crate::ConversationId) -> StoreResult<()> {
        self.leave_conversation(conversation_id).await?;
        self.notify(StoreEntityId::from(conversation_id).updated());
        Ok(())
    }

    async fn add_contact(
        &self,
        user_name: &QualifiedUserName,
    ) -> StoreResult<crate::ConversationId> {
        let id = self.add_contact(user_name.clone()).await?;
        self.notify(StoreEntityId::from(id).added());
        Ok(id)
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

    async fn mark_conversation_as_read(
        &self,
        until: impl IntoIterator<Item = (crate::ConversationId, chrono::DateTime<chrono::Utc>)>,
    ) -> StoreResult<()> {
        let conversation_ids = self.mark_as_read(until).await?;
        self.notify(StoreNotification::builder().update_many(conversation_ids));
        Ok(())
    }

    async fn send_message(
        &self,
        conversation_id: crate::ConversationId,
        content: crate::MimiContent,
    ) -> StoreResult<crate::ConversationMessage> {
        let message = self.send_message(conversation_id, content).await?;
        self.notify(
            StoreNotification::builder()
                .add(conversation_id)
                .add(message.id()),
        );
        Ok(message)
    }

    async fn resend_message(&self, local_message_id: uuid::Uuid) -> StoreResult<()> {
        self.re_send_message(local_message_id).await?;
        self.notify(
            StoreNotification::builder().update(ConversationMessageId::from_uuid(local_message_id)),
        );
        Ok(())
    }

    fn subcribe(
        &self,
    ) -> impl tokio_stream::Stream<Item = std::sync::Arc<super::StoreNotification>> {
        self.inner.connection.notifications_tx().subscribe()
    }
}
