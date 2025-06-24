// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::convert::Infallible;

use mimi_content::content_container::EncryptionAlgorithm;
use phnxcommon::crypto::{
    ear::{
        Ciphertext, EarDecryptable, EarEncryptable, EarKey, GenericDeserializable,
        GenericSerializable, Payload, keys::AttachmentEarKey,
    },
    errors::{DecryptionError, EncryptionError},
};

use super::AttachmentBytes;

pub(super) const PHNX_ATTACHMENT_ENCRYPTION_ALG: EncryptionAlgorithm =
    EncryptionAlgorithm::Aes256Gcm12;

/// Custom hash algorithm
///
/// Unused value in IANA Hash Algorithm registry
pub(super) const PHNX_BLAKE3_HASH_ID: u8 = 42;

#[derive(Debug, Clone)]
pub struct EncryptedAttachmentCtype;

pub type EncryptedAttachment = Ciphertext<EncryptedAttachmentCtype>;

impl GenericSerializable for AttachmentBytes {
    type Error = Infallible;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        unreachable!("attachment content is encrypted directly")
    }
}

impl EarEncryptable<AttachmentEarKey, EncryptedAttachmentCtype> for AttachmentBytes {
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

impl GenericDeserializable for AttachmentBytes {
    type Error = Infallible;

    fn deserialize(_bytes: &[u8]) -> Result<Self, Self::Error> {
        unreachable!("attachment content is decrypted directly")
    }
}

impl EarDecryptable<AttachmentEarKey, EncryptedAttachmentCtype> for AttachmentBytes {
    fn decrypt(
        ear_key: &AttachmentEarKey,
        ciphertext: &EncryptedAttachment,
    ) -> Result<Self, DecryptionError> {
        let bytes = ear_key.decrypt(ciphertext.aead_ciphertext())?;
        Ok(AttachmentBytes::new(bytes))
    }

    fn decrypt_with_aad<Aad: GenericSerializable>(
        ear_key: &AttachmentEarKey,
        ciphertext: &Ciphertext<EncryptedAttachmentCtype>,
        aad: &Aad,
    ) -> Result<Self, DecryptionError> {
        let aad = aad.serialize().map_err(|e| {
            tracing::error!(error = %e, "Could not serialize aad");
            DecryptionError::SerializationError
        })?;
        let plaintext = ear_key.decrypt_with_aad(ciphertext.aead_ciphertext(), aad.as_slice())?;
        Self::deserialize(&plaintext).map_err(|_| DecryptionError::DeserializationError)
    }
}
