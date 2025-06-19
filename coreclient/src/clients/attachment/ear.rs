// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::convert::Infallible;

use mimi_content::content_container::EncryptionAlgorithm;
use phnxcommon::crypto::{
    ear::{
        Ciphertext, EarEncryptable, EarKey, GenericSerializable, Payload, keys::AttachmentEarKey,
    },
    errors::EncryptionError,
};

use super::AttachmentContent;

pub(super) const PHNX_ATTACHMENT_ENCRYPTION_ALG: EncryptionAlgorithm =
    EncryptionAlgorithm::Aes256Gcm;

/// Custom hash algorithm
///
///Unused value in IANA Hash Algorithm registry
pub(super) const PHNX_BLAKE3_HASH_ID: u8 = 42;

#[derive(Debug, Clone)]
pub struct EncryptedAttachmentCtype;

pub type EncryptedAttachment = Ciphertext<EncryptedAttachmentCtype>;

impl GenericSerializable for AttachmentContent {
    type Error = Infallible;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        unreachable!("attachment content is encrypted directly")
    }
}

impl EarEncryptable<AttachmentEarKey, EncryptedAttachmentCtype> for AttachmentContent {
    fn encrypt(&self, key: &AttachmentEarKey) -> Result<EncryptedAttachment, EncryptionError> {
        Ok(key.encrypt(self.as_ref())?.into())
    }

    fn encrypt_with_aad<Aad: GenericSerializable>(
        &self,
        key: &AttachmentEarKey,
        aad: &Aad,
    ) -> Result<EncryptedAttachment, EncryptionError> {
        let aad = aad
            .serialize()
            .map_err(|_| EncryptionError::SerializationError)?;
        let payload = Payload {
            msg: self.as_ref(),
            aad: aad.as_slice(),
        };
        Ok(key.encrypt(payload)?.into())
    }
}
