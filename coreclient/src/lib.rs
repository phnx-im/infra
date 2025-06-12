// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Implements the protocol logic of the client component

pub mod clients;
mod contacts;
mod conversations;
mod groups;
mod key_stores;
pub mod store;
mod user_handles;
mod user_profiles;
mod utils;

pub use crate::{
    contacts::{Contact, PartialContact},
    conversations::{
        Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
        InactiveConversation,
        messages::{
            ContentMessage, ConversationMessage, ConversationMessageId, ErrorMessage, EventMessage,
            Message, NotificationType, SystemMessage,
        },
    },
    user_handles::UserHandleRecord,
    user_profiles::{Asset, DisplayName, DisplayNameError, UserProfile},
    utils::persistence::{delete_client_database, delete_databases, open_client_db},
};
