// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::HashType,
    openmls_rust_crypto::RustCrypto,
    openmls_traits::{crypto::OpenMlsCrypto, types::HpkeCiphertext},
};

use thiserror::Error;
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};

use serde::{Deserialize, Serialize};

use crate::{
    credentials::{
        AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialPayload,
        CredentialFingerprint,
        keys::{ClientKeyType, ClientSignature},
    },
    crypto::{
        ConnectionEncryptionKey, RatchetEncryptionKey,
        ear::Ciphertext,
        kdf::keys::RatchetSecret,
        signatures::signable::{Signable, SignedStruct, VerifiedStruct},
    },
    time::ExpirationData,
};

use super::{
    MlsInfraVersion,
    client_as_out::{EncryptedUserProfile, VerifiableConnectionPackage},
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackageTbs {
    pub protocol_version: MlsInfraVersion,
    pub encryption_key: ConnectionEncryptionKey,
    pub lifetime: ExpirationData,
    pub client_credential: ClientCredential,
}

impl ConnectionPackageTbs {
    pub fn new(
        protocol_version: MlsInfraVersion,
        encryption_key: ConnectionEncryptionKey,
        lifetime: ExpirationData,
        client_credential: ClientCredential,
    ) -> Self {
        Self {
            protocol_version,
            encryption_key,
            lifetime,
            client_credential,
        }
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes,
)]
#[cfg_attr(any(feature = "test_utils", test), derive(PartialEq))]
#[serde(transparent)]
pub struct ConnectionPackageHash([u8; 32]);

#[derive(Debug, Error)]
pub enum ConnectionPackageHashError {
    #[error("Invalid length: expected 32 bytes, got {actual} bytes")]
    InvalidLength { actual: usize },
}

impl TryFrom<Vec<u8>> for ConnectionPackageHash {
    type Error = ConnectionPackageHashError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let value_len = value.len();
        let array = value
            .try_into()
            .map_err(|_| ConnectionPackageHashError::InvalidLength { actual: value_len })?;
        Ok(Self(array))
    }
}

impl ConnectionPackageHash {
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    #[cfg(feature = "test_utils")]
    pub fn random() -> Self {
        use rand::RngCore;

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackage {
    payload: ConnectionPackageTbs,
    signature: ClientSignature,
}

impl ConnectionPackage {
    pub fn new(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }

    pub fn into_parts(self) -> (ConnectionPackageTbs, ClientSignature) {
        (self.payload, self.signature)
    }

    pub fn client_credential(&self) -> &ClientCredential {
        &self.payload.client_credential
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
    }

    pub fn client_credential_signer_fingerprint(&self) -> &CredentialFingerprint {
        self.payload.client_credential.signer_fingerprint()
    }

    pub fn hash(&self) -> ConnectionPackageHash {
        let rust_crypto = RustCrypto::default();
        let payload = self.tls_serialize_detached().unwrap_or_default();
        debug_assert!(!payload.is_empty());
        let input = [b"Connection Package".to_vec(), payload].concat();
        let value: [u8; 32] = rust_crypto
            .hash(HashType::Sha2_256, &input)
            .unwrap_or_default()
            .try_into()
            // Output length of `hash` is always 32 bytes
            .unwrap();
        debug_assert!(!value.is_empty());
        ConnectionPackageHash(value)
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }
}

impl VerifiedStruct<VerifiableConnectionPackage> for ConnectionPackage {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableConnectionPackage, _seal: Self::SealingType) -> Self {
        Self {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

impl Signable for ConnectionPackageTbs {
    type SignedOutput = ConnectionPackage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ConnectionPackage"
    }
}

impl SignedStruct<ConnectionPackageTbs, ClientKeyType> for ConnectionPackage {
    fn from_payload(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }
}

// === User ===

#[derive(Debug)]
pub struct RegisterUserParams {
    pub client_payload: ClientCredentialPayload,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug)]
pub struct RegisterUserResponse {
    pub client_credential: ClientCredential,
}

// === Client ===

#[derive(Debug)]
pub struct EncryptedFriendshipPackageCtype;
pub type EncryptedFriendshipPackage = Ciphertext<EncryptedFriendshipPackageCtype>;

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct EncryptedConnectionOffer {
    ciphertext: HpkeCiphertext,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct ConnectionOfferMessage {
    connection_package_hash: ConnectionPackageHash,
    ciphertext: EncryptedConnectionOffer,
}

impl ConnectionOfferMessage {
    pub fn new(
        connection_package_hash: ConnectionPackageHash,
        ciphertext: EncryptedConnectionOffer,
    ) -> Self {
        Self {
            connection_package_hash,
            ciphertext,
        }
    }

    pub fn into_parts(self) -> (EncryptedConnectionOffer, ConnectionPackageHash) {
        (self.ciphertext, self.connection_package_hash)
    }
}

impl From<HpkeCiphertext> for EncryptedConnectionOffer {
    fn from(ciphertext: HpkeCiphertext) -> Self {
        Self { ciphertext }
    }
}

impl AsRef<HpkeCiphertext> for EncryptedConnectionOffer {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

// === Anonymous requests ===

#[derive(Debug)]
pub struct AsCredentialsParams {}

#[derive(Debug)]
pub struct AsCredentialsResponse {
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<AsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}
