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
        crypto::OpenMlsCrypto,
        random::OpenMlsRand,
        types::{HpkeCiphertext, HpkePrivateKey},
        OpenMlsProvider,
    },
};
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::identifiers::{ClientConfig, SealedClientReference};

use super::{
    ear::{GenericDeserializable, GenericSerializable},
    errors::{DecryptionError, EncryptionError, RandomnessError},
    secrets::SecretBytes,
};

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct EncryptionPublicKey(Vec<u8>);

impl From<Vec<u8>> for EncryptionPublicKey {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

pub const HPKE_CONFIG: HpkeConfig = HpkeConfig(
    HpkeKemType::DhKem25519,
    HpkeKdfType::HkdfSha256,
    HpkeAeadType::AesGcm256,
);

impl EncryptionPublicKey {
    /// Encrypt the given plaintext using this key.
    pub(crate) fn encrypt(&self, info: &[u8], aad: &[u8], plain_txt: &[u8]) -> HpkeCiphertext {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .hpke_seal(HPKE_CONFIG, &self.0, info, aad, plain_txt)
            // TODO: get rid of unwrap
            .unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "decryption_key_data")
)]
pub struct DecryptionKey {
    decryption_key: SecretBytes,
    encryption_key: EncryptionPublicKey,
}

impl DecryptionKey {
    pub fn new(decryption_key: HpkePrivateKey, encryption_key: EncryptionPublicKey) -> Self {
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
        Ok(Self::new(
            keypair.private,
            EncryptionPublicKey(keypair.public),
        ))
    }

    pub fn public_key(&self) -> &EncryptionPublicKey {
        &self.encryption_key
    }
}

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

pub trait HpkeDecryptionKey: AsRef<DecryptionKey> {}

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
    decryption_key: DecryptionKey,
}

impl JoinerInfoDecryptionKey {
    pub fn public_key(&self) -> JoinerInfoEncryptionKey {
        let encryption_key: HpkePublicKey = self.decryption_key.encryption_key.0.clone().into();
        encryption_key.into()
    }
}

impl HpkeDecryptionKey for JoinerInfoDecryptionKey {}

impl AsRef<DecryptionKey> for JoinerInfoDecryptionKey {
    fn as_ref(&self) -> &DecryptionKey {
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
            decryption_key: DecryptionKey::new(sk, vec.into()),
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
    ) -> Result<SealedClientReference, EncryptionError> {
        let bytes = client_config
            .tls_serialize_detached()
            .map_err(|_| EncryptionError::SerializationError)?;
        let ciphertext = self.public_key.encrypt(&[], &[], &bytes);
        Ok(SealedClientReference { ciphertext })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[serde(transparent)]
pub struct ClientIdDecryptionKey(DecryptionKey);

impl AsRef<DecryptionKey> for ClientIdDecryptionKey {
    fn as_ref(&self) -> &DecryptionKey {
        &self.0
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
        let private_key = DecryptionKey::generate()?;
        Ok(Self(private_key))
    }

    pub fn encryption_key(&self) -> ClientIdEncryptionKey {
        ClientIdEncryptionKey {
            public_key: self.0.encryption_key.clone(),
        }
    }
}
