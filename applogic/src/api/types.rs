// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Types exposed to the Flutter app
//!
//! Some types are mirrored, especially identifiers. All types that are not mirrored are prefixed
//! with `Ui`.

use std::fmt;

use chrono::{DateTime, Duration, Utc};
use flutter_rust_bridge::frb;
pub use phnxcommon::identifiers::UserHandle;
use phnxcommon::identifiers::UserId;
use phnxcoreclient::{
    Asset, Contact, ContentMessage, ConversationAttributes, ConversationMessage,
    ConversationStatus, ConversationType, DisplayName, ErrorMessage, EventMessage,
    InactiveConversation, Message, MessageDraft, MessageStatusBit, SystemMessage, UserProfile,
    store::Store,
};
pub use phnxcoreclient::{ConversationId, ConversationMessageId};
use uuid::Uuid;

use crate::api::message_content::UiMimiContent;

/// Mirror of the [`ConversationId`] type
#[doc(hidden)]
#[frb(mirror(ConversationId))]
#[frb(dart_code = "
    @override
    String toString() => 'ConversationId($uuid)';
")]
pub struct _ConversationId {
    pub uuid: Uuid,
}

/// UI representation of an [`UserId`]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_code = "
    @override
    String toString() => '$uuid@$domain';
")]
pub struct UiUserId {
    pub uuid: Uuid,
    pub domain: String,
}

impl From<UserId> for UiUserId {
    fn from(user_id: UserId) -> Self {
        let (uuid, domain) = user_id.into_parts();
        Self {
            uuid,
            domain: domain.into(),
        }
    }
}

impl From<UiUserId> for UserId {
    fn from(user_id: UiUserId) -> Self {
        UserId::new(
            user_id.uuid,
            user_id.domain.parse().expect("logic error: invalid data"),
        )
    }
}

/// A conversation which is a 1:1 connection or a group conversation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversation {
    pub id: ConversationId,
    pub status: UiConversationStatus,
    pub conversation_type: UiConversationType,
    pub attributes: UiConversationAttributes,
}

/// Details of a conversation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(type_64bit_int)]
pub struct UiConversationDetails {
    pub id: ConversationId,
    pub status: UiConversationStatus,
    pub conversation_type: UiConversationType,
    pub last_used: String,
    pub attributes: UiConversationAttributes,
    pub messages_count: usize,
    pub unread_messages: usize,
    pub last_message: Option<UiConversationMessage>,
    pub draft: Option<UiMessageDraft>,
}

/// Draft of a message in a conversation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"))]
pub struct UiMessageDraft {
    pub message: String,
    pub editing_id: Option<ConversationMessageId>,
    pub updated_at: DateTime<Utc>,
}

impl UiMessageDraft {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            editing_id: None,
            updated_at: Utc::now(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.message.trim().is_empty() && self.editing_id.is_none()
    }
}

impl From<MessageDraft> for UiMessageDraft {
    fn from(
        MessageDraft {
            message,
            editing_id,
            updated_at,
        }: MessageDraft,
    ) -> Self {
        Self {
            message,
            editing_id,
            updated_at,
        }
    }
}

impl From<UiMessageDraft> for MessageDraft {
    fn from(
        UiMessageDraft {
            message,
            editing_id,
            updated_at,
        }: UiMessageDraft,
    ) -> Self {
        Self {
            message,
            editing_id,
            updated_at,
        }
    }
}

/// Status of a conversation
///
/// A conversation can be inactive or active.
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

/// Inactive conversation with past members
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct UiInactiveConversation {
    pub past_members: Vec<UiUserId>,
}

impl From<InactiveConversation> for UiInactiveConversation {
    fn from(inactive: InactiveConversation) -> Self {
        Self {
            past_members: inactive
                .past_members()
                .iter()
                .cloned()
                .map(From::from)
                .collect(),
        }
    }
}

/// Type of a conversation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiConversationType {
    /// A connection conversation which was established via a handle and is not yet confirmed by
    /// the other party.
    HandleConnection(UiUserHandle),
    /// A connection conversation that is confirmed by the other party and for which we have
    /// received the necessary secrets.
    Connection(UiUserProfile),
    /// A group conversation, that is, it can contains multiple participants.
    Group,
}

