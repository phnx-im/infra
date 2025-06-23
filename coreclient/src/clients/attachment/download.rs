// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, anyhow, bail};
use mimi_content::content_container::{EncryptionAlgorithm, HashAlgorithm};
use phnxcommon::{
    crypto::ear::{AeadCiphertext, EarDecryptable, keys::AttachmentEarKey},
    identifiers::AttachmentId,
};
use tracing::{debug, info};

use crate::{
    clients::{
        CoreUser,
        attachment::{
            AttachmentContent, AttachmentRecord, PHNX_BLAKE3_HASH_ID,
            ear::EncryptedAttachment,
            persistence::{AttachmentStatus, PendingAttachmentRecord},
        },
    },
    groups::Group,
};

impl CoreUser {
    pub(crate) async fn download_attachment(
        &self,
        attachment_id: AttachmentId,
    ) -> anyhow::Result<()> {
        info!(?attachment_id, "downloading attachment");

        // Load the pending attachment record and update the status to `Downloading`.
        let Some((pending_record, group)) = self
            .with_transaction(async |txn| {
                let Some(pending_record) =
                    PendingAttachmentRecord::load_pending(txn.as_mut(), attachment_id).await?
                else {
                    debug!(
                        ?attachment_id,
                        "Skipping downloading non-pending attachment"
                    );
                    return Ok(None);
                };
                let record = AttachmentRecord::load(txn.as_mut(), attachment_id)
                    .await?
                    .context("attachment record not found")?;
                let conversation_id = record.conversation_id;
                let group = Group::load_with_conversation_id_clean(txn, conversation_id)
                    .await?
                    .context("group not found")?;

                AttachmentRecord::update_status(
                    txn.as_mut(),
                    attachment_id,
                    AttachmentStatus::Downloading,
                )
                .await?;

                Ok(Some((pending_record, group)))
            })
            .await?
        else {
            return Ok(());
        };

        // Check encryption parameters
        debug!(?attachment_id, "Checking encryption parameters");
        match pending_record.enc_alg {
            EncryptionAlgorithm::Aes256Gcm => (),
            other => bail!("unsupported encryption algorithm: {other:?}"),
        };
        let nonce: [u8; 12] = pending_record
            .nonce
            .try_into()
            .map_err(|_| anyhow!("invalid nonce length"))?;
        let key = AttachmentEarKey::from_bytes(
            pending_record
                .enc_key
                .try_into()
                .map_err(|_| anyhow!("invalid key length"))?,
        );
        match pending_record.hash_alg {
            HashAlgorithm::Custom(value) if value == PHNX_BLAKE3_HASH_ID => (),
            other => bail!("unsupported hash algorithm: {other:?}"),
        };

        // TODO: Retries and marking as failed

        // Get the download URL from DS
        let api_client = self.api_client()?;
        let download_url = api_client
            .ds_get_attachment_url(
                self.signing_key(),
                group.group_state_ear_key(),
                group.group_id(),
                group.own_index(),
                attachment_id,
            )
            .await?;
        debug!(?attachment_id, %download_url, "Got download URL from DS");

        // Download the attachment
        debug!(?attachment_id, "Downloading attachment");
        let response = self
            .http_client()
            .get(download_url)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;

        // Verify hash
        debug!(?attachment_id, "Verifying hash");
        let hash = blake3::hash(&bytes);
        if hash.as_bytes().as_slice() != pending_record.hash {
            bail!("hash mismatch");
        }

        // Decrypt the attachment
        debug!(?attachment_id, "Decrypting attachment");
        let ciphertext = EncryptedAttachment::from(AeadCiphertext::new(bytes.into(), nonce));
        let content: AttachmentContent = AttachmentContent::decrypt(&key, &ciphertext)?;

        // Store the attachment and mark it as downloaded
        self.with_transaction_and_notifier(async move |txn, notifier| {
            AttachmentRecord::mark_as_ready(
                txn.as_mut(),
                notifier,
                attachment_id,
                content.as_ref(),
            )
            .await?;
            PendingAttachmentRecord::delete(txn.as_mut(), attachment_id).await?;
            Ok(())
        })
        .await?;

        Ok(())
    }
}
