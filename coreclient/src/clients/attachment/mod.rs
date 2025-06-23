// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::Utc;
use infer::MatcherType;
use mimi_content::{
    MimiContent,
    content_container::{Disposition, HashAlgorithm, NestedPart, NestedPartContent, PartSemantics},
};
use phnxapiclient::ApiClient;
use phnxcommon::{
    credentials::keys::ClientSigningKey,
    crypto::ear::{AeadCiphertext, EarEncryptable, keys::AttachmentEarKey},
    identifiers::AttachmentId,
};

use crate::{
    Conversation, ConversationId, ConversationMessage, ConversationMessageId,
    clients::{
        CoreUser,
        attachment::{
            ear::{PHNX_ATTACHMENT_ENCRYPTION_ALG, PHNX_BLAKE3_HASH_ID},
            persistence::{AttachmentImageRecord, AttachmentStatus},
        },
    },
    groups::Group,
    utils::image::{ReencodedAttachmentImage, reencode_attachment_image},
};

pub(crate) use persistence::AttachmentRecord;

mod content;
mod download;
mod ear;
mod persistence;
mod process;

/// In-memory loaded and processed attachment
///
/// If it is an image, it will contain additional image data, like a thumbnail and blurhash.
struct Attachment {
    filename: String,
    content: AttachmentContent,
    mime: Option<infer::Type>,
    image_data: Option<AttachmentImageData>,
}

#[derive(derive_more::From)]
struct AttachmentContent {
    bytes: Vec<u8>,
}

impl AttachmentContent {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl AsRef<[u8]> for AttachmentContent {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Attachment {
    fn from_file(path: &Path) -> anyhow::Result<Self> {
        // TODO: Avoid reading the whole file into memory when it is an image.
        // Instead, it should be re-encoded directly from the file.
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file at {}", path.display()))?;
        let mime = infer::get(&content);

        let (content, image_data) = if mime
            .map(|mime| mime.matcher_type() == MatcherType::Image)
            .unwrap_or(false)
        {
            let ReencodedAttachmentImage {
                webp_image,
                image_dimensions: (width, height),
                blurhash,
            } = reencode_attachment_image(content)?;
            let image_data = AttachmentImageData {
                blurhash,
                width,
                height,
            };
            (webp_image.into(), Some(image_data))
        } else {
            (content.into(), None)
        };

        let mut filename = PathBuf::from(
            path.file_name()
                .unwrap_or_else(|| OsStr::new("attachment.bin")),
        );
        if image_data.is_some() {
            filename.set_extension("webp");
        }

        Ok(Self {
            filename: filename.to_string_lossy().to_string(),
            content,
            mime,
            image_data,
        })
    }

    fn mime_type(&self) -> &'static str {
        self.mime
            .as_ref()
            .map(|mime| mime.mime_type())
            .unwrap_or("application/octet-stream")
    }

