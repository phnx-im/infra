// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use openmls::prelude::GroupId;
use phnxbackend::auth_service::UserName;
use serde::{Deserialize, Serialize};

use crate::types::*;

use super::*;

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct ConversationStore {
    #[serde(
        serialize_with = "crate::utils::serialize_hashmap",
        deserialize_with = "crate::utils::deserialize_hashmap"
    )]
    conversations: HashMap<Uuid, Conversation>,
    messages: HashMap<Uuid, HashMap<Uuid, ConversationMessage>>,
}

impl ConversationStore {
    pub(crate) fn conversation_by_group_id(&self, group_id: &GroupId) -> Option<&Conversation> {
        self.conversations
            .values()
            .find(|conversation| conversation.group_id.as_group_id() == *group_id)
    }

    pub(crate) fn create_connection_conversation(
        &mut self,
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Uuid {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        let conversation = Conversation {
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name.to_string()),
            last_used: Timestamp::now().as_u64(),
            attributes,
        };
        self.conversations
            .insert(uuid_bytes.as_uuid().clone(), conversation);
        uuid_bytes.as_uuid()
    }

    pub(crate) fn create_group_conversation(
        &mut self,
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Uuid {
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        let conversation_id = uuid_bytes.as_uuid();
        let conversation = Conversation {
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            last_used: Timestamp::now().as_u64(),
            attributes,
        };
        self.conversations
            .insert(conversation_id.clone(), conversation);
        conversation_id
    }

    pub(crate) fn confirm_connection_conversation(&mut self, conversation_id: &Uuid) {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            if let ConversationType::UnconfirmedConnection(user_name) =
                conversation.conversation_type.clone()
            {
                conversation.conversation_type = ConversationType::Connection(user_name);
            }
        }
    }

    pub(crate) fn conversations(&self) -> Vec<Conversation> {
        let mut conversations: Vec<Conversation> = self.conversations.values().cloned().collect();
        conversations.sort_by(|a, b| a.last_used.cmp(&b.last_used));
        conversations
    }

    pub(crate) fn conversation(&self, conversation_id: Uuid) -> Option<&Conversation> {
        self.conversations.get(&conversation_id)
    }

    pub(crate) fn set_inactive(&mut self, conversation_id: Uuid, past_members: &[String]) {
        self.conversations
            .get_mut(&conversation_id)
            .map(|conversation| {
                conversation.status = ConversationStatus::Inactive(InactiveConversation {
                    past_members: past_members.iter().map(|m| m.to_owned()).collect(),
                })
            });
    }

    pub(crate) fn messages(
        &self,
        conversation_id: Uuid,
        last_n: usize,
    ) -> Vec<ConversationMessage> {
        match self.messages.get(&conversation_id) {
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
        conversation_id: Uuid,
        message: ConversationMessage,
    ) -> Result<(), ConversationStoreError> {
        let message_id = message.id.clone();
        match self.conversations.get(&conversation_id) {
            Some(_conversation_data) => {
                match self.messages.get_mut(&conversation_id) {
                    Some(conversation_messages) => {
                        conversation_messages.insert(message_id.as_uuid(), message);
                    }
                    None => {
                        let mut conversation_messages = HashMap::new();
                        conversation_messages.insert(message_id.as_uuid(), message);
                        self.messages.insert(conversation_id, conversation_messages);
                    }
                }
                Ok(())
            }
            None => Err(ConversationStoreError::UnknownConversation),
        }
    }
}
