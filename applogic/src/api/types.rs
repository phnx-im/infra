// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use chrono::{DateTime, Duration, Utc};
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    Contact, ContentMessage, Conversation, ConversationAttributes, ConversationMessage,
    ConversationStatus, ConversationType, ErrorMessage, EventMessage, InactiveConversation,
    Message, MessageId, MimiContent, NotificationType, SystemMessage, UserProfile,
};
pub use phnxcoreclient::{ConversationId, ConversationMessageId};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct _GroupId {
    pub uuid: Uuid,
}

#[frb(mirror(ConversationId))]
#[frb(dart_code = "
    @override
    String toString() => 'ConversationId($uuid)';
")]
pub struct _ConversationId {
    pub uuid: Uuid,
}

#[frb(dart_code = "
    @override
    String toString() => 'GroupId(${hex.encode(bytes)})';
")]
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct GroupId {
    pub bytes: Vec<u8>,
}

impl From<openmls::group::GroupId> for GroupId {
    fn from(group_id: openmls::group::GroupId) -> Self {
        Self {
            bytes: group_id.as_slice().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversation {
    pub id: ConversationId,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupId,
    pub status: UiConversationStatus,
    pub conversation_type: UiConversationType,
    pub attributes: UiConversationAttributes,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversationDetails {
    pub id: ConversationId,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupId,
    pub status: UiConversationStatus,
    pub conversation_type: UiConversationType,
    pub last_used: String,
    pub attributes: UiConversationAttributes,
    pub messages_count: u32,
    pub unread_messages: u32,
    pub last_message: Option<UiConversationMessage>,
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub enum UiConversationStatus {
    Inactive(UiInactiveConversation),
    Active,
}

impl From<ConversationStatus> for UiConversationStatus {
    fn from(status: ConversationStatus) -> Self {
        match status {
            ConversationStatus::Inactive(inactive) => {
                UiConversationStatus::Inactive(UiInactiveConversation::from(inactive))
            }
            ConversationStatus::Active => UiConversationStatus::Active,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct UiInactiveConversation {
    pub past_members: Vec<String>,
}

impl From<InactiveConversation> for UiInactiveConversation {
    fn from(inactive: InactiveConversation) -> Self {
        Self {
            past_members: inactive
                .past_members()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub enum UiConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(String),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(String),
    Group,
}

impl From<ConversationType> for UiConversationType {
    fn from(conversation_type: ConversationType) -> Self {
        match conversation_type {
            ConversationType::UnconfirmedConnection(user_name) => {
                UiConversationType::UnconfirmedConnection(user_name.to_string())
            }
            ConversationType::Connection(user_name) => {
                UiConversationType::Connection(user_name.to_string())
            }
            ConversationType::Group => UiConversationType::Group,
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct UiConversationAttributes {
    pub title: String,
    pub picture: Option<Vec<u8>>,
}

impl fmt::Debug for UiConversationAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiConversationAttributes")
            .field("title", &self.title)
            .field("picture", &self.picture.as_ref().map(|b| b.len()))
            .finish()
    }
}

impl From<ConversationAttributes> for UiConversationAttributes {
    fn from(attributes: ConversationAttributes) -> Self {
        Self {
            title: attributes.title().to_string(),
            picture: attributes.picture().map(|a| a.to_vec()),
        }
    }
}

impl From<Conversation> for UiConversation {
    fn from(conversation: Conversation) -> Self {
        Self {
            id: conversation.id(),
            group_id: GroupId::from(conversation.group_id().clone()),
            status: UiConversationStatus::from(conversation.status().clone()),
            conversation_type: UiConversationType::from(conversation.conversation_type().clone()),
            attributes: UiConversationAttributes::from(conversation.attributes().clone()),
        }
    }
}

#[frb(mirror(ConversationMessageId))]
#[frb(dart_code = "
    @override
    String toString() => 'ConversationMessageId($uuid)';
")]
pub struct _ConversationMessageId {
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversationMessage {
    pub conversation_id: ConversationId,
    pub id: ConversationMessageId,
    pub timestamp: String, // We don't convert this to a DateTime because Dart can't handle nanoseconds.
    pub message: UiMessage,
    pub position: UiFlightPosition,
}

impl UiConversationMessage {
    pub(crate) fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp.parse().ok()
    }
}

impl From<ConversationMessage> for UiConversationMessage {
    fn from(conversation_message: ConversationMessage) -> Self {
        Self {
            conversation_id: conversation_message.conversation_id(),
            id: conversation_message.id(),
            timestamp: conversation_message.timestamp().to_rfc3339(),
            message: UiMessage::from(conversation_message.message().clone()),
            position: UiFlightPosition::Unique,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiMessage {
    Content(Box<UiContentMessage>),
    Display(UiEventMessage),
}

impl From<Message> for UiMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::Content(content_message) => {
                UiMessage::Content(Box::new(UiContentMessage::from(content_message)))
            }
            Message::Event(display_message) => {
                UiMessage::Display(UiEventMessage::from(display_message))
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiMessageId {
    pub id: Uuid,
    pub domain: String,
}

impl From<MessageId> for UiMessageId {
    fn from(message_id: MessageId) -> Self {
        Self {
            id: message_id.id(),
            domain: message_id.domain().to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiReplyToInfo {
    pub message_id: UiMessageId,
    pub hash: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiMimiContent {
    pub id: UiMessageId,
    pub timestamp: DateTime<Utc>,
    pub replaces: Option<UiMessageId>,
    pub topic_id: Option<Vec<u8>>,
    pub expires: Option<DateTime<Utc>>,
    pub in_reply_to: Option<UiReplyToInfo>,
    pub last_seen: Vec<UiMessageId>,
    // This will need to become more complex.
    pub body: String,
}

impl From<MimiContent> for UiMimiContent {
    fn from(mimi_content: MimiContent) -> Self {
        let body = mimi_content.string_rendering();
        Self {
            id: UiMessageId::from(mimi_content.id().clone()),
            timestamp: mimi_content.timestamp.into(),
            replaces: mimi_content.replaces.map(UiMessageId::from),
            topic_id: mimi_content.topic_id.map(|t| t.id.to_vec()),
            expires: mimi_content.expires.map(|e| e.into()),
            in_reply_to: mimi_content.in_reply_to.map(|i| UiReplyToInfo {
                message_id: UiMessageId::from(i.message_id),
                hash: i.hash.hash,
            }),
            last_seen: mimi_content
                .last_seen
                .into_iter()
                .map(UiMessageId::from)
                .collect(),
            body,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiContentMessage {
    pub sender: String,
    pub sent: bool,
    pub content: UiMimiContent,
}

impl From<ContentMessage> for UiContentMessage {
    fn from(content_message: ContentMessage) -> Self {
        Self {
            sender: content_message.sender().to_string(),
            sent: content_message.was_sent(),
            content: UiMimiContent::from(content_message.content().clone()),
        }
    }
}

impl From<Box<ContentMessage>> for UiContentMessage {
    fn from(content_message: Box<ContentMessage>) -> Self {
        Self::from(*content_message)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiEventMessage {
    System(UiSystemMessage),
    Error(UiErrorMessage),
}

impl From<EventMessage> for UiEventMessage {
    fn from(event_message: EventMessage) -> Self {
        match event_message {
            EventMessage::System(message) => UiEventMessage::System(message.into()),
            EventMessage::Error(message) => UiEventMessage::Error(message.into()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiSystemMessage {
    pub message: String,
}

impl From<SystemMessage> for UiSystemMessage {
    fn from(system_message: SystemMessage) -> Self {
        Self {
            message: system_message.to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiErrorMessage {
    pub message: String,
}

impl From<ErrorMessage> for UiErrorMessage {
    fn from(error_message: ErrorMessage) -> Self {
        Self {
            message: error_message.into(),
        }
    }
}

/// Position of a conversation message in a flight.
///
/// A flight is a sequence of messages that are grouped to be displayed together.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum UiFlightPosition {
    /// The message is the only message in the flight.
    Unique,
    /// The message is the first message in the flight and the flight has more than one message.
    Start,
    /// The message is in the middle of the flight and the flight has more than one message.
    Middle,
    /// The message is the last message in the flight and the flight has more than one message.
    End,
}

impl UiFlightPosition {
    /// Calculate the position of a `message` in a flight.
    ///
    /// The position is determined by the message and its previous and next messages in the
    /// conversation timeline.
    ///
    /// The implementation of this function defines which messages are grouped together in a
    /// flight.
    pub(crate) fn calculate(
        message: &UiConversationMessage,
        prev_message: Option<&UiConversationMessage>,
        next_message: Option<&UiConversationMessage>,
    ) -> Self {
        match (prev_message, next_message) {
            (None, None) => Self::Unique,
            (Some(_prev), None) => Self::End,
            (None, Some(_next)) => Self::Start,
            (Some(prev), Some(next)) => {
                let at_flight_start = Self::flight_break_condition(prev, message);
                let at_flight_end = Self::flight_break_condition(message, next);
                match (at_flight_start, at_flight_end) {
                    (true, true) => Self::Unique,
                    (true, false) => Self::Start,
                    (false, true) => Self::End,
                    (false, false) => Self::Middle,
                }
            }
        }
    }

    /// Returns true if there is a flight break between the messages `a` and `b`.
    fn flight_break_condition(a: &UiConversationMessage, b: &UiConversationMessage) -> bool {
        const TIME_THRESHOLD: Duration = Duration::minutes(1);
        match (&a.message, &b.message) {
            (UiMessage::Content(a_content), UiMessage::Content(b_content)) => {
                a_content.sender != b_content.sender
                    || a.timestamp()
                        .zip(b.timestamp())
                        .map(|(a_timestamp, b_timestamp)| {
                            TIME_THRESHOLD <= b_timestamp.signed_duration_since(a_timestamp).abs()
                        })
                        .unwrap_or(true)
            }
            // all non-content messages are considered to be flight breaks
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UiContact {
    pub user_name: String,
}

impl From<Contact> for UiContact {
    fn from(contact: Contact) -> Self {
        Self {
            user_name: contact.user_name().to_string(),
        }
    }
}

pub struct UiUserProfile {
    pub user_name: String,
    pub display_name: Option<String>,
    pub profile_picture: Option<Vec<u8>>,
}

impl UiUserProfile {
    pub(crate) fn from_profile(user_profile: &UserProfile) -> Self {
        Self {
            user_name: user_profile.user_name().to_string(),
            display_name: user_profile.display_name().map(|name| name.to_string()),
            profile_picture: user_profile
                .profile_picture()
                .and_then(|asset| asset.value())
                .map(|bytes| bytes.to_vec()),
        }
    }
}
