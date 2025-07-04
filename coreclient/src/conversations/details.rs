// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};

use crate::{Conversation, ConversationMessage, conversations::draft::ConversationMessageDraft};

/// A conversation with additional details
pub struct ConversationDetails {
    pub conversation: Conversation,
    pub messages_count: usize,
    pub unread_messages: usize,
    pub last_message: Option<ConversationMessage>,
    pub last_used: DateTime<Utc>,
    pub draft: Option<ConversationMessageDraft>,
}

mod persistence {
    use sqlx::SqliteTransaction;

    use super::*;

    use crate::{
        ConversationId,
        clients::{CoreUser, conversations::ConversationMessageDraft},
        store::Store,
    };

    impl ConversationDetails {
        pub(crate) async fn load(
            txn: &mut SqliteTransaction<'_>,
            conversation_id: &ConversationId,
        ) -> sqlx::Result<Option<Self>> {
            let Some(conversation) = Conversation::load(txn, conversation_id).await? else {
                return Ok(None);
            };

            let messages_count = store
                .messages_count(conversation_id)
                .await?
                .unwrap_or_default();
            let unread_messages = store
                .unread_messages_count(conversation_id)
                .await?
                .unwrap_or_default();
            let last_message = store
                .last_message(conversation_id)
                .await?
                .map(|m| ConversationMessage::from(m));
            let last_used = conversation.last_read();
            let draft = ConversationMessageDraft::load(store, conversation_id).await?;
            Ok(Some(Self {
                conversation,
                messages_count,
                unread_messages,
                last_message,
                last_used,
                draft,
            }))
        }
    }
}
