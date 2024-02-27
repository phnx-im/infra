// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxtypes::{
    identifiers::{AsClientId, Fqdn, UserName},
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use url::Url;
use uuid::Uuid;

use self::builder::MimiContentBuilder;

mod builder;
mod codec;

// A TLS encoded byte string that contains a UTF-8 encoded string.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
struct TlsStr<'a> {
    value: &'a str,
}

impl<'a> From<&'a str> for TlsStr<'a> {
    fn from(value: &'a str) -> Self {
        Self { value }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
struct TlsStrOwned {
    value: String,
}

/// A domain-scoped message id.
///
/// This is only pub(super), because we add such an id for event message also.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct MessageId {
    id: Uuid,
    domain: Fqdn,
}

impl MessageId {
    pub(crate) fn new(domain: Fqdn) -> Self {
        Self {
            id: Uuid::new_v4(),
            domain,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub(super) fn id_ref(&self) -> &Uuid {
        &self.id
    }

    pub fn domain(&self) -> &Fqdn {
        &self.domain
    }
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
pub struct TopicId {
    id: Vec<u8>,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[repr(u8)]
pub enum HashAlg {
    None,
    Sha256,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
pub struct ReplyToHash {
    hash_alg: HashAlg,
    hash: Vec<u8>,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
pub struct ReplyToInfo {
    message_id: MessageId,
    hash: ReplyToHash,
}

/// IANA Media Type
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
enum ContentType {
    TextMarkdown,
    // Add more as needed
}

/// These are the (IANA) content types we support at the moment.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
enum SinglePart {
    TextMarkdown(String),
    // Add more as needed
}

impl SinglePart {
    fn template(&self) -> String {
        match self {
            SinglePart::TextMarkdown(_) => "text/markdown".to_string(),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
struct ExternalPartUrl {
    url: Url,
}

// This is a placeholder for the actual IANA registered AEAD algorithm.
#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[repr(u16)]
enum AeadAlg {
    // This is just a placeholder.
    None,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
struct ExternalPart {
    content_type: ContentType,  // An IANA media type {10}
    url: ExternalPartUrl,       // A URL where the content can be fetched
    expires: Option<TimeStamp>, // This is actually a u32 and needs to be parsed as such. 0 means no expiration, i.e. None.
    size: u64,                  // size of content in octets
    aead_alg: AeadAlg,          // An IANA AEAD Algorithm number, or zero
    // TODO: Key and nonce need their own types.
    key: Vec<u8>,             // AEAD key
    nonce: Vec<u8>,           // AEAD nonce
    aad: Vec<u8>,             // AEAD additional authentiation data
    description: TlsStrOwned, // an optional text description
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
struct MultiParts {
    pars: Vec<NestablePart>,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[repr(u16)]
enum PartSemantics {
    NullPart,
    SinglePart,
    ChooseOne,
    SingleUnit,
    ProcessAll,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[repr(u16)]
enum Part {
    NullPart,
    SinglePart(SinglePart),
    ExternalPart(ExternalPart),
    MultiParts(MultiParts),
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[repr(u16)]
enum Disposition {
    Unspecified,
    Render,
    Reaction,
    Profile,
    Inline,
    Icon,
    Attachment,
    Session,
    Preview,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
struct Language {
    language: TlsStrOwned,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
struct NestablePart {
    disposition: Disposition,
    languages: Vec<Language>,
    part_index: u16,
    part_semantic: PartSemantics,
    part: Part,
}

impl Default for NestablePart {
    fn default() -> Self {
        Self {
            disposition: Disposition::Unspecified,
            languages: Vec::new(),
            part_index: 0,
            part_semantic: PartSemantics::NullPart,
            part: Part::NullPart,
        }
    }
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
struct MessageDerivedValues {
    mls_group_id: GroupId,
    sender_leaf_index: u32,
    sender_client_id: AsClientId,
    sender_user_id: UserName,
    group_name: TlsStrOwned,
}

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
pub struct MimiContent {
    id: MessageId,
    timestamp: TimeStamp,
    replaces: Option<MessageId>,
    topic_id: Option<TopicId>,
    // The point in time when the message expires. If None, the message never
    // expires.
    expires: Option<TimeStamp>, // This is actually a u32 and needs to be parsed as such. 0 means no expiration, i.e. None.
    in_reply_to: Option<ReplyToInfo>,
    last_seen: Vec<MessageId>,
    body: NestablePart,
}

impl MimiContent {
    pub fn simple_markdown_message(sender_domain: Fqdn, markdown_text: String) -> Self {
        // For now, we just encode text as markdown.
        let single_part = SinglePart::TextMarkdown(markdown_text);
        let nestable_part = NestablePart {
            disposition: Disposition::Render,
            languages: Vec::new(),
            part_index: 0,
            part_semantic: PartSemantics::SinglePart,
            part: Part::SinglePart(single_part),
        };
        MimiContentBuilder::new(sender_domain, nestable_part).build()
    }

    pub(crate) fn string_rendering(&self) -> String {
        // For now, we only support SingleParts that contain markdown messages.
        match &self.body.part {
            Part::SinglePart(single_part) => match single_part {
                SinglePart::TextMarkdown(text) => text.clone(),
            },
            _ => "Unsupported content type".to_string(),
        }
    }

    pub fn id(&self) -> &MessageId {
        &self.id
    }
}
