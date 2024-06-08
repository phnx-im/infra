// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use phnxcoreclient::{
    Asset, Contact, ContentMessage, Conversation, ConversationAttributes, ConversationId,
    ConversationMessage, ConversationStatus, ConversationType, DisplayName, ErrorMessage,
    EventMessage, InactiveConversation, Message, MessageId, MimiContent, NotificationType,
    SystemMessage, UserProfile,
};
use phnxtypes::identifiers::SafeTryInto;
use uuid::Uuid;

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct GroupIdBytes {
    pub bytes: Vec<u8>,
}

impl From<GroupId> for GroupIdBytes {
    fn from(group_id: GroupId) -> Self {
        Self {
            bytes: group_id.as_slice().to_vec(),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct ConversationIdBytes {
    pub bytes: [u8; 16],
}

impl From<ConversationId> for ConversationIdBytes {
    fn from(conversation_id: ConversationId) -> Self {
        Self {
            bytes: conversation_id.as_uuid().into_bytes(),
        }
    }
}

impl From<ConversationIdBytes> for ConversationId {
    fn from(conversation_id: ConversationIdBytes) -> Self {
        ConversationId::from(Uuid::from_bytes(conversation_id.bytes))
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct UuidBytes {
    pub bytes: [u8; 16],
}

impl From<Uuid> for UuidBytes {
    fn from(uuid: Uuid) -> Self {
        Self {
            bytes: *uuid.as_bytes(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversation {
    pub id: ConversationIdBytes,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupIdBytes,
    pub status: UiConversationStatus,
    pub conversation_type: UiConversationType,
    pub last_used: u64,
    pub attributes: UiConversationAttributes,
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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UiConversationAttributes {
    pub title: String,
    pub conversation_picture_option: Option<Vec<u8>>,
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
            id: ConversationIdBytes::from(conversation.id()),
            group_id: GroupIdBytes::from(conversation.group_id().clone()),
            status: UiConversationStatus::from(conversation.status().clone()),
            conversation_type: UiConversationType::from(conversation.conversation_type().clone()),
            last_used: conversation.last_used().as_u64(),
            attributes: UiConversationAttributes::from(conversation.attributes().clone()),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiConversationMessage {
    pub conversation_id: ConversationIdBytes,
    pub id: UuidBytes,
    pub timestamp: DateTime<Utc>,
    pub message: UiMessage,
}

impl From<ConversationMessage> for UiConversationMessage {
    fn from(conversation_message: ConversationMessage) -> Self {
        Self {
            conversation_id: ConversationIdBytes::from(conversation_message.conversation_id()),
            id: UuidBytes::from(conversation_message.id()),
            timestamp: conversation_message.timestamp().time(),
            message: UiMessage::from(conversation_message.message().clone()),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum UiMessage {
    Content(UiContentMessage),
    Display(UiEventMessage),
    Unsent(UiMimiContent),
}

impl From<Message> for UiMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::Content(content_message) => {
                UiMessage::Content(UiContentMessage::from(content_message))
            }
            Message::Event(display_message) => {
                UiMessage::Display(UiEventMessage::from(display_message))
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiMessageId {
    pub id: UuidBytes,
    pub domain: String,
}

impl From<MessageId> for UiMessageId {
    fn from(message_id: MessageId) -> Self {
        Self {
            id: UuidBytes::from(message_id.id()),
            domain: message_id.domain().to_string(),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiReplyToInfo {
    pub message_id: UiMessageId,
    pub hash: Vec<u8>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiMimiContent {
    pub id: UiMessageId,
    pub timestamp: u64,
    pub replaces: Option<UiMessageId>,
    pub topic_id: Option<Vec<u8>>,
    pub expires: Option<u64>,
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
            timestamp: mimi_content.timestamp.as_u64(),
            replaces: mimi_content.replaces.map(UiMessageId::from),
            topic_id: mimi_content.topic_id.map(|t| t.id.to_vec()),
            expires: mimi_content.expires.map(|e| e.as_u64()),
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

#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
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
pub enum UiNotificationType {
    ConversationChange(ConversationIdBytes), // The id of the changed conversation.
    Message(UiConversationMessage),
}

impl From<NotificationType> for UiNotificationType {
    fn from(value: NotificationType) -> Self {
        match value {
            NotificationType::ConversationChange(conversation_id) => {
                UiNotificationType::ConversationChange(conversation_id.into())
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

impl From<UserProfile> for UiUserProfile {
    fn from(user_profile: UserProfile) -> Self {
        Self {
            user_name: user_profile.user_name().to_string(),
            display_name: user_profile.display_name().map(|a| a.to_string()),
            profile_picture_option: user_profile
                .profile_picture()
                .and_then(|a| a.value())
                .map(|a| a.to_vec()),
        }
    }
}

impl TryFrom<UiUserProfile> for UserProfile {
    type Error = anyhow::Error;

    fn try_from(value: UiUserProfile) -> Result<Self, Self::Error> {
        let user_name = <String as SafeTryInto<_>>::try_into(value.user_name)?;
        let display_name = value.display_name.map(DisplayName::try_from).transpose()?;
        let profile_picture_option = value.profile_picture_option.map(Asset::Value);
        Ok(UserProfile::new(
            user_name,
            display_name,
            profile_picture_option,
        ))
    }
}
