//! # OpenMLS Delivery Service Library
//!
//! This library provides structs and necessary implementations to interact with
//! the OpenMLS DS.
//!
//! Clients are represented by the `ClientInfo` struct.

use openmls::prelude::{MlsMessageIn, Welcome};
use tls_codec::{Deserialize, Serialize, Size, TlsDeserialize, TlsSerialize, TlsSize};


/// The DS returns a list of messages on `/recv/{name}`, which is a
/// `Vec<Message>`. A `Message` is either an `MlSMessage` or a `Welcome` message
/// (see OpenMLS) for details.
#[derive(Debug, Clone)]
pub enum DsQueueMessage {
    /// An `MlSMessageIn` is either an OpenMLS `MLSCiphertext` or `MLSPlaintext`.
    MlsMessage(MlsMessageIn),

    /// An OpenMLS `Welcome` message.
    Welcome(Welcome),
}

/// Enum defining encodings for the different message types/
#[derive(Debug, Clone, Copy, TlsSize, TlsSerialize, TlsDeserialize)]
#[allow(clippy::upper_case_acronyms)]
#[repr(u8)]
pub enum MessageType {
    /// An MlsMessage message.
    MlsMessage = 0,
    /// A Welcome message.
    Welcome = 1,
}

impl Size for DsQueueMessage {
    fn tls_serialized_len(&self) -> usize {
        match self {
            DsQueueMessage::MlsMessage(m) => {
                MessageType::MlsMessage.tls_serialized_len() + m.tls_serialized_len()
            }

            DsQueueMessage::Welcome(m) => {
                MessageType::Welcome.tls_serialized_len() + m.tls_serialized_len()
            }
        }
    }
}

impl Serialize for DsQueueMessage {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        Ok(match self {
            DsQueueMessage::MlsMessage(m) => {
                MessageType::MlsMessage.tls_serialize(writer)? + m.tls_serialize(writer)?
            }

            DsQueueMessage::Welcome(m) => {
                MessageType::Welcome.tls_serialize(writer)? + m.tls_serialize(writer)?
            }
        })
    }
}

impl Deserialize for DsQueueMessage {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let msg_type = MessageType::tls_deserialize(bytes)?;
        let msg = match msg_type {
            MessageType::MlsMessage => {
                DsQueueMessage::MlsMessage(MlsMessageIn::tls_deserialize(bytes)?)
            }
            MessageType::Welcome => DsQueueMessage::Welcome(Welcome::tls_deserialize(bytes)?),
        };
        Ok(msg)
    }
}
