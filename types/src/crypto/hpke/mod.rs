// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::{
        key_packages::InitKey,
        prelude::{HpkeAeadType, HpkeConfig, HpkeKdfType, HpkeKemType, HpkePublicKey},
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{
        OpenMlsProvider,
        crypto::OpenMlsCrypto,
        random::OpenMlsRand,
        types::{HpkeCiphertext, HpkePrivateKey},
    },
};
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};
use tracing::error;

use crate::identifiers::{ClientConfig, SealedClientReference};

use super::{
    ear::{GenericDeserializable, GenericSerializable},
    errors::{DecryptionError, EncryptionError, RandomnessError},
    secrets::SecretBytes,
};

pub mod sqlx_impls;

#[derive(
    Clone, PartialEq, Eq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[serde(transparent)]
pub struct EncryptionKey<KT> {
    key: Vec<u8>,
    _type: std::marker::PhantomData<KT>,
}

impl<KT> From<Vec<u8>> for EncryptionKey<KT> {
    fn from(value: Vec<u8>) -> Self {
        Self {
            key: value,
            _type: std::marker::PhantomData,
        }
    }
}

pub const HPKE_CONFIG: HpkeConfig = HpkeConfig(
    HpkeKemType::DhKem25519,
    HpkeKdfType::HkdfSha256,
    HpkeAeadType::AesGcm256,
);

impl<KT> EncryptionKey<KT> {
    /// Encrypt the given plaintext using this key.
    pub(crate) fn encrypt(&self, info: &[u8], aad: &[u8], plain_txt: &[u8]) -> HpkeCiphertext {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .hpke_seal(HPKE_CONFIG, &self.key, info, aad, plain_txt)
            // TODO: get rid of unwrap
            .unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionKey<KT> {
    decryption_key: SecretBytes,
    encryption_key: EncryptionKey<KT>,
}

#[cfg(any(test, feature = "test_utils"))]
impl<KT: PartialEq> PartialEq for DecryptionKey<KT> {
    fn eq(&self, other: &Self) -> bool {
        let decryption_key: &[u8] = self.decryption_key.as_ref();
        let other_decryption_key: &[u8] = other.decryption_key.as_ref();
        decryption_key == other_decryption_key && self.encryption_key == other.encryption_key
    }
}

#[cfg(any(test, feature = "test_utils"))]
impl<KT: Eq> Eq for DecryptionKey<KT> {}

impl<KT> DecryptionKey<KT> {
    pub fn new(decryption_key: HpkePrivateKey, encryption_key: EncryptionKey<KT>) -> Self {
        Self {
            decryption_key: decryption_key.as_ref().to_vec().into(),
            encryption_key,
        }
    }

    pub fn decrypt(
        &self,
        info: &[u8],
        aad: &[u8],
        ct: &HpkeCiphertext,
    ) -> Result<Vec<u8>, DecryptionError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .hpke_open(HPKE_CONFIG, ct, &self.decryption_key, info, aad)
            .map_err(|_| DecryptionError::DecryptionError)
    }

    pub fn generate() -> Result<Self, RandomnessError> {
        let provider = OpenMlsRustCrypto::default();
        let key_seed = provider
            .rand()
            .random_array::<32>()
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let keypair = provider
            .crypto()
            .derive_hpke_keypair(HPKE_CONFIG, &key_seed)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self::new(keypair.private, keypair.public.into()))
    }

    pub fn encryption_key(&self) -> &EncryptionKey<KT> {
        &self.encryption_key
    }
}

// TODO: We might want to properly fix AAD and Info at some point.
pub trait HpkeEncryptable<KT, HpkeCiphertextType: From<HpkeCiphertext>>:
    GenericSerializable
{
    /// Encrypts the data with the given encryption key.
    fn encrypt(
        &self,
        encryption_key: &EncryptionKey<KT>,
        info: &[u8],
        aad: &[u8],
    ) -> HpkeCiphertextType {
        // Hiding a LibraryError behind an empty vec.
        let plain_txt = self.serialize().unwrap_or_default();
        encryption_key.encrypt(info, aad, &plain_txt).into()
    }
}

pub trait HpkeDecryptable<KT, HpkeCiphertextType: AsRef<HpkeCiphertext>>:
    GenericDeserializable
{
    fn decrypt(
        ct: HpkeCiphertextType,
        decryption_key: &DecryptionKey<KT>,
        info: &[u8],
        aad: &[u8],
    ) -> Result<Self, DecryptionError> {
        let plaintext = decryption_key.decrypt(info, aad, ct.as_ref())?;
        Self::deserialize(&plaintext).map_err(|e| {
            error!(%e, "Error deserializing decrypted data");
            DecryptionError::DeserializationError
        })
    }
}

pub struct JoinerInfoKeyType;
pub type JoinerInfoEncryptionKey = EncryptionKey<JoinerInfoKeyType>;

// We need this From trait, because we have to work with the hpke init key from
// the KeyPackage, which we get as HpkePublicKey from OpenMLS.
impl From<HpkePublicKey> for JoinerInfoEncryptionKey {
    fn from(value: HpkePublicKey) -> Self {
        value.as_slice().to_vec().into()
    }
}

impl From<InitKey> for JoinerInfoEncryptionKey {
    fn from(value: InitKey) -> Self {
        value.as_slice().to_vec().into()
    }
}

pub type JoinerInfoDecryptionKey = DecryptionKey<JoinerInfoKeyType>;

// We need this From trait, because we have to work with the hpke init key from
// the KeyPackage, which we get as HpkePrivateKey and HpkePublicKey from
// OpenMLS.
impl From<(HpkePrivateKey, InitKey)> for JoinerInfoDecryptionKey {
    fn from((sk, init_key): (HpkePrivateKey, InitKey)) -> Self {
        let vec: Vec<u8> = init_key.key().as_slice().to_vec();
        DecryptionKey::new(sk, vec.into())
    }
}

#[derive(
    Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize, PartialEq, Eq,
)]
pub struct ClientIdKeyType;
pub type ClientIdEncryptionKey = EncryptionKey<ClientIdKeyType>;

impl ClientIdEncryptionKey {
    pub fn seal_client_config(
        &self,
        client_config: ClientConfig,
    ) -> Result<SealedClientReference, EncryptionError> {
        let bytes = client_config
            .tls_serialize_detached()
            .map_err(|_| EncryptionError::SerializationError)?;
        let ciphertext = self.encrypt(&[], &[], &bytes);
        Ok(SealedClientReference { ciphertext })
    }
}

pub type ClientIdDecryptionKey = DecryptionKey<ClientIdKeyType>;
