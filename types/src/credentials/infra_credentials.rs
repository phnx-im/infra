// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use mls_assist::{
    openmls::{
        ciphersuite::signature::SignaturePublicKey,
        credentials::{errors::BasicCredentialError, BasicCredential, Credential},
        key_packages::Lifetime,
    },
    openmls_traits::types::SignatureScheme,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{
    DeserializeBytes as _, Serialize as _, TlsDeserialize, TlsDeserializeBytes, TlsSerialize,
    TlsSize,
};

use crate::crypto::{
    ear::{keys::IdentityLinkKey, EarDecryptable},
    signatures::{
        signable::{
            EncryptedSignature, Signable, Signature, SignedStruct, Verifiable, VerifiedStruct,
        },
        traits::SignatureVerificationError,
    },
};

use super::{
    private_mod, AsIntermediateCredential, ClientCredential, CredentialFingerprint,
    EncryptedClientCredential, VerifiableClientCredential,
};

/// A credential that contains a (pseudonymous) identity, some metadata, as well
/// as an encrypted signature.
#[derive(
    Debug, PartialEq, Eq, Clone, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes,
)]
pub struct PseudonymousCredential {
    // (Pseudonymous) identity
    tbs: PseudonymousCredentialTbs,
    identity_link_ctxt: IdentityLinkCtxt,
}

impl PseudonymousCredential {
    /// Create a new [`PseudonymousCredential`].
    pub(crate) fn new(
        identity: Vec<u8>,
        expiration_data: Lifetime,
        credential_ciphersuite: SignatureScheme,
        verifying_key: SignaturePublicKey,
        identity_link_ctxt: IdentityLinkCtxt,
    ) -> Self {
        let tbs = PseudonymousCredentialTbs {
            identity,
            expiration_data,
            signature_scheme: credential_ciphersuite,
            verifying_key,
        };
        Self {
            tbs,
            identity_link_ctxt,
        }
    }

    /// Returns the identity of a given credential.
    pub fn identity(&self) -> &[u8] {
        &self.tbs.identity
    }

    /// Returns the expiration data of a given credential.
    pub fn expiration_data(&self) -> Lifetime {
        self.tbs.expiration_data
    }

    /// Returns the credential ciphersuite of a given credential.
    pub fn signature_scheme(&self) -> SignatureScheme {
        self.tbs.signature_scheme
    }

    /// Returns the verifying key of a given credential.
    pub fn verifying_key(&self) -> &SignaturePublicKey {
        &self.tbs.verifying_key
    }

    /// Returns the encrypted signature of a given credential.
    pub fn identity_link_ctxt(&self) -> &IdentityLinkCtxt {
        &self.identity_link_ctxt
    }
}

impl TryFrom<&PseudonymousCredential> for Credential {
    type Error = tls_codec::Error;

    fn try_from(value: &PseudonymousCredential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::new(value.tls_serialize_detached()?);
        Ok(basic_credential.into())
    }
}

impl TryFrom<Credential> for PseudonymousCredential {
    type Error = BasicCredentialError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::try_from(value)?;
        let pseudonymous_credential =
            PseudonymousCredential::tls_deserialize_exact_bytes(basic_credential.identity())?;
        Ok(pseudonymous_credential)
    }
}

#[derive(
    TlsSerialize, TlsDeserializeBytes, TlsSize, Debug, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
pub(crate) struct IdentityLinkCtxt {
    pub(crate) encrypted_signature: EncryptedSignature,
    pub(crate) encrypted_client_credential: EncryptedClientCredential,
}

#[derive(TlsSerialize, TlsSize, Debug, Clone)]
pub struct PseudonymousCredentialPlaintext {
    pub(crate) payload: PseudonymousCredentialTbs,
    pub(crate) signature: Signature,
    pub(crate) client_credential: ClientCredential,
}

impl PseudonymousCredentialPlaintext {
    pub fn decrypt_and_verify(
        credential: &PseudonymousCredential,
        identity_link_key: &IdentityLinkKey,
        as_verifying_keys: &HashMap<CredentialFingerprint, AsIntermediateCredential>,
    ) -> Result<Self, IdentityLinkDecryptionError> {
        let signature = Signature::decrypt(
            identity_link_key,
            &credential.identity_link_ctxt().encrypted_signature,
        )
        .map_err(|_| IdentityLinkDecryptionError::SignatureDecryptionError)?;
        let verifiable_client_credential = VerifiableClientCredential::decrypt(
            identity_link_key,
            &credential.identity_link_ctxt().encrypted_client_credential,
        )
        .map_err(|_| IdentityLinkDecryptionError::SignatureDecryptionError)?;
        let as_verifying_key = as_verifying_keys
            .get(&verifiable_client_credential.signer_fingerprint())
            .ok_or(IdentityLinkDecryptionError::NoVerifyingKey)?;
        let client_credential =
            verifiable_client_credential.verify(as_verifying_key.verifying_key())?;
        let payload = PseudonymousCredentialTbs {
            identity: credential.identity().to_vec(),
            expiration_data: credential.expiration_data(),
            signature_scheme: credential.signature_scheme(),
            verifying_key: credential.verifying_key().clone(),
        };
        Ok(Self {
            payload,
            signature,
            client_credential,
        })
    }
}

#[derive(Debug, Error)]
pub enum IdentityLinkDecryptionError {
    #[error(transparent)]
    DeserializationError(#[from] tls_codec::Error),
    #[error("Error decrypting signature")]
    SignatureDecryptionError,
    #[error("Missing AS verifying key")]
    NoVerifyingKey,
    #[error("Error verifying client credential: {0}")]
    ClientCredentialVerificationError(#[from] SignatureVerificationError),
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsSize,
    TlsDeserialize,
    TlsDeserializeBytes,
)]
pub struct PseudonymousCredentialTbs {
    pub(crate) identity: Vec<u8>,
    pub(crate) expiration_data: Lifetime,
    pub(crate) signature_scheme: SignatureScheme,
    pub(crate) verifying_key: SignaturePublicKey,
}

#[derive(Debug)]
pub struct SignedPseudonymousCredential {
    pub(super) payload: PseudonymousCredentialTbs,
    pub(super) signature: Signature,
}

impl Verifiable for SignedPseudonymousCredential {
    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "PseudonymousCredential"
    }
}

impl VerifiedStruct<SignedPseudonymousCredential> for PseudonymousCredentialTbs {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: SignedPseudonymousCredential, _seal: Self::SealingType) -> Self {
        verifiable.payload
    }
}

impl SignedStruct<PseudonymousCredentialTbs> for SignedPseudonymousCredential {
    fn from_payload(payload: PseudonymousCredentialTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

impl Signable for PseudonymousCredentialTbs {
    type SignedOutput = SignedPseudonymousCredential;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "PseudonymousCredential"
    }
}
