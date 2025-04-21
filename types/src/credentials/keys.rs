// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{Lifetime, OpenMlsProvider, SignatureScheme},
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{
        random::OpenMlsRand,
        signatures::{Signer, SignerError},
    },
};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use tracing::error;

use crate::codec::PhnxCodec;

use super::{
    AsCredential, AsIntermediateCredential,
    pseudonymous_credentials::{
        IdentityLinkCtxt, PseudonymousCredential, PseudonymousCredentialTbs,
    },
};

use crate::crypto::{
    ear::{EarEncryptable, keys::IdentityLinkKey},
    errors::{EncryptionError, KeyGenerationError, RandomnessError},
    kdf::{KdfDerivable, keys::ConnectionKey},
    signatures::{
        DEFAULT_SIGNATURE_SCHEME,
        private_keys::{SigningKey, VerifyingKey},
        signable::Signable,
        traits::{SigningKeyBehaviour, VerifyingKeyBehaviour},
    },
};

use thiserror::Error;

use super::ClientCredential;

#[derive(Clone, Serialize, Deserialize)]
pub struct AsIntermediateSigningKey {
    signing_key: SigningKey,
    credential: AsIntermediateCredential,
}

impl AsRef<SigningKey> for AsIntermediateSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl SigningKeyBehaviour for AsIntermediateSigningKey {}

impl AsIntermediateSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryAsIntermediateSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key.into_signing_key(),
            credential,
        })
    }

    pub fn credential(&self) -> &AsIntermediateCredential {
        &self.credential
    }

    pub fn into_credential(self) -> AsIntermediateCredential {
        self.credential
    }
}

#[derive(Debug, Error)]
pub enum SigningKeyCreationError {
    #[error("Public key mismatch")]
    PublicKeyMismatch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AsSigningKey {
    signing_key: SigningKey,
    credential: AsCredential,
}

impl AsRef<SigningKey> for AsSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl AsSigningKey {
    pub fn from_private_key_and_credential(
        private_key: SigningKey,
        credential: AsCredential,
    ) -> Self {
        Self {
            signing_key: private_key,
            credential,
        }
    }

    pub fn credential(&self) -> &AsCredential {
        &self.credential
    }

    pub fn into_credential(self) -> AsCredential {
        self.credential
    }
}

impl SigningKeyBehaviour for AsSigningKey {}

#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct AsVerifyingKey(pub(super) VerifyingKey);

impl VerifyingKeyBehaviour for AsVerifyingKey {}

impl AsRef<VerifyingKey> for AsVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct AsIntermediateVerifyingKey(pub(super) VerifyingKey);

impl VerifyingKeyBehaviour for AsIntermediateVerifyingKey {}

impl AsRef<VerifyingKey> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientSigningKey {
    signing_key: SigningKey,
    credential: ClientCredential,
}

impl AsRef<SigningKey> for ClientSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl SigningKeyBehaviour for ClientSigningKey {}

impl ClientSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key.into_signing_key(),
            credential,
        })
    }

    pub fn credential(&self) -> &ClientCredential {
        &self.credential
    }
}

#[derive(
    Clone,
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct ClientVerifyingKey(pub(super) VerifyingKey);

impl ClientVerifyingKey {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(VerifyingKey::from_bytes(bytes))
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl VerifyingKeyBehaviour for ClientVerifyingKey {}

impl AsRef<VerifyingKey> for ClientVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PseudonymousCredentialSigningKey {
    signing_key: SigningKey,
    credential: PseudonymousCredential,
}

impl Type<Sqlite> for PseudonymousCredentialSigningKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for PseudonymousCredentialSigningKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let bytes = PhnxCodec::to_vec(self)?;
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for PseudonymousCredentialSigningKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
        let value = PhnxCodec::from_slice(bytes)?;
        Ok(value)
    }
}

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Error)]
pub enum CredentialCreationError {
    #[error(transparent)]
    KeyGenerationError(#[from] KeyGenerationError),
    #[error(transparent)]
    RandomnessError(#[from] RandomnessError),
    #[error("Failed to derive identity link key")]
    KeyDerivationFailed,
    #[error(transparent)]
    EncryptionFailed(#[from] EncryptionError),
}

impl PseudonymousCredentialSigningKey {
    pub fn generate(
        client_signer: &ClientSigningKey,
        connection_key: &ConnectionKey,
    ) -> Result<(Self, IdentityLinkKey), CredentialCreationError> {
        // Construct the TBS
        let signing_key = SigningKey::generate()?;
        let identity = OpenMlsRustCrypto::default()
            .rand()
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let tbs = PseudonymousCredentialTbs {
            identity,
            expiration_data: Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            signature_scheme: DEFAULT_SIGNATURE_SCHEME,
            verifying_key: signing_key.verifying_key().clone().into(),
        };

        // Derive the identity link key based on the TBS
        let identity_link_key = IdentityLinkKey::derive(connection_key, &tbs).map_err(|e| {
            error!(%e, "Failed to derive identity link key");
            CredentialCreationError::KeyDerivationFailed
        })?;

        // Sign the TBS and encrypt the identity link
        let signed_pseudonymous_credential = tbs.sign(client_signer).unwrap();
        let encrypted_signature = signed_pseudonymous_credential
            .signature
            .encrypt(&identity_link_key)?;
        let encrypted_client_credential = client_signer.credential().encrypt(&identity_link_key)?;
        let identity_link_ctxt = IdentityLinkCtxt {
            encrypted_signature,
            encrypted_client_credential,
        };
        let credential = PseudonymousCredential::new(
            signed_pseudonymous_credential.payload.identity,
            signed_pseudonymous_credential.payload.expiration_data,
            signed_pseudonymous_credential.payload.signature_scheme,
            signed_pseudonymous_credential.payload.verifying_key,
            identity_link_ctxt,
        );
        let credential = Self {
            signing_key,
            credential,
        };
        Ok((credential, identity_link_key))
    }

    pub fn credential(&self) -> &PseudonymousCredential {
        &self.credential
    }
}

impl SigningKeyBehaviour for PseudonymousCredentialSigningKey {}
impl SigningKeyBehaviour for &PseudonymousCredentialSigningKey {}

impl AsRef<SigningKey> for PseudonymousCredentialSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl Signer for PseudonymousCredentialSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        <Self as SigningKeyBehaviour>::sign(self, payload)
            .map_err(|_| SignerError::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.signature_scheme()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PreliminaryClientSigningKey(SigningKey);

impl PreliminaryClientSigningKey {
    pub(super) fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub(super) fn into_signing_key(self) -> SigningKey {
        self.0
    }

    pub(super) fn verifying_key(&self) -> ClientVerifyingKey {
        ClientVerifyingKey(self.0.verifying_key().clone())
    }
}

pub struct PreliminaryAsIntermediateSigningKey(SigningKey);

impl PreliminaryAsIntermediateSigningKey {
    pub(super) fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub(super) fn into_signing_key(self) -> SigningKey {
        self.0
    }

    pub(super) fn verifying_key(&self) -> AsIntermediateVerifyingKey {
        AsIntermediateVerifyingKey(self.0.verifying_key().clone())
    }
}
