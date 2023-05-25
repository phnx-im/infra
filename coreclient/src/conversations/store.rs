// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use crate::types::*;

use super::*;

#[derive(Default)]
pub(crate) struct ConversationStore {
    conversations: HashMap<Uuid, Conversation>,
    messages: HashMap<Uuid, HashMap<Uuid, ConversationMessage>>,
}

impl ConversationStore {
    pub(crate) fn create_group_conversation(
        &mut self,
        conversation_id: Uuid,
        attributes: ConversationAttributes,
    ) {
        let conversation = Conversation {
            id: UuidBytes::from_uuid(&conversation_id),
            conversation_type: ConversationType::Group.into(),
            last_used: Timestamp::now().as_u64(),
            attributes: attributes,
            status: ConversationStatus::Active(ActiveConversation {}),
        };
        self.conversations.insert(conversation_id, conversation);
    }

    pub(crate) fn conversations(&self) -> Vec<Conversation> {
        let mut conversations: Vec<Conversation> = self.conversations.values().cloned().collect();
        conversations.sort_by(|a, b| a.last_used.cmp(&b.last_used));
        conversations
    }

    pub(crate) fn _conversation(&self, conversation_id: &Uuid) -> Option<&Conversation> {
        self.conversations.get(conversation_id)
    }

    pub(crate) fn messages(
        &self,
        conversation_id: &Uuid,
        last_n: usize,
    ) -> Vec<ConversationMessage> {
        match self.messages.get(conversation_id) {
            Some(messages) => {
                let mut messages: Vec<ConversationMessage> = messages
                    .iter()
                    .map(|(_uuid, message)| message.clone())
                    .collect();
                messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                if last_n >= messages.len() {
                    messages
                } else {
                    let (_left, right) = messages.split_at(messages.len() - last_n);
                    right.to_vec()
                }
            }
            None => {
                vec![]
            }
        }
    }

    pub(crate) fn store_message(
        &mut self,
        conversation_id: &Uuid,
        message: ConversationMessage,
    ) -> Result<(), ConversationStoreError> {
        let message_id = message.id.clone();
        match self.conversations.get(conversation_id) {
            Some(_conversation_data) => {
                match self.messages.get_mut(conversation_id) {
                    Some(conversation_messages) => {
                        conversation_messages.insert(message_id.as_uuid(), message);
                    }
                    None => {
                        let mut conversation_messages = HashMap::new();
                        conversation_messages.insert(message_id.as_uuid(), message);
                        self.messages
                            .insert(*conversation_id, conversation_messages);
                    }
                }
                Ok(())
            }
            None => Err(ConversationStoreError::UnknownConversation),
        }
    }
}
