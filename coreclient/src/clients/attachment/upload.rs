// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    ffi::OsStr,
    mem,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::Utc;
use infer::MatcherType;
use mimi_content::{
    MimiContent,
    content_container::{Disposition, NestedPart, NestedPartContent, PartSemantics},
};
use phnxapiclient::ApiClient;
use phnxcommon::{
    credentials::keys::ClientSigningKey,
    crypto::ear::{AeadCiphertext, EarEncryptable, keys::AttachmentEarKey},
    identifiers::AttachmentId,
};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    AttachmentStatus, Conversation, ConversationId, ConversationMessage, ConversationMessageId,
    clients::{
        CoreUser,
        attachment::{
            AttachmentBytes, AttachmentRecord,
            ear::{PHNX_ATTACHMENT_ENCRYPTION_ALG, PHNX_ATTACHMENT_HASH_ALG},
        },
    },
    groups::Group,
    utils::image::{ReencodedAttachmentImage, reencode_attachment_image},
};

impl CoreUser {
    /// Uploads an attachment and sends a message containing it.
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
        let mut attachment = ProcessedAttachment::from_file(path)?;

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

        // send attachment message
        let attachment_id = attachment_metadata.attachment_id;
        let content_bytes = mem::take(&mut attachment.content.bytes);
        let content_type = attachment.mime_type();

        let content = MimiContent {
            nested_part: NestedPart {
                disposition: Disposition::Attachment,
                part: NestedPartContent::MultiPart {
                    part_semantics: PartSemantics::ProcessAll,
                    parts: attachment.into_nested_parts(attachment_metadata)?,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Note: Acquire a transaction here to ensure that the attachment will be deleted from the
        // local database in case of an error.
        self.with_transaction_and_notifier(async |txn, notifier| {
            let conversation_message_id = ConversationMessageId::random();
            let message = self
                .send_message_transactional(
                    txn,
                    notifier,
                    conversation_id,
                    conversation_message_id,
                    content,
                )
                .await?;

            // store attachment locally
            // (must be done after the message is stored locally due to foreign key constraints)
            let record = AttachmentRecord {
                attachment_id,
                conversation_id: conversation.id(),
                conversation_message_id,
                content_type: content_type.to_owned(),
                status: AttachmentStatus::Ready,
                created_at: Utc::now(),
            };
            record
                .store(txn.as_mut(), notifier, Some(&content_bytes))
                .await?;

            Ok(message)
        })
        .await
    }
}

/// In-memory loaded and processed attachment
///
/// If it is an image, it will contain additional image data, like a blurhash.
struct ProcessedAttachment {
    filename: String,
    content: AttachmentBytes,
    content_hash: Vec<u8>,
    mime: Option<infer::Type>,
    image_data: Option<ProcessedAttachmentImageData>,
}

struct ProcessedAttachmentImageData {
    blurhash: String,
    width: u32,
    height: u32,
}

impl ProcessedAttachment {
    fn from_file(path: &Path) -> anyhow::Result<Self> {
        // TODO(#589): Avoid reading the whole file into memory when it is an image.
        // Instead, it should be re-encoded directly from the file.
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file at {}", path.display()))?;
        let mime = infer::get(&content);

        let content_hash = Sha256::digest(&content).to_vec();

        let (content, image_data) = if mime
            .map(|mime| mime.matcher_type() == MatcherType::Image)
            .unwrap_or(false)
        {
            let ReencodedAttachmentImage {
                webp_image,
                image_dimensions: (width, height),
                blurhash,
            } = reencode_attachment_image(content)?;
            let image_data = ProcessedAttachmentImageData {
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
            content_hash,
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

    fn into_nested_parts(self, metadata: AttachmentMetadata) -> anyhow::Result<Vec<NestedPart>> {
        let url = metadata.attachment_id.url();
        let mut url = Url::parse(&url)?;
        // TODO: Currently, there is no way to specify the image dimensions in the MIMI content.
        // This is a workaround for that.
        if let Some(image_data) = &self.image_data {
            url.query_pairs_mut()
                .append_pair("width", &image_data.width.to_string());
            url.query_pairs_mut()
                .append_pair("height", &image_data.height.to_string());
        }

        let attachment = NestedPart {
            disposition: Disposition::Attachment,
            language: String::new(),
            part: NestedPartContent::ExternalPart {
                content_type: self.mime_type().to_owned(),
                url: url.to_string(),
                expires: 0,
                size: metadata.size,
                enc_alg: PHNX_ATTACHMENT_ENCRYPTION_ALG,
                key: metadata.key.into_bytes().to_vec().into(),
                nonce: metadata.nonce.to_vec().into(),
                aad: Default::default(),
                hash_alg: PHNX_ATTACHMENT_HASH_ALG,
                content_hash: self.content_hash.into(),
                description: Default::default(),
                filename: self.filename,
            },
        };

        let blurhash = self.image_data.map(|data| NestedPart {
            disposition: Disposition::Preview,
            language: String::new(),
            part: NestedPartContent::SinglePart {
                content_type: "text/blurhash".to_owned(),
                content: data.blurhash.into_bytes().into(),
            },
        });

        Ok([Some(attachment), blurhash].into_iter().flatten().collect())
    }
}

/// Metadata of an encrypted and uploaded attachment
struct AttachmentMetadata {
    attachment_id: AttachmentId,
    key: AttachmentEarKey,
    size: u64,
    nonce: [u8; 12],
}

async fn encrypt_and_upload(
    api_client: &ApiClient,
    http_client: &reqwest::Client,
    signing_key: &ClientSigningKey,
    content: &AttachmentBytes,
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
        size,
        nonce,
    })
}
