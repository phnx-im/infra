// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Formatter;

use openmls::framing::ApplicationMessage;

use crate::mimi_content::MimiContent;

use super::*;

pub(crate) mod persistence;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TimestampedMessage {
    timestamp: TimeStamp,
    message: Message,
}

impl TimestampedMessage {
    /// Returns the timestamp of the message. If the message was sent, it's the
    /// timestamp issues by the DS, otherwise it's the timestamp when the
    /// message was created.
    pub(crate) fn timestamp(&self) -> TimeStamp {
        self.timestamp
    }

    /// Mark the message as sent and update the timestamp. If the message was
    /// already marked as sent, nothing happens.  
    pub(super) fn mark_as_sent(&mut self, ds_timestamp: TimeStamp) {
        if let Message::Content(content) = &mut self.message {
            self.timestamp = ds_timestamp;
            content.sent = true
        }
    }

    /// Create a new timestamped message from an incoming application message.
    /// The message is marked as sent.
    pub(crate) fn from_application_message(
        application_message: ApplicationMessage,
        ds_timestamp: TimeStamp,
        sender_name: UserName,
    ) -> Result<Self, tls_codec::Error> {
        let content = MimiContent::tls_deserialize_exact_bytes(&application_message.into_bytes())?;
        let message = Message::Content(ContentMessage::new(sender_name.to_string(), true, content));
        Ok(Self {
            timestamp: ds_timestamp,
            message,
        })
    }

    pub(crate) fn from_message_and_timestamp(message: Message, ds_timestamp: TimeStamp) -> Self {
        Self {
            message,
            timestamp: ds_timestamp,
        }
    }

    pub(crate) fn system_message(system_message: SystemMessage, ds_timestamp: TimeStamp) -> Self {
        let message = Message::Event(EventMessage::System(system_message));
        Self {
            message,
            timestamp: ds_timestamp,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub(super) conversation_id: ConversationId,
    pub(super) local_message_id: Uuid,
    pub(super) timestamped_message: TimestampedMessage,
}

impl ConversationMessage {
    /// Create a new conversation message from a group message. New messages are
    /// marked as unread by default.
    pub(crate) fn from_timestamped_message(
        conversation_id: ConversationId,
        timestamped_message: TimestampedMessage,
    ) -> ConversationMessage {
        ConversationMessage {
            conversation_id,
            local_message_id: Uuid::new_v4(),
            timestamped_message,
        }
    }

    pub(crate) fn new_unsent_message(
        sender: String,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> ConversationMessage {
        let message = Message::Content(ContentMessage::new(sender, false, content));
        let timestamped_message =
            TimestampedMessage::from_message_and_timestamp(message, TimeStamp::now());
        ConversationMessage {
            conversation_id,
            local_message_id: Uuid::new_v4(),
            timestamped_message,
        }
    }

    /// Mark the message as sent and update the timestamp.
    pub(crate) fn mark_as_sent(
        &mut self,
        connection: &Connection,
        ds_timestamp: TimeStamp,
    ) -> Result<(), rusqlite::Error> {
        self.timestamped_message.mark_as_sent(ds_timestamp);
        self.update_sent_status(connection, ds_timestamp, true)
    }

    pub fn id_ref(&self) -> &Uuid {
        &self.local_message_id
    }

    pub fn id(&self) -> Uuid {
        self.local_message_id
    }

    pub fn timestamp(&self) -> TimeStamp {
        self.timestamped_message.timestamp()
    }

    pub fn was_sent(&self) -> bool {
        if let Message::Content(content) = &self.timestamped_message.message {
            content.was_sent()
        } else {
            true
        }
    }

    pub fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn message(&self) -> &Message {
        &self.timestamped_message.message
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Content(ContentMessage),
    Event(EventMessage),
}

impl Message {
    /// Returns a string representation of the message for use in UI
    /// notifications.
    pub fn string_representation(&self, conversation_type: &ConversationType) -> String {
        match self {
            Message::Content(content_message) => match conversation_type {
                ConversationType::Group => {
                    let sender = &content_message.sender;
                    let content = content_message.content.string_rendering();
                    format!("{sender}: {content}")
                }
                ConversationType::Connection(_) | ConversationType::UnconfirmedConnection(_) => {
                    let content = content_message.content.string_rendering();
                    format!("{content}")
                }
            },
            Message::Event(event_message) => match &event_message {
                EventMessage::System(system) => system.to_string(),
                EventMessage::Error(error) => error.message().to_string(),
            },
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ContentMessage {
    pub(super) sender: String,
    pub(super) sent: bool,
    pub(super) content: MimiContent,
}

impl ContentMessage {
    pub fn new(sender: String, sent: bool, content: MimiContent) -> Self {
        Self {
            sender,
            sent,
            content,
        }
    }

    pub fn was_sent(&self) -> bool {
        self.sent
    }

    pub fn sender(&self) -> &str {
        self.sender.as_ref()
    }

    pub fn content(&self) -> &MimiContent {
        &self.content
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum EventMessage {
    System(SystemMessage),
    Error(ErrorMessage),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    // The first UserName is the adder/remover the second is the added/removed.
    Add(UserName, UserName),
    Remove(UserName, UserName),
}

impl Display for SystemMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemMessage::Add(adder, added) => {
                if adder == added {
                    write!(f, "{} joined the conversation", adder)
                } else {
                    write!(f, "{} added {} to the conversation", adder, added)
                }
            }
            SystemMessage::Remove(remover, removed) => {
                if remover == removed {
                    write!(f, "{} left the conversation", remover)
                } else {
                    write!(f, "{} removed {} from the conversation", remover, removed)
                }
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    message: String,
}

impl ErrorMessage {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        self.message.as_ref()
    }
}

#[derive(Debug, Clone)]
pub enum NotificationType {
    ConversationChange(ConversationId), // The id of the changed conversation.
    Message(ConversationMessage),
}
