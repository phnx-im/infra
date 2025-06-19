// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use anyhow::Context;
use infer::MatcherType;
use mimi_content::{
    MimiContent,
    content_container::{Disposition, HashAlgorithm, NestedPart, NestedPartContent},
};
use phnxcommon::crypto::ear::{AeadCiphertext, EarEncryptable, keys::AttachmentEarKey};
use uuid::Uuid;

use crate::{
    Conversation, ConversationId, ConversationMessage,
    clients::{
        CoreUser,
        attachment::{
            ear::{AttachmentContent, PHNX_ATTACHMENT_ENCRYPTION_ALG, PHNX_BLAKE3_HASH_ID},
            persistence::AttachmentRecord,
        },
    },
    groups::Group,
};

mod ear;
mod image;
mod persistence;

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

        // read the file
        let mime = infer::get_from_path(path)
            .with_context(|| format!("Failed to read file at {}", path.display()))?;
        let content = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read file at {}", path.display()))?;
        let content = AttachmentContent(content);

        // encrypt the file
        let key = AttachmentEarKey::random()?;
        let ciphertext: AeadCiphertext = content.encrypt(&key)?.into();
        let (ciphertext, nonce) = ciphertext.into_parts();
        let size: u64 = ciphertext
            .len()
            .try_into()
            .context("attachment size overflow")?;
        let content_hash = blake3::hash(&ciphertext).as_bytes().to_vec();

        // provision and upload encrypted content
        let api_client = self.api_client()?;
        let response = api_client
            .ds_provision_attachment(
                self.signing_key(),
                group.group_state_ear_key(),
                conversation.group_id(),
                group.own_index(),
            )
            .await?;
        let attachment_id: Uuid = response.attachment_id.context("no attachment id")?.into();

        let mut request = self.http_client().put(response.upload_url);
        for header in response.upload_headers {
            request = request.header(header.key, header.value);
        }
        request.body(ciphertext).send().await?.error_for_status()?;

        // store attachment locally

        // Note: Acquire a transaction here to ensure that the attachment will be deleted from the
        // local database in case of an error.
        let mut txn = self.pool().begin().await?;
        let mut notifier = self.store_notifier();

        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "attachment".to_owned());
        let content_type = mime
            .map(|mime| mime.mime_type())
            .unwrap_or("application/octet-stream");

        let record = AttachmentRecord {
            attachment_id,
            conversation_id,
            content_type: content_type.to_owned(),
            content: content.0,
        };
        record.store(txn.as_mut()).await?;

        if mime
            .map(|mime| mime.matcher_type() == MatcherType::Image)
            .unwrap_or(false)
        {
            let record = record.calculate_image_record().await?;
            record.store(txn.as_mut()).await?;
        }

        // send attachment message
        let content = MimiContent {
            nested_part: NestedPart {
                disposition: Disposition::Attachment,
                part: NestedPartContent::MultiPart {
                    part_semantics: mimi_content::content_container::PartSemantics::SingleUnit,
                    parts: vec![
                        NestedPart {
                            disposition: Disposition::Render,
                            language: String::new(),
                            part: NestedPartContent::SinglePart {
                                content_type: "text/markdown".to_owned(),
                                // Will become an optional message
                                content: filename.clone().into_bytes().into(),
                            },
                        },
                        NestedPart {
                            disposition: Disposition::Attachment,
                            language: String::new(),
                            part: NestedPartContent::ExternalPart {
                                content_type: content_type.to_owned(),
                                url: format!("phnx://attachment/{attachment_id}"),
                                expires: 0,
                                size,
                                enc_alg: PHNX_ATTACHMENT_ENCRYPTION_ALG,
                                key: key.into_bytes().to_vec().into(),
                                nonce: nonce.to_vec().into(),
                                aad: Default::default(),
                                hash_alg: HashAlgorithm::Custom(PHNX_BLAKE3_HASH_ID),
                                content_hash: content_hash.into(),
                                description: String::new(),
                                filename,
                            },
                        },
                    ],
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let message = self
            .send_message_transactional(&mut txn, &mut notifier, conversation_id, content)
            .await?;

        txn.commit().await?;
        notifier.notify();

        Ok(message)
    }
}
