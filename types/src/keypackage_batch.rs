// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::{
        prelude::{KeyPackage, KeyPackageIn, KeyPackageRef, KeyPackageVerifyError},
        versions::ProtocolVersion,
    },
    openmls_traits::crypto::OpenMlsCrypto,
};

use crate::{
    credentials::EncryptedClientCredential,
    crypto::{
        ear::{
            keys::{AddPackageEarKey, EncryptedSignatureEarKey},
            Ciphertext, EarDecryptable, EarEncryptable,
        },
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
    },
    identifiers::Fqdn,
    time::TimeStamp,
};

use super::*;

// This is used to check keypackage batch freshness by the DS, so it's
// reasonable to assume the batch is relatively fresh.
pub const KEYPACKAGEBATCH_EXPIRATION_DAYS: i64 = 1;

#[derive(Clone, Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct KeyPackageBatch<const IS_VERIFIED: bool> {
    payload: KeyPackageBatchTbs,
    signature: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct KeyPackageBatchTbs {
    homeserver_domain: Fqdn,
    key_package_refs: Vec<KeyPackageRef>,
    time_of_signature: TimeStamp,
}

impl KeyPackageBatchTbs {
    pub fn new(
        homeserver_domain: Fqdn,
        key_package_refs: Vec<KeyPackageRef>,
        time_of_signature: TimeStamp,
    ) -> Self {
        Self {
            homeserver_domain,
            key_package_refs,
            time_of_signature,
        }
    }
}

impl Signable for KeyPackageBatchTbs {
    type SignedOutput = KeyPackageBatch<VERIFIED>;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "KeyPackageBatch"
    }
}

impl SignedStruct<KeyPackageBatchTbs> for KeyPackageBatch<VERIFIED> {
    fn from_payload(payload: KeyPackageBatchTbs, signature: Signature) -> Self {
        KeyPackageBatch { payload, signature }
    }
}

pub const VERIFIED: bool = true;
pub const UNVERIFIED: bool = false;

impl KeyPackageBatch<UNVERIFIED> {
    pub fn homeserver_domain(&self) -> &Fqdn {
        &self.payload.homeserver_domain
    }
}

impl Verifiable for KeyPackageBatch<UNVERIFIED> {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "KeyPackageBatch"
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

impl VerifiedStruct<KeyPackageBatch<UNVERIFIED>> for KeyPackageBatch<VERIFIED> {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: KeyPackageBatch<UNVERIFIED>, _seal: Self::SealingType) -> Self {
        KeyPackageBatch::<VERIFIED> {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

impl KeyPackageBatch<VERIFIED> {
    pub fn key_package_refs(&self) -> &[KeyPackageRef] {
        &self.payload.key_package_refs
    }

    pub fn has_expired(&self, expiration_days: i64) -> bool {
        self.payload.time_of_signature.has_expired(expiration_days)
    }
}

#[derive(Debug, Serialize, Deserialize, TlsSerialize, TlsSize)]
pub struct AddPackage {
    key_package: KeyPackage,
    encrypted_signature_ear_key: EncryptedSignatureEarKey,
    encrypted_client_credential: EncryptedClientCredential,
}

impl AddPackage {
    pub fn new(
        key_package: KeyPackage,
        encrypted_signature_ear_key: EncryptedSignatureEarKey,
        encrypted_client_credential: EncryptedClientCredential,
    ) -> Self {
        Self {
            key_package,
            encrypted_signature_ear_key,
            encrypted_client_credential,
        }
    }

    pub fn key_package(&self) -> &KeyPackage {
        &self.key_package
    }

    pub fn encrypted_signature_ear_key(&self) -> &EncryptedSignatureEarKey {
        &self.encrypted_signature_ear_key
    }
}

#[derive(Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct AddPackageIn {
    key_package: KeyPackageIn,
    encrypted_signature_ear_key: EncryptedSignatureEarKey,
    encrypted_client_credential: EncryptedClientCredential,
}

impl AddPackageIn {
    pub fn validate(
        self,
        crypto: &impl OpenMlsCrypto,
        protocol_version: ProtocolVersion,
    ) -> Result<AddPackage, KeyPackageVerifyError> {
        let key_package = self.key_package.validate(crypto, protocol_version)?;
        Ok(AddPackage {
            key_package,
            encrypted_signature_ear_key: self.encrypted_signature_ear_key,
            encrypted_client_credential: self.encrypted_client_credential,
        })
    }
}

/// Ciphertext that contains a KeyPackage and an intermediary client certficate.
/// TODO: do we want a key committing scheme here?
#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
pub struct QsEncryptedAddPackage {
    ctxt: Ciphertext,
}

impl AsRef<Ciphertext> for QsEncryptedAddPackage {
    fn as_ref(&self) -> &Ciphertext {
        &self.ctxt
    }
}

impl From<Ciphertext> for QsEncryptedAddPackage {
    fn from(ctxt: Ciphertext) -> Self {
        Self { ctxt }
    }
}

impl EarDecryptable<AddPackageEarKey, QsEncryptedAddPackage> for AddPackageIn {}
impl EarEncryptable<AddPackageEarKey, QsEncryptedAddPackage> for AddPackage {}
