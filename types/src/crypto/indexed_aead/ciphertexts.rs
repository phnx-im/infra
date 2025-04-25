// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::crypto::{
    ear::{Ciphertext, EarDecryptable, EarEncryptable},
    errors::{DecryptionError, EncryptionError},
};

use super::keys::{Index, IndexedAeadKey, IndexedKeyType};

pub struct IndexedCiphertext<KT, CT> {
    key_index: Index<KT>,
    ciphertext: Ciphertext<CT>,
}

pub trait IndexEncryptable<KT: IndexedKeyType + Clone, CT>:
    EarEncryptable<IndexedAeadKey<KT>, CT>
{
    fn encrypt_with_index(
        &self,
        key: &IndexedAeadKey<KT>,
    ) -> Result<IndexedCiphertext<KT, CT>, IndexEncryptionError> {
        let ciphertext = self.encrypt(key)?;
        let indexed_ciphertext = IndexedCiphertext {
            key_index: key.index().clone(),
            ciphertext,
        };
        Ok(indexed_ciphertext)
    }
}

pub trait IndexDecryptable<KT: IndexedKeyType + Clone + PartialEq, CT>:
    EarDecryptable<IndexedAeadKey<KT>, CT>
{
    fn decrypt_with_index(
        key: &IndexedAeadKey<KT>,
        ciphertext: &IndexedCiphertext<KT, CT>,
    ) -> Result<Self, IndexDecryptionError> {
        if &ciphertext.key_index != key.index() {
            return Err(IndexDecryptionError::InvalidKeyIndex);
        }
        let plaintext = Self::decrypt(key, &ciphertext.ciphertext)?;
        Ok(plaintext)
    }
}

#[derive(Error, Debug)]
pub enum IndexEncryptionError {
    /// Encryption error
    #[error(transparent)]
    EncryptionError(#[from] EncryptionError),
    /// Invalid key index
    #[error("Invalid key index")]
    InvalidKeyIndex,
}

#[derive(Error, Debug)]
pub enum IndexDecryptionError {
    /// Encryption error
    #[error(transparent)]
    DecryptionError(#[from] DecryptionError),
    /// Invalid key index
    #[error("Invalid key index")]
    InvalidKeyIndex,
}
