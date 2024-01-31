// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
mod errors;
mod contacts;
mod conversations;
mod groups;
mod key_stores;
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
            Knock, Message, MessageContentType, NotificationType, SystemMessage, TextMessage,
        },
        Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
        InactiveConversation,
    },
    groups::GroupMessage,
};

pub use crate::utils::persistence::delete_databases;

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
