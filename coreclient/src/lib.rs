// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
mod errors;
mod contacts;
mod conversations;
mod groups;
mod key_stores;
mod mimi_content;
mod providers;
pub mod users;
mod utils;

use std::collections::HashMap;

pub(crate) use crate::errors::*;

pub use crate::{
    contacts::{Contact, PartialContact},
    conversations::{
        messages::{
            ContentMessage, ConversationMessage, DisplayMessage, DisplayMessageType, ErrorMessage,
            Message, NotificationType, SystemMessage,
        },
        Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
        InactiveConversation,
    },
    mimi_content::{MessageId, MimiContent, ReplyToInfo, TopicId},
};

pub use crate::utils::persistence::delete_databases;

pub(crate) use openmls::prelude::*;
