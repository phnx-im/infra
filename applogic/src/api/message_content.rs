// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
pub use phnxcommon::identifiers::AttachmentId;
use uuid::Uuid;

use crate::api::markdown::MessageContent;

/// Mirror of the [`AttachmentId`] type
#[doc(hidden)]
#[frb(mirror(AttachmentId))]
#[frb(dart_code = "
    @override
    String toString() => 'AttachmentId($uuid)';
")]
pub struct _AttachmentId {
    pub uuid: Uuid,
}

/// The actual content of a message
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"))]
pub struct UiMimiContent {
    pub replaces: Option<Vec<u8>>,
    pub topic_id: Vec<u8>,
    pub in_reply_to: Option<Vec<u8>>,
    pub plain_body: Option<String>,
    pub content: Option<MessageContent>,
    pub attachments: Vec<UiAttachment>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"), type_64bit_int)]
pub struct UiAttachment {
    pub attachment_id: AttachmentId,
    pub filename: String,
    pub content_type: String,
    pub description: Option<String>,
    pub size: u64,
    pub image_metadata: Option<UiImageMetadata>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"))]
pub struct UiImageMetadata {
    pub blurhash: String,
    pub width: u32,
    pub height: u32,
}
