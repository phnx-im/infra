// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
pub use phnxcoreclient::ConversationId;
use phnxcoreclient::{
    Contact, ContentMessage, Conversation, ConversationAttributes, ConversationMessage,
    ConversationMessageId, ConversationStatus, ConversationType, ErrorMessage, EventMessage,
    InactiveConversation, Message, MessageId, MimiContent, NotificationType, SystemMessage,
    UserProfile,
};
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
    pub conversation_picture_option: Option<Vec<u8>>,
}

impl fmt::Debug for UiConversationAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiConversationAttributes")
            .field("title", &self.title)
            .field(
                "conversation_picture_option",
                &self.conversation_picture_option.as_ref().map(|b| b.len()),
            )
            .finish()
    }
}

impl From<ConversationAttributes> for UiConversationAttributes {
    fn from(attributes: ConversationAttributes) -> Self {
        Self {
            title: attributes.title().to_string(),
            conversation_picture_option: attributes
                .conversation_picture_option()
                .map(|a| a.to_vec()),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct UiConversationMessageId {
    pub uuid: Uuid,
}

impl From<ConversationMessageId> for UiConversationMessageId {
    fn from(id: ConversationMessageId) -> Self {
        Self { uuid: id.to_uuid() }
    }
}

impl From<UiConversationMessageId> for ConversationMessageId {
    fn from(id: UiConversationMessageId) -> Self {
        Self::from_uuid(id.uuid)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversationMessage {
    pub conversation_id: ConversationId,
    pub id: UiConversationMessageId,
    pub timestamp: String, // We don't convert this to a DateTime because Dart can't handle nanoseconds.
    pub message: UiMessage,
    pub is_read: bool,
}

impl From<ConversationMessage> for UiConversationMessage {
    fn from(conversation_message: ConversationMessage) -> Self {
        Self {
            conversation_id: conversation_message.conversation_id(),
            id: UiConversationMessageId::from(conversation_message.id()),
            timestamp: conversation_message.timestamp().to_rfc3339(),
            message: UiMessage::from(conversation_message.message().clone()),
            is_read: conversation_message.is_read(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiMessage {
    ContentFlight(Vec<UiContentMessage>),
    Display(UiEventMessage),
    Unsent(Box<UiMimiContent>),
}

impl From<Message> for UiMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::Content(content_message) => {
                UiMessage::ContentFlight(vec![UiContentMessage::from(content_message)])
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
            message: error_message.message().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum UiNotificationType {
    ConversationChange(ConversationId), // The id of the changed conversation.
    Message(UiConversationMessage),
}

impl From<NotificationType> for UiNotificationType {
    fn from(value: NotificationType) -> Self {
        match value {
            NotificationType::ConversationChange(conversation_id) => {
                UiNotificationType::ConversationChange(conversation_id)
            }
            NotificationType::Message(message) => UiNotificationType::Message(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
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
    pub profile_picture_option: Option<Vec<u8>>,
}

impl UiUserProfile {
    pub(crate) fn from_profile(user_profile: &UserProfile) -> Self {
        Self {
            user_name: user_profile.user_name().to_string(),
            display_name: user_profile.display_name().map(|name| name.to_string()),
            profile_picture_option: user_profile
                .profile_picture()
                .and_then(|asset| asset.value())
                .map(|bytes| bytes.to_vec()),
        }
    }
}
