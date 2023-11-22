// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxcoreclient::{
    ContentMessage, Conversation, ConversationAttributes, ConversationId, ConversationMessage,
    ConversationStatus, ConversationType, DispatchedConversationMessage, DisplayMessage,
    DisplayMessageType, ErrorMessage, InactiveConversation, Knock, Message, MessageContentType,
    NotificationType, SystemMessage, TextMessage,
};
use phnxtypes::identifiers::UserName;
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

impl GroupIdBytes {
    pub fn as_group_id(&self) -> GroupId {
        GroupId::from_slice(&self.bytes)
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct ConversationIdBytes {
    pub bytes: [u8; 16],
}

impl From<ConversationId> for ConversationIdBytes {
    fn from(conversation_id: ConversationId) -> Self {
        Self {
            bytes: conversation_id.as_uuid().to_bytes_le(),
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

impl UiInactiveConversation {
    pub fn new(past_members: Vec<UserName>) -> Self {
        Self {
            past_members: past_members
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        }
    }

    pub fn past_members(&self) -> Vec<UserName> {
        self.past_members
            .iter()
            .map(|s| UserName::from(s.clone()))
            .collect()
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
}

impl From<ConversationAttributes> for UiConversationAttributes {
    fn from(attributes: ConversationAttributes) -> Self {
        Self {
            title: attributes.title,
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
    pub timestamp: u64,
    pub message: UiMessage,
}

impl From<ConversationMessage> for UiConversationMessage {
    fn from(conversation_message: ConversationMessage) -> Self {
        Self {
            conversation_id: ConversationIdBytes::from(conversation_message.conversation_id),
            id: UuidBytes::from(conversation_message.id),
            timestamp: conversation_message.timestamp.as_u64(),
            message: UiMessage::from(conversation_message.message),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum UiMessage {
    Content(UiContentMessage),
    Display(UiDisplayMessage),
}

impl From<Message> for UiMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::Content(content_message) => {
                UiMessage::Content(UiContentMessage::from(content_message))
            }
            Message::Display(display_message) => {
                UiMessage::Display(UiDisplayMessage::from(display_message))
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiContentMessage {
    pub sender: String,
    pub content: UiMessageContentType,
}

impl From<ContentMessage> for UiContentMessage {
    fn from(content_message: ContentMessage) -> Self {
        Self {
            sender: content_message.sender.to_string(),
            content: UiMessageContentType::from(content_message.content),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
#[repr(u16)]
pub enum UiMessageContentType {
    Text(UiTextMessage),
    Knock(UiKnock),
}

impl From<UiMessageContentType> for MessageContentType {
    fn from(ui_message_content_type: UiMessageContentType) -> Self {
        match ui_message_content_type {
            UiMessageContentType::Text(text_message) => {
                MessageContentType::Text(TextMessage::from(text_message))
            }
            UiMessageContentType::Knock(knock) => MessageContentType::Knock(knock.into()),
        }
    }
}

impl From<MessageContentType> for UiMessageContentType {
    fn from(message_content_type: MessageContentType) -> Self {
        match message_content_type {
            MessageContentType::Text(text_message) => {
                UiMessageContentType::Text(UiTextMessage::from(text_message))
            }
            MessageContentType::Knock(knock) => UiMessageContentType::Knock(knock.into()),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiTextMessage {
    pub message: Vec<u8>,
}

impl From<TextMessage> for UiTextMessage {
    fn from(text_message: TextMessage) -> Self {
        Self {
            message: text_message.message().to_vec(),
        }
    }
}

impl From<UiTextMessage> for TextMessage {
    fn from(ui_text_message: UiTextMessage) -> Self {
        TextMessage::new(ui_text_message.message)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiKnock {}

impl From<Knock> for UiKnock {
    fn from(_: Knock) -> Self {
        Self {}
    }
}

impl From<UiKnock> for Knock {
    fn from(_: UiKnock) -> Self {
        Self {}
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct UiDisplayMessage {
    pub message: UiDisplayMessageType,
}

impl From<DisplayMessage> for UiDisplayMessage {
    fn from(display_message: DisplayMessage) -> Self {
        Self {
            message: UiDisplayMessageType::from(display_message.message),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum UiDisplayMessageType {
    System(UiSystemMessage),
    Error(UiErrorMessage),
}

impl From<DisplayMessageType> for UiDisplayMessageType {
    fn from(display_message_type: DisplayMessageType) -> Self {
        match display_message_type {
            DisplayMessageType::System(message) => UiDisplayMessageType::System(message.into()),
            DisplayMessageType::Error(message) => UiDisplayMessageType::Error(message.into()),
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
            message: system_message.message().to_string(),
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

#[derive(PartialEq, Debug, Clone)]
pub struct UiDispatchedConversationMessage {
    pub conversation_id: ConversationIdBytes,
    pub conversation_message: UiConversationMessage,
}

impl From<DispatchedConversationMessage> for UiDispatchedConversationMessage {
    fn from(dispatched_conversation_message: DispatchedConversationMessage) -> Self {
        Self {
            conversation_id: ConversationIdBytes::from(
                dispatched_conversation_message.conversation_id,
            ),
            conversation_message: UiConversationMessage::from(
                dispatched_conversation_message.conversation_message,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiNotificationsRequest {}

#[derive(Debug, Clone)]
pub enum UiNotificationType {
    ConversationChange(UuidBytes), // The id of the changed conversation.
    Message(UiDispatchedConversationMessage),
}

impl From<NotificationType> for UiNotificationType {
    fn from(value: NotificationType) -> Self {
        match value {
            NotificationType::ConversationChange(conversation_id) => {
                UiNotificationType::ConversationChange(conversation_id.as_uuid().into())
            }
            NotificationType::Message(message) => UiNotificationType::Message(message.into()),
        }
    }
}
