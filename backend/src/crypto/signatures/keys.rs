use mls_assist::{OpenMlsCrypto, OpenMlsCryptoProvider, OpenMlsRustCrypto, SignaturePublicKey};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::ds::group_state::UserKeyHash;

use super::traits::{SigningKey, VerifyingKey};

#[derive(Clone, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct LeafSignatureKey {
    signature_key: SignaturePublicKey,
}

impl LeafSignatureKey {
    pub fn signature_key(&self) -> &SignaturePublicKey {
        &self.signature_key
    }
}

impl VerifyingKey for LeafSignatureKey {}

impl AsRef<[u8]> for LeafSignatureKey {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

#[derive(Debug)]
pub struct LeafSignatureKeyRef<'a> {
    signature_key: &'a SignaturePublicKey,
}

impl<'a> VerifyingKey for LeafSignatureKeyRef<'a> {}

impl<'a> AsRef<[u8]> for LeafSignatureKeyRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

impl<'a> From<&'a SignaturePublicKey> for LeafSignatureKeyRef<'a> {
    fn from(pk_ref: &'a SignaturePublicKey) -> Self {
        Self {
            signature_key: pk_ref,
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
        todo!()
    }
}

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct OwnerVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for OwnerVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for OwnerVerifyingKey {}

pub struct OwnerSigningKey {
    signing_key: Vec<u8>,
}

#[derive(Debug)]
pub struct RandomnessError {}

impl OwnerSigningKey {
    pub fn random() -> Result<(Self, OwnerVerifyingKey), RandomnessError> {
        let backend = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = backend
            .crypto()
            .signature_key_gen(mls_assist::SignatureScheme::ED25519)
            .map_err(|_| RandomnessError {})?;
        Ok((Self { signing_key }, OwnerVerifyingKey { verifying_key }))
    }
}

impl AsRef<[u8]> for OwnerSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for OwnerSigningKey {}

#[derive(Debug)]
pub struct QsVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsVerifyingKey {}
