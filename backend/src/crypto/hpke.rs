// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls_traits::types::HpkeCiphertext;

use super::{
    ear::{GenericDeserializable, GenericSerializable},
    DecryptionError, DecryptionPrivateKey, EncryptionPublicKey,
};

// TODO: We might want to properly fix AAD and Info at some point.
pub trait HpkeEncryptable<
    HpkeEncryptionKeyType: HpkeEncryptionKey,
    HpkeCiphertextType: From<HpkeCiphertext>,
>: GenericSerializable
{
    /// Encrypts the data with the given encryption key.
    fn encrypt(
        &self,
        encryption_key: &HpkeEncryptionKeyType,
        info: &[u8],
        aad: &[u8],
    ) -> HpkeCiphertextType {
        // Hiding a LibraryError behind an empty vec.
        let plain_txt = self.serialize().unwrap_or_default();
        encryption_key
            .as_ref()
            .encrypt(info, aad, &plain_txt)
            .into()
    }
}

pub trait HpkeEncryptionKey: AsRef<EncryptionPublicKey> {}

pub trait HpkeDecryptable<
    HpkeDecryptionKeyType: HpkeDecryptionKey,
    HpkeCiphertextType: AsRef<HpkeCiphertext>,
>: GenericDeserializable
{
    fn decrypt(
        ct: HpkeCiphertextType,
        decryption_key: &HpkeDecryptionKeyType,
        info: &[u8],
        aad: &[u8],
    ) -> Result<Self, DecryptionError> {
        let plaintext = decryption_key.as_ref().decrypt(info, aad, ct.as_ref())?;
        Self::deserialize(&plaintext).map_err(|_| DecryptionError::DeserializationError)
    }
}

pub trait HpkeDecryptionKey: AsRef<DecryptionPrivateKey> {}
