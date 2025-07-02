// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, path::Path, sync::Arc};

use mimi_room_policy::VerifiedRoomState;
use phnxcommon::identifiers::{AttachmentId, UserHandle, UserId};
use tokio_stream::Stream;
use uuid::Uuid;

use crate::{
    AttachmentContent, Contact, Conversation, ConversationId, ConversationMessage,
    ConversationMessageId, DownloadProgress, MessageDraft,
    clients::{CoreUser, attachment::AttachmentRecord},
    contacts::HandleContact,
    user_handles::UserHandleRecord,
    user_profiles::UserProfile,
};

use super::{Store, StoreNotification, StoreResult};

impl Store for CoreUser {
    fn user_id(&self) -> &UserId {
        self.user_id()
    }

    async fn own_user_profile(&self) -> StoreResult<UserProfile> {
        Ok(self.own_user_profile().await?)
    }

    async fn set_own_user_profile(&self, user_profile: UserProfile) -> StoreResult<UserProfile> {
        self.set_own_user_profile(user_profile).await
    }

    async fn user_handles(&self) -> StoreResult<Vec<UserHandle>> {
        Ok(UserHandleRecord::load_all_handles(self.pool()).await?)
    }

    async fn user_handle_records(&self) -> StoreResult<Vec<UserHandleRecord>> {
        Ok(UserHandleRecord::load_all(self.pool()).await?)
    }

    async fn add_user_handle(
        &self,
        user_handle: &UserHandle,
    ) -> StoreResult<Option<UserHandleRecord>> {
        self.add_user_handle(user_handle).await
    }

    async fn remove_user_handle(&self, user_handle: &UserHandle) -> StoreResult<()> {
        self.remove_user_handle(user_handle).await
    }

    async fn create_conversation(
        &self,
        title: String,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<ConversationId> {
        self.create_conversation(title, picture).await
    }

    async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> StoreResult<()> {
        self.set_conversation_picture(conversation_id, picture)
            .await
    }

    async fn conversations(&self) -> StoreResult<Vec<Conversation>> {
        Ok(self.conversations().await?)
    }

    async fn conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<HashSet<UserId>>> {
        self.try_conversation_participants(conversation_id).await
    }

    async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.delete_conversation(conversation_id).await
    }

    async fn leave_conversation(&self, conversation_id: ConversationId) -> StoreResult<()> {
        self.leave_conversation(conversation_id).await
    }

    async fn update_key(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.update_key(conversation_id).await
    }

    async fn remove_users(
        &self,
        conversation_id: ConversationId,
        target_users: Vec<UserId>,
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.remove_users(conversation_id, target_users).await
    }

    async fn invite_users(
        &self,
        conversation_id: ConversationId,
        invited_users: &[UserId],
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.invite_users(conversation_id, invited_users).await
    }

    async fn load_room_state(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<(UserId, VerifiedRoomState)> {
        self.load_room_state(&conversation_id).await
    }

    async fn add_contact(&self, handle: UserHandle) -> StoreResult<Option<ConversationId>> {
        self.add_contact_via_handle(handle).await
    }

    async fn contacts(&self) -> StoreResult<Vec<Contact>> {
        Ok(self.contacts().await?)
    }

    async fn contact(&self, user_id: &UserId) -> StoreResult<Option<Contact>> {
        Ok(self.try_contact(user_id).await?)
    }

    async fn handle_contacts(&self) -> StoreResult<Vec<HandleContact>> {
        Ok(self.handle_contacts().await?)
    }

    async fn user_profile(&self, user_id: &UserId) -> UserProfile {
        self.user_profile(user_id).await
    }

    async fn messages(
        &self,
        conversation_id: ConversationId,
        limit: usize,
    ) -> StoreResult<Vec<ConversationMessage>> {
        self.get_messages(conversation_id, limit).await
    }

    async fn message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>> {
        Ok(self.message(message_id).await?)
    }

    async fn prev_message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>> {
        self.prev_message(message_id).await
    }

    async fn next_message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>> {
        self.next_message(message_id).await
    }

    async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<ConversationMessage>> {
        Ok(self.try_last_message(conversation_id).await?)
    }

    async fn message_draft(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<MessageDraft>> {
        Ok(MessageDraft::load(self.pool(), conversation_id).await?)
    }

    async fn store_message_draft(
        &self,
        conversation_id: ConversationId,
        message_draft: Option<&MessageDraft>,
    ) -> StoreResult<()> {
        let mut notifier = self.store_notifier();
        if let Some(message_draft) = message_draft {
            message_draft
                .store(self.pool(), &mut notifier, conversation_id)
                .await?;
        } else {
            MessageDraft::delete(self.pool(), &mut notifier, conversation_id).await?;
        }
        notifier.notify();
        Ok(())
    }

    async fn messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize> {
        Ok(self.try_messages_count(conversation_id).await?)
    }

    async fn unread_messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize> {
        Ok(self.try_unread_messages_count(conversation_id).await?)
    }

    async fn global_unread_messages_count(&self) -> StoreResult<usize> {
        Ok(self.global_unread_messages_count().await?)
    }

    async fn mark_conversation_as_read(
        &self,
        conversation_id: ConversationId,
        until: ConversationMessageId,
    ) -> StoreResult<bool> {
        Ok(self
            .mark_conversation_as_read(conversation_id, until)
            .await?)
    }

    async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: mimi_content::MimiContent,
    ) -> StoreResult<ConversationMessage> {
        self.send_message(conversation_id, content).await
    }

    async fn upload_attachment(
        &self,
        conversation_id: ConversationId,
        path: &Path,
    ) -> StoreResult<ConversationMessage> {
        self.upload_attachment(conversation_id, path).await
    }

    fn download_attachment(
        &self,
        attachment_id: AttachmentId,
    ) -> (
        DownloadProgress,
        impl Future<Output = StoreResult<()>> + use<>,
    ) {
        self.download_attachment(attachment_id)
    }

    async fn pending_attachments(&self) -> StoreResult<Vec<AttachmentId>> {
        Ok(AttachmentRecord::load_all_pending(self.pool()).await?)
    }

    async fn load_attachment(&self, attachment_id: AttachmentId) -> StoreResult<AttachmentContent> {
        Ok(AttachmentRecord::load_content(self.pool(), attachment_id).await?)
    }

    async fn resend_message(&self, local_message_id: Uuid) -> StoreResult<()> {
        self.re_send_message(local_message_id).await
    }

    fn notify(&self, notification: StoreNotification) {
        self.send_store_notification(notification);
    }

    fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> + Send + 'static {
        self.subscribe_to_store_notifications()
    }

    fn subscribe_iter(&self) -> impl Iterator<Item = Arc<StoreNotification>> + Send + 'static {
        self.subscribe_iter_to_store_notifications()
    }

    async fn enqueue_notification(&self, notification: &StoreNotification) -> StoreResult<()> {
        self.enqueue_store_notification(notification).await
    }

    async fn dequeue_notification(&self) -> StoreResult<StoreNotification> {
        self.dequeue_store_notification().await
    }
}
