// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use mimi_content::{
    MimiContent,
    content_container::{Disposition, NestedPart, NestedPartContent, PartSemantics},
};
pub use phnxcoreclient::clients::attachment::AttachmentId;
use tracing::warn;
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
    pub plain_body: String,
    pub content: MessageContent,
    pub attachments: Vec<UiAttachment>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[frb(dart_metadata = ("freezed"))]
pub struct UiAttachment {
    pub attachment_id: AttachmentId,
    pub filename: String,
    pub content_type: String,
    pub blurhash: Option<String>,
    pub discription: Option<String>,
}

impl From<MimiContent> for UiMimiContent {
    fn from(mut mimi_content: MimiContent) -> Self {
        let (plain_body, attachments) = match (
            mimi_content.nested_part.disposition,
            std::mem::take(&mut mimi_content.nested_part.part),
        ) {
            // multipart attachment message with ProcessAll semantics
            (
                Disposition::Attachment,
                NestedPartContent::MultiPart {
                    part_semantics: PartSemantics::ProcessAll,
                    parts,
                },
            ) => {
                let Some(attachment) = convert_attachment(parts) else {
                    return error_message(mimi_content, "Unsupported attachment message");
                };
                (attachment.filename.clone(), vec![attachment])
            }

            // single part message
            (
                _,
                NestedPartContent::SinglePart {
                    content,
                    content_type,
                },
            ) if content_type == "text/markdown" => {
                let plain_body = String::from_utf8(content.into_vec())
                    .unwrap_or_else(|_| "Invalid non-UTF8 message".to_owned());
                (plain_body, Default::default())
            }

            // any other message
            (disposition, _) => {
                return error_message(
                    mimi_content,
                    format!("Unsupported message: {disposition:?}"),
                );
            }
        };

        let parsed_message = MessageContent::parse_markdown(&plain_body);

        Self {
            plain_body,
            replaces: mimi_content.replaces.map(|v| v.into_vec()),
            topic_id: mimi_content.topic_id.into_vec(),
            in_reply_to: mimi_content.in_reply_to.map(|i| i.hash.into_vec()),
            content: parsed_message,
            attachments,
        }
    }
}

fn error_message(mimi_content: MimiContent, message: impl Into<String>) -> UiMimiContent {
    let message = message.into();
    UiMimiContent {
        plain_body: message.clone(),
        replaces: mimi_content.replaces.map(|v| v.into_vec()),
        topic_id: mimi_content.topic_id.into_vec(),
        in_reply_to: mimi_content.in_reply_to.map(|i| i.hash.into_vec()),
        content: MessageContent::error(message),
        attachments: Default::default(),
    }
}

fn convert_attachment(parts: Vec<NestedPart>) -> Option<UiAttachment> {
    let mut attachment: Option<UiAttachment> = None;
    let mut blurhash: Option<String> = None;

    for part in parts {
        match (part.disposition, part.part) {
            // actual attachment
            (
                Disposition::Attachment,
                NestedPartContent::ExternalPart {
                    content_type,
                    url,
                    description,
                    filename,
                    ..
                },
            ) => {
                if attachment.is_some() {
                    warn!("Skipping duplicate attachment part");
                    continue;
                }

                let Some(attachment_id) = AttachmentId::from_url(&url) else {
                    warn!(url, "Skipping attachment part with invalid url");
                    continue;
                };

                attachment = Some(UiAttachment {
                    attachment_id,
                    filename,
                    content_type,
                    blurhash: None,
                    discription: Some(description).filter(|d| !d.is_empty()),
                });
            }

            // blurhash preview
            (
                Disposition::Preview,
                NestedPartContent::SinglePart {
                    content,
                    content_type,
                },
            ) if content_type == "text/blurhash" => {
                if blurhash.is_some() {
                    warn!("Skipping duplicate blurhash preview part");
                    continue;
                }
                let Ok(content) = String::from_utf8(content.to_vec()) else {
                    warn!("Skipping blurhash preview with non-UTF8 content");
                    continue;
                };
                blurhash = Some(content);
            }

            // other parts
            (disposition, _) => {
                warn!("Skipping unsupported attachment part: {disposition:?}");
            }
        }
    }

    if let Some(attachment) = &mut attachment {
        attachment.blurhash = blurhash;
    }

    attachment
}
