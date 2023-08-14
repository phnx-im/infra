// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{HashType, OpenMlsCrypto, OpenMlsProvider, SignaturePublicKey},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    auth_service::credentials::keys::{generate_signature_keypair, KeyGenerationError},
    ds::group_state::UserKeyHash,
};

use super::traits::{SigningKey, VerifyingKey};

#[derive(Debug)]
pub struct LeafVerifyingKeyRef<'a> {
    verifying_key: &'a SignaturePublicKey,
}

impl<'a> VerifyingKey for LeafVerifyingKeyRef<'a> {}

impl<'a> AsRef<[u8]> for LeafVerifyingKeyRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key.as_slice()
    }
}

impl<'a> From<&'a SignaturePublicKey> for LeafVerifyingKeyRef<'a> {
    fn from(pk_ref: &'a SignaturePublicKey) -> Self {
        Self {
            verifying_key: pk_ref,
        }
    }
}

/// Public signature key known to all clients of a given user. This signature
/// key is used by pseudomnymous clients to prove they belong to a certain
/// pseudonymous user account.
#[derive(
    Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone,
)]
pub struct UserAuthVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for UserAuthVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for UserAuthVerifyingKey {}

impl UserAuthVerifyingKey {
    pub fn hash(&self) -> UserKeyHash {
        let hash = OpenMlsRustCrypto::default()
            .crypto()
            .hash(HashType::Sha2_256, &self.verifying_key)
            .unwrap_or_default();
        UserKeyHash::new(hash)
    }
}

#[derive(Debug)]
pub struct UserAuthSigningKey {
    signing_key: Vec<u8>,
    verifying_key: UserAuthVerifyingKey,
}

impl UserAuthSigningKey {
    pub fn verifying_key(&self) -> &UserAuthVerifyingKey {
        &self.verifying_key
    }

    pub fn generate() -> Result<Self, KeyGenerationError> {
        let keypair = generate_signature_keypair()?;
        let verifying_key = UserAuthVerifyingKey {
            verifying_key: keypair.1,
        };
        Ok(Self {
            signing_key: keypair.0,
            verifying_key,
        })
    }
}

impl AsRef<[u8]> for UserAuthSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for UserAuthSigningKey {}
impl SigningKey for &UserAuthSigningKey {}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
)]
pub struct QsClientVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsClientVerifyingKey {}

pub struct QsClientSigningKey {
    signing_key: Vec<u8>,
    verifying_key: QsClientVerifyingKey,
}

#[derive(Debug)]
pub struct RandomnessError {}

impl QsClientSigningKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = rust_crypto
            .crypto()
            .signature_key_gen(mls_assist::openmls::prelude::SignatureScheme::ED25519)
            .map_err(|_| RandomnessError {})?;
        Ok(Self {
            signing_key,
            verifying_key: QsClientVerifyingKey { verifying_key },
        })
    }

    pub fn verifying_key(&self) -> &QsClientVerifyingKey {
        &self.verifying_key
    }
}

impl AsRef<[u8]> for QsClientSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for QsClientSigningKey {}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
)]
pub struct QsUserVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsUserVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsUserVerifyingKey {}

pub struct QsUserSigningKey {
    signing_key: Vec<u8>,
    verifying_key: QsUserVerifyingKey,
}

impl QsUserSigningKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = rust_crypto
            .crypto()
            .signature_key_gen(mls_assist::openmls::prelude::SignatureScheme::ED25519)
            .map_err(|_| RandomnessError {})?;
        Ok(Self {
            signing_key,
            verifying_key: QsUserVerifyingKey { verifying_key },
        })
    }

    pub fn verifying_key(&self) -> &QsUserVerifyingKey {
        &self.verifying_key
    }
}

impl AsRef<[u8]> for QsUserSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for QsUserSigningKey {}
