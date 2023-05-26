// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{HashType, OpenMlsCrypto, OpenMlsCryptoProvider, SignaturePublicKey},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::ds::group_state::UserKeyHash;

use super::traits::{SigningKey, VerifyingKey};

#[derive(Clone, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct LeafVerifyingKey {
    verifying_key: SignaturePublicKey,
}

impl LeafVerifyingKey {
    pub fn verifying_key(&self) -> &SignaturePublicKey {
        &self.verifying_key
    }
}

impl VerifyingKey for LeafVerifyingKey {}

impl AsRef<[u8]> for LeafVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key.as_slice()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct LeafSigningKey {
    signing_key: Vec<u8>,
    verifying_key: LeafVerifyingKey,
}

impl LeafSigningKey {
    pub fn verifying_key(&self) -> &LeafVerifyingKey {
        &self.verifying_key
    }
}

impl SigningKey for LeafSigningKey {}

impl AsRef<[u8]> for LeafSigningKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key.as_ref()
    }
}

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
#[derive(Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct UserAuthKey {
    signature_key: Vec<u8>,
}

impl AsRef<[u8]> for UserAuthKey {
    fn as_ref(&self) -> &[u8] {
        &self.signature_key
    }
}

impl VerifyingKey for UserAuthKey {}

impl UserAuthKey {
    pub fn hash(&self) -> UserKeyHash {
        let hash = OpenMlsRustCrypto::default()
            .crypto()
            .hash(HashType::Sha2_256, &self.signature_key)
            .unwrap_or_default();
        UserKeyHash::new(hash)
    }
}

pub struct UserAuthSigningKey {
    signing_key: Vec<u8>,
    verifying_key: UserAuthKey,
}

impl UserAuthSigningKey {
    pub fn verifying_key(&self) -> &UserAuthKey {
        &self.verifying_key
    }
}

impl AsRef<[u8]> for UserAuthSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for UserAuthSigningKey {}

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
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
        let backend = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = backend
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

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
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
        let backend = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = backend
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
