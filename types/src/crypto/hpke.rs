// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::{key_packages::InitKey, prelude::HpkePublicKey},
    openmls_traits::types::{HpkeCiphertext, HpkePrivateKey},
};
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::identifiers::{ClientConfig, SealedClientReference};

use super::{
    ear::{GenericDeserializable, GenericSerializable},
    errors::{RandomnessError, SealError},
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

impl From<InitKey> for JoinerInfoEncryptionKey {
    fn from(value: InitKey) -> Self {
        Self {
            encryption_key: EncryptionPublicKey::from(value.key().as_slice().to_vec()),
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
impl From<(HpkePrivateKey, InitKey)> for JoinerInfoDecryptionKey {
    fn from((sk, init_key): (HpkePrivateKey, InitKey)) -> Self {
        let vec: Vec<u8> = init_key.key().as_slice().to_vec();
        Self {
            decryption_key: DecryptionPrivateKey::new(sk, vec.into()),
        }
    }
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ClientIdEncryptionKey {
    public_key: EncryptionPublicKey,
}

impl AsRef<EncryptionPublicKey> for ClientIdEncryptionKey {
    fn as_ref(&self) -> &EncryptionPublicKey {
        &self.public_key
    }
}

impl HpkeEncryptionKey for ClientIdEncryptionKey {}

impl ClientIdEncryptionKey {
    pub fn seal_client_config(
        &self,
        client_config: ClientConfig,
    ) -> Result<SealedClientReference, SealError> {
        let bytes = client_config
            .tls_serialize_detached()
            .map_err(|_| SealError::CodecError)?;
        let ciphertext = self.public_key.encrypt(&[], &[], &bytes);
        Ok(SealedClientReference { ciphertext })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdDecryptionKey {
    private_key: DecryptionPrivateKey,
    encryption_key: ClientIdEncryptionKey,
}

impl AsRef<DecryptionPrivateKey> for ClientIdDecryptionKey {
    fn as_ref(&self) -> &DecryptionPrivateKey {
        &self.private_key
    }
}

impl HpkeDecryptionKey for ClientIdDecryptionKey {}

impl ClientIdDecryptionKey {
    //pub(super) fn unseal_client_config(
    //    &self,
    //    sealed_client_reference: &SealedClientReference,
    //) -> Result<ClientConfig, UnsealError> {
    //    let bytes = self
    //        .private_key
    //        .decrypt(&[], &[], &sealed_client_reference.ciphertext)
    //        .map_err(|_| UnsealError::DecryptionError)?;
    //    ClientConfig::tls_deserialize_exact_bytes(&bytes).map_err(|_| UnsealError::CodecError)
    //}

    pub fn generate() -> Result<Self, RandomnessError> {
        let private_key = DecryptionPrivateKey::generate()?;
        let encryption_key = ClientIdEncryptionKey {
            public_key: private_key.public_key().clone(),
        };
        Ok(Self {
            private_key,
            encryption_key,
        })
    }

    pub fn encryption_key(&self) -> &ClientIdEncryptionKey {
        &self.encryption_key
    }
}
