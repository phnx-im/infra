// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use phnxcoreclient::{clients::CoreUser, Conversation, ConversationId};
use phnxtypes::identifiers::{QualifiedUserName, SafeTryInto};

use crate::notifier::dispatch_message_notifications;

use super::{
    types::{UiContact, UiConversation, UiConversationDetails, UiConversationMessage},
    user::User,
};

impl User {
    pub async fn get_conversations(&self) -> Vec<UiConversation> {
        self.user
            .conversations()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    pub async fn get_conversation_details(&self) -> Vec<UiConversationDetails> {
        let conversations = self.user.conversations().await.unwrap_or_default();
        let mut conversation_details = Vec::with_capacity(conversations.len());
        for conversation in conversations {
            let details = converation_into_ui_details(&self.user, conversation).await;
            conversation_details.push(details);
        }
        // Sort the conversations by last used timestamp in descending order
        conversation_details.sort_unstable_by(|a, b| b.last_used.cmp(&a.last_used));
        conversation_details
    }

    pub async fn create_conversation(&self, name: String) -> Result<ConversationId> {
        self.user.create_conversation(&name, None).await
    }

    pub async fn add_users_to_conversation(
        &self,
        conversation_id: ConversationId,
        user_names: Vec<String>,
    ) -> Result<()> {
        let conversation_messages = self
            .user
            .invite_users(
                conversation_id,
                &user_names
                    .into_iter()
                    .map(<String as SafeTryInto<QualifiedUserName>>::try_into)
                    .collect::<Result<Vec<QualifiedUserName>, _>>()?,
            )
            .await?;
        dispatch_message_notifications(&self.notification_hub, conversation_messages).await;
        Ok(())
    }

    pub async fn remove_users_from_conversation(
        &self,
        conversation_id: ConversationId,
        user_names: Vec<String>,
    ) -> Result<()> {
        let conversation_messages = self
            .user
            .remove_users(
                conversation_id,
                &user_names
                    .into_iter()
                    .map(<String as SafeTryInto<QualifiedUserName>>::try_into)
                    .collect::<Result<Vec<QualifiedUserName>, _>>()?,
            )
            .await?;
        dispatch_message_notifications(&self.notification_hub, conversation_messages).await;
        Ok(())
    }

    /// Get a list of contacts to be added to the conversation with the given
    /// [`phnxcoreclient::ConversationId`].
    pub async fn member_candidates(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<UiContact>> {
        let group_members = self
            .user
            .conversation_participants(conversation_id)
            .await
            .ok_or(anyhow!("Conversation not found"))?;
        let add_candidates = self
            .user
            .contacts()
            .await?
            .into_iter()
            .filter_map(|c| {
                if !group_members.contains(&c.user_name) {
                    Some(c.into())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(add_candidates)
    }
}

/// Loads additional details for a conversation and converts it into a
/// [`UiConversationDetails`]
pub(crate) async fn converation_into_ui_details(
    user: &CoreUser,
    conversation: Conversation,
) -> UiConversationDetails {
    let unread_messages = user.unread_messages_count(conversation.id()).await;
    let last_message = user.last_message(conversation.id()).await.map(|m| m.into());
    let last_used = last_message
        .as_ref()
        .map(|m: &UiConversationMessage| m.timestamp.clone())
        .unwrap_or_default();
    // default is UNIX_EPOCH

    let conversation = UiConversation::from(conversation);
    UiConversationDetails {
        id: conversation.id,
        group_id: conversation.group_id,
        status: conversation.status,
        conversation_type: conversation.conversation_type,
        last_used,
        attributes: conversation.attributes,
        unread_messages,
        last_message,
    }
}