impl UiConversationType {
    /// Converts [`ConversationType`] to [`UiConversationType`] but also load the corresponding
    /// user profile.
    ///
    /// If the user profile cannot be loaded, or is not set, a minimal user profile is returned
    /// with the display name derived from the client id.
    #[frb(ignore)]
    pub(crate) async fn load_from_conversation_type(
        store: &impl Store,
        conversation_type: ConversationType,
    ) -> Self {
        match conversation_type {
            ConversationType::HandleConnection(handle) => {
                Self::HandleConnection(UiUserHandle::from(handle))
            }
            ConversationType::Connection(user_id) => {
                let user_profile = store.user_profile(&user_id).await;
                let profile = UiUserProfile::from_profile(user_profile);
                Self::Connection(profile)
            }
            ConversationType::Group => Self::Group,
        }
    }
}

/// Attributes of a conversation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UiConversationAttributes {
    /// Title of the conversation
    pub title: String,
    /// Optional picture of the conversation
    pub picture: Option<ImageData>,
}

impl From<ConversationAttributes> for UiConversationAttributes {
    fn from(attributes: ConversationAttributes) -> Self {
        Self {
            title: attributes.title().to_string(),
            picture: attributes
                .picture()
                .map(|a| ImageData::from_bytes(a.to_vec())),
        }
    }
}

/// Mirror of the [`ConversationMessageId`] type
#[doc(hidden)]
#[frb(mirror(ConversationMessageId))]
#[frb(dart_code = "
    @override
    String toString() => 'ConversationMessageId($uuid)';
")]
pub struct _ConversationMessageId {
    pub uuid: Uuid,
}

/// A message in a conversation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiConversationMessage {
    pub conversation_id: ConversationId,
    pub id: ConversationMessageId,
    pub timestamp: String, // We don't convert this to a DateTime because Dart can't handle nanoseconds.
    pub message: UiMessage,
    pub position: UiFlightPosition,
    pub status: UiMessageStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum UiMessageStatus {
    Sending,
    /// The message was sent to the server.
    Sent,
    /// The message was received by at least one user in the conversation.
    Delivered,
    /// The message was read by at least one user in the conversation.
    Read,
}

impl From<ConversationMessage> for UiConversationMessage {
    #[frb(ignore)]
    fn from(message: ConversationMessage) -> Self {
        let status = if !message.is_sent() {
            UiMessageStatus::Sending
        } else if !message.status().contains(MessageStatusBit::Delivered)
            && !message.status().contains(MessageStatusBit::Read)
        {
            UiMessageStatus::Sent
        } else if message.status().contains(MessageStatusBit::Delivered)
            && !message.status().contains(MessageStatusBit::Read)
        {
            UiMessageStatus::Delivered
        } else {
            UiMessageStatus::Read
        };
        Self {
            conversation_id: message.conversation_id(),
            id: message.id(),
            timestamp: message.timestamp().to_rfc3339(),
            message: UiMessage::from(message.message().clone()),
            position: UiFlightPosition::Single,
            status,
        }
    }
}

impl UiConversationMessage {
    pub(crate) fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp.parse().ok()
    }
}

/// The actual message in a conversation
///
/// Can be either a message to display (e.g. a system message) or a message from a user.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiMessage {
    Content(Box<UiContentMessage>),
    Display(UiEventMessage),
}

impl From<Message> for UiMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::Content(content_message) => {
                UiMessage::Content(Box::new(UiContentMessage::from(*content_message)))
            }
            Message::Event(display_message) => {
                UiMessage::Display(UiEventMessage::from(display_message))
            }
        }
    }
}

/// Content of a message including the sender and whether it was sent
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiContentMessage {
    pub sender: UiUserId,
    pub sent: bool,
    pub content: UiMimiContent,
}

impl From<ContentMessage> for UiContentMessage {
    fn from(content_message: ContentMessage) -> Self {
        let (sender, sent, content) = content_message.into_parts();
        Self {
            sender: sender.into(),
            sent,
            content: UiMimiContent::from(content),
        }
    }
}

/// Event message (e.g. a message from the system)
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

/// System message
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UiSystemMessage {
    Add(UiUserId, UiUserId),
    Remove(UiUserId, UiUserId),
}

