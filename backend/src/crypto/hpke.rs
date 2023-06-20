// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude_test::HpkePublicKey,
    openmls_traits::types::{HpkeCiphertext, HpkePrivateKey},
};

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

pub struct JoinerInfoEncryptionKey {
    encryption_key: EncryptionPublicKey,
}

// We need this From trait, because we have to work with the hpke init key from
// the KeyPackage, which we get as HpkePublicKey from OpenMLS.
impl From<HpkePublicKey> for JoinerInfoEncryptionKey {
    fn from(value: HpkePublicKey) -> Self {
        Self {
            encryption_key: EncryptionPublicKey::from(value.as_slice().to_vec()),
        }
    }
}

impl AsRef<EncryptionPublicKey> for JoinerInfoEncryptionKey {
    fn as_ref(&self) -> &EncryptionPublicKey {
        &self.encryption_key
    }
}

impl HpkeEncryptionKey for JoinerInfoEncryptionKey {}

pub struct JoinerInfoDecryptionKey {
    decryption_key: DecryptionPrivateKey,
}

impl JoinerInfoDecryptionKey {
    pub fn public_key(&self) -> JoinerInfoEncryptionKey {
        let encryption_key: HpkePublicKey =
            self.decryption_key.public_key.public_key.clone().into();
        encryption_key.into()
    }
}

impl HpkeDecryptionKey for JoinerInfoDecryptionKey {}

impl AsRef<DecryptionPrivateKey> for JoinerInfoDecryptionKey {
    fn as_ref(&self) -> &DecryptionPrivateKey {
        &self.decryption_key
    }
}

// We need this From trait, because we have to work with the hpke init key from
// the KeyPackage, which we get as HpkePrivateKey and HpkePublicKey from
// OpenMLS.
impl From<(HpkePrivateKey, HpkePublicKey)> for JoinerInfoDecryptionKey {
    fn from((sk, pk): (HpkePrivateKey, HpkePublicKey)) -> Self {
        let vec: Vec<u8> = pk.into();
        Self {
            decryption_key: DecryptionPrivateKey::new(sk, vec.into()),
        }
    }
}