    fn to_nested_parts(&self, metadata: &AttachmentMetadata) -> Vec<NestedPart> {
        let attachment = NestedPart {
            disposition: Disposition::Attachment,
            language: String::new(),
            part: NestedPartContent::ExternalPart {
                content_type: self.mime_type().to_owned(),
                url: metadata.attachment_id.url(),
                expires: 0,
                size: metadata.size,
                enc_alg: PHNX_ATTACHMENT_ENCRYPTION_ALG,
                key: metadata.key.clone().into_bytes().to_vec().into(),
                nonce: metadata.nonce.to_vec().into(),
                aad: Default::default(),
                hash_alg: HashAlgorithm::Custom(PHNX_BLAKE3_HASH_ID),
                content_hash: metadata.content_hash.clone().into(),
                description: Default::default(),
                filename: self.filename.clone(),
            },
        };

        let blurhash = self.image_data.as_ref().map(|data| NestedPart {
            disposition: Disposition::Preview,
            language: String::new(),
            part: NestedPartContent::SinglePart {
                content_type: "text/blurhash".to_owned(),
                content: data.blurhash.clone().into_bytes().into(),
            },
        });

        [Some(attachment), blurhash].into_iter().flatten().collect()
    }
}

struct AttachmentImageData {
    blurhash: String,
    width: u32,
    height: u32,
}

impl CoreUser {
    pub(crate) async fn upload_attachment(
        &self,
        conversation_id: ConversationId,
        path: &Path,
    ) -> anyhow::Result<ConversationMessage> {
        let (conversation, group) = self
            .with_transaction(async |txn| {
                let conversation = Conversation::load(txn, &conversation_id)
                    .await?
                    .with_context(|| {
                        format!("Can't find conversation with id {conversation_id}")
                    })?;

                let group_id = conversation.group_id();
                let group = Group::load_clean(txn, group_id)
                    .await?
                    .with_context(|| format!("Can't find group with id {group_id:?}"))?;
                Ok((conversation, group))
            })
            .await?;

        // load the attachment data
        let attachment = Attachment::from_file(path)?;

        // encrypt the content and upload the content
        let api_client = self.api_client()?;
        let http_client = self.http_client();

        let attachment_metadata = encrypt_and_upload(
            &api_client,
            &http_client,
            self.signing_key(),
            &attachment.content,
            &group,
        )
        .await?;

        // Note: Acquire a transaction here to ensure that the attachment will be deleted from the
        // local database in case of an error.
        let mut txn = self.pool().begin().await?;
        let mut notifier = self.store_notifier();

        // send attachment message
        let content = MimiContent {
            nested_part: NestedPart {
                disposition: Disposition::Attachment,
                part: NestedPartContent::MultiPart {
                    part_semantics: PartSemantics::ProcessAll,
                    parts: attachment.to_nested_parts(&attachment_metadata),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let conversation_message_id = ConversationMessageId::random();
        let message = self
            .send_message_transactional(
                &mut txn,
                &mut notifier,
                conversation_id,
                conversation_message_id,
                content,
            )
            .await?;

        // store attachment locally
        // (must be done after the message is stored locally due to foreign key constraints)
        let record = AttachmentRecord {
            attachment_id: attachment_metadata.attachment_id,
            conversation_id: conversation.id(),
            conversation_message_id,
            content_type: attachment.mime_type().to_owned(),
            status: AttachmentStatus::Ready,
            arrived_at: Utc::now(),
        };
        let image_record = if let Some(image_data) = attachment.image_data.as_ref() {
            Some(AttachmentImageRecord {
                attachment_id: attachment_metadata.attachment_id.uuid(),
                blurhash: image_data.blurhash.clone(),
                width: image_data.width,
                height: image_data.height,
            })
        } else {
            None
        };
        record
            .store(
                txn.as_mut(),
                &mut notifier,
                Some(attachment.content.as_ref()),
            )
            .await?;
        if let Some(image_record) = image_record {
            image_record.store(txn.as_mut()).await?;
        }

        txn.commit().await?;
        notifier.notify();

        Ok(message)
    }
}

/// Metadata of an encrypted and uploaded attachment
struct AttachmentMetadata {
    attachment_id: AttachmentId,
    key: AttachmentEarKey,
    content_hash: Vec<u8>,
    size: u64,
    nonce: [u8; 12],
}

async fn encrypt_and_upload(
    api_client: &ApiClient,
    http_client: &reqwest::Client,
    signing_key: &ClientSigningKey,
    content: &AttachmentContent,
    group: &Group,
) -> anyhow::Result<AttachmentMetadata> {
    // encrypt the content
    let key = AttachmentEarKey::random()?;
    let ciphertext: AeadCiphertext = content.encrypt(&key)?.into();
    let (ciphertext, nonce) = ciphertext.into_parts();
    let size: u64 = ciphertext
        .len()
        .try_into()
        .context("attachment size overflow")?;
    let content_hash = blake3::hash(&ciphertext).as_bytes().to_vec();

    // provision attachment
    let response = api_client
        .ds_provision_attachment(
            signing_key,
            group.group_state_ear_key(),
            group.group_id(),
            group.own_index(),
        )
        .await?;
    let attachment_id =
        AttachmentId::new(response.attachment_id.context("no attachment id")?.into());

    // upload encrypted content
    let mut request = http_client.put(response.upload_url);
    for header in response.upload_headers {
        request = request.header(header.key, header.value);
    }
    request.body(ciphertext).send().await?.error_for_status()?;

    Ok(AttachmentMetadata {
        attachment_id,
        key,
        content_hash,
        size,
        nonce,
    })
}
