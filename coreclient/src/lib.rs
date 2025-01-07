// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Implements the protocol logic of the client component

pub mod clients;
mod contacts;
mod conversations;
mod groups;
mod key_stores;
mod mimi_content;
pub mod store;
mod user_profiles;
mod utils;

pub use crate::{
    contacts::{Contact, PartialContact},
    conversations::{
        messages::{
            ContentMessage, ConversationMessage, ConversationMessageId,
            ConversationMessageNeighbor, ConversationMessageNeighbors, ErrorMessage, EventMessage,
            Message, NotificationType, SystemMessage,
        },
        Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
        InactiveConversation,
    },
    mimi_content::{MessageId, MimiContent, ReplyToInfo, TopicId},
    user_profiles::{Asset, DisplayName, DisplayNameError, UserProfile},
};

pub use crate::utils::persistence::delete_databases;