impl From<SystemMessage> for UiSystemMessage {
    fn from(system_message: SystemMessage) -> Self {
        match system_message {
            SystemMessage::Add(user_id, contact_id) => {
                UiSystemMessage::Add(user_id.into(), contact_id.into())
            }
            SystemMessage::Remove(user_id, contact_id) => {
                UiSystemMessage::Remove(user_id.into(), contact_id.into())
            }
        }
    }
}

/// Error message
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

/// Position of a conversation message in a flight
///
/// A flight is a sequence of messages that are grouped to be displayed together.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum UiFlightPosition {
    /// The message is the only message in the flight.
    Single,
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
            (None, None) => Self::Single,
            (Some(prev), None) => {
                if Self::flight_break_condition(prev, message) {
                    Self::Single
                } else {
                    Self::End
                }
            }
            (None, Some(next)) => {
                if Self::flight_break_condition(message, next) {
                    Self::Single
                } else {
                    Self::Start
                }
            }
            (Some(prev), Some(next)) => {
                let at_flight_start = Self::flight_break_condition(prev, message);
                let at_flight_end = Self::flight_break_condition(message, next);
                match (at_flight_start, at_flight_end) {
                    (true, true) => Self::Single,
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

/// Contact of the logged-in user
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UiContact {
    pub user_id: UiUserId,
}

impl From<Contact> for UiContact {
    fn from(contact: Contact) -> Self {
        Self {
            user_id: contact.user_id.into(),
        }
    }
}

/// Profile of a user
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct UiUserProfile {
    /// ID of the user
    pub user_id: UiUserId,
    /// Display name
    pub display_name: String,
    /// Optional profile picture
    pub profile_picture: Option<ImageData>,
}

impl UiUserProfile {
    pub(crate) fn from_profile(user_profile: UserProfile) -> Self {
        Self {
            user_id: user_profile.user_id.into(),
            display_name: user_profile.display_name.into_string(),
            profile_picture: user_profile.profile_picture.map(ImageData::from_asset),
        }
    }

    pub(crate) fn from_user_id(user_id: UserId) -> Self {
        let display_name = DisplayName::from_user_id(&user_id);
        Self {
            user_id: user_id.into(),
            display_name: display_name.into_string(),
            profile_picture: None,
        }
    }
}

/// Image binary data together with its hashsum
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ImageData {
    /// The image data
    pub(crate) data: Vec<u8>,
    /// Opaque hash of the image data as hex string
    pub(crate) hash: String,
}

impl fmt::Debug for ImageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageData")
            .field("data", &self.data.len())
            .field("hash", &self.hash)
            .finish()
    }
}

impl ImageData {
    pub(crate) fn from_bytes(data: Vec<u8>) -> Self {
        let hash = Self::compute_hash(&data);
        Self { data, hash }
    }

    pub(crate) fn from_asset(asset: Asset) -> Self {
        match asset {
            Asset::Value(bytes) => Self::from_bytes(bytes),
        }
    }

    /// Computes opaque hashsum of the data and returns it as a hex string.
    #[frb(sync, positional)]
    pub fn compute_hash(bytes: &[u8]) -> String {
        let hash = blake3::hash(bytes);
        hash.to_hex().to_string()
    }
}

/// Client record of a user
///
/// Each user has a client record which identifies the users database.
#[derive(Debug)]
pub struct UiClientRecord {
    /// The unique identifier of the user
    ///
    /// Also used for identifying the client database path.
    pub(crate) user_id: UiUserId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) user_profile: UiUserProfile,
    pub(crate) is_finished: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"))]
pub struct UiUserHandle {
    pub(crate) plaintext: String,
}

impl UiUserHandle {
    /// Returns `None` if the handle is valid, otherwise returns an error message why it is
    /// invalid.
    #[frb(sync)]
    pub fn validation_error(&self) -> Option<String> {
        if let Err(error) = UserHandle::new(self.plaintext.clone()) {
            Some(error.to_string())
        } else {
            None
        }
    }
}

impl From<UserHandle> for UiUserHandle {
    fn from(user_handle: UserHandle) -> Self {
        Self {
            plaintext: user_handle.into_plaintext(),
        }
    }
}
