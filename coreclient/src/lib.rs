// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
mod errors;
mod contacts;
mod conversations;
mod groups;
mod key_stores;
pub mod notifications;
mod providers;
pub mod users;
mod utils;

use std::collections::HashMap;

pub(crate) use crate::errors::*;

pub use crate::conversations::{
    messages::{
        ContentMessage, ConversationMessage, DispatchedConversationMessage, DisplayMessage,
        DisplayMessageType, ErrorMessage, Knock, Message, MessageContentType, NotificationType,
        SystemMessage, TextMessage,
    },
    Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
    InactiveConversation,
};
pub use crate::groups::GroupMessage;

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
