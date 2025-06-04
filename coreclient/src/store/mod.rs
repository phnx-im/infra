// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Arc;

use mimi_room_policy::VerifiedRoomState;
use phnxcommon::identifiers::{UserHandle, UserId};
use tokio_stream::Stream;
use uuid::Uuid;

use crate::{
    Contact, Conversation, ConversationId, ConversationMessage, ConversationMessageId,
    PartialContact, user_profiles::UserProfile,
};

pub use notification::{StoreEntityId, StoreNotification, StoreOperation};
pub(crate) use notification::{StoreNotificationsSender, StoreNotifier};

mod r#impl;
mod notification;
mod persistence;

/// The result type of a failable [`Store`] method
pub type StoreResult<T> = anyhow::Result<T>;

/// Unified access to the client data
///
/// This trait is used to access the client data, e.g. the user profile, the conversations or
/// the messages. Additionaly, it is used to listen to changes in the client data via the
/// [`Self::subscribe`] method and the [`StoreNotification`] type.
#[allow(async_fn_in_trait, reason = "trait is only used in the workspace")]
#[trait_variant::make(Send)]
pub trait Store {
    // user

    async fn own_user_profile(&self) -> StoreResult<UserProfile>;

    async fn set_own_user_profile(&self, user_profile: UserProfile) -> StoreResult<UserProfile>;

    // user handles

    async fn user_handles(&self) -> StoreResult<Vec<UserHandle>>;

    async fn add_user_handle(&self, user_handle: &UserHandle) -> StoreResult<bool>;

    async fn remove_user_handle(&self, user_handle: &UserHandle) -> StoreResult<()>;

    // conversations

    /// Create new conversation.
    ///
    /// Returns the id of the newly created conversation.
    async fn create_conversation(
        &self,
        title: String,
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
    ) -> StoreResult<Option<HashSet<UserId>>>;

    async fn mark_conversation_as_read(
        &self,
        conversation_id: ConversationId,
        until: ConversationMessageId,
    ) -> StoreResult<bool>;

    /// Delete the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>>;

    async fn leave_conversation(&self, conversation_id: ConversationId) -> StoreResult<()>;

    // user management

    /// Update the user's key material in the conversation with the given
    /// [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    async fn update_key(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Vec<ConversationMessage>>;

    /// Remove users from the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    async fn remove_users(
        &self,
        conversation_id: ConversationId,
        target_users: Vec<UserId>,
    ) -> StoreResult<Vec<ConversationMessage>>;

    /// Invite users to an existing conversation.
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    async fn invite_users(
        &self,
        conversation_id: ConversationId,
        invited_users: &[UserId],
    ) -> StoreResult<Vec<ConversationMessage>>;

    async fn load_room_state(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<(UserId, VerifiedRoomState)>;

    // contacts

    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    async fn add_contact(&self, user_id: UserId) -> StoreResult<ConversationId>;

    async fn contacts(&self) -> StoreResult<Vec<Contact>>;

    async fn contact(&self, user_id: &UserId) -> StoreResult<Option<Contact>>;

    async fn partial_contacts(&self) -> StoreResult<Vec<PartialContact>>;

    async fn user_profile(&self, user_id: &UserId) -> UserProfile;

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

    async fn prev_message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>>;

    async fn next_message(
        &self,
        message_id: ConversationMessageId,
    ) -> StoreResult<Option<ConversationMessage>>;

    async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> StoreResult<Option<ConversationMessage>>;

    async fn messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize>;

    async fn unread_messages_count(&self, conversation_id: ConversationId) -> StoreResult<usize>;

    async fn global_unread_messages_count(&self) -> StoreResult<usize>;

    async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: mimi_content::MimiContent,
    ) -> StoreResult<ConversationMessage>;

    async fn resend_message(&self, local_message_id: Uuid) -> StoreResult<()>;

    // observability

    fn notify(&self, notification: StoreNotification);

    fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> + Send + 'static;

    fn subscribe_iter(&self) -> impl Iterator<Item = Arc<StoreNotification>> + Send + 'static;

    async fn enqueue_notification(&self, notification: &StoreNotification) -> StoreResult<()>;

    async fn dequeue_notification(&self) -> StoreResult<StoreNotification>;
}
