pub(crate) mod error;
pub(crate) mod store;

pub(crate) use error::*;
pub(crate) use store::*;

use crate::{types::*, utils::*};

use uuid::Uuid;

pub(crate) fn new_conversation_message(message: Message) -> ConversationMessage {
    ConversationMessage {
        id: UuidBytes::from_uuid(&Uuid::new_v4()),
        timestamp: Timestamp::now().as_u64(),
        message,
    }
}
