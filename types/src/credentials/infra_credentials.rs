// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    TlsSize, VLBytes,
};

use crate::crypto::{
    ear::{keys::SignatureEarKey, Ciphertext, EarDecryptable},
    signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
};

use super::private_mod;

/// A credential that contains a (pseudonymous) identity, some metadata, as well
/// as an encrypted signature.
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
pub struct InfraCredential {
    // (Pseudonymous) identity
    tbs: InfraCredentialTbs,
    encrypted_signature: VLBytes,
}

impl InfraCredential {
    /// Create a new [`InfraCredential`].
    pub fn new(
        identity: Vec<u8>,
        expiration_data: Lifetime,
        credential_ciphersuite: SignatureScheme,
        verifying_key: SignaturePublicKey,
        encrypted_signature: VLBytes,
    ) -> Self {
        let tbs = InfraCredentialTbs {
            identity,
            expiration_data,
            signature_scheme: credential_ciphersuite,
            verifying_key,
        };
        Self {
            tbs,
            encrypted_signature,
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
    pub fn encrypted_signature(&self) -> &VLBytes {
        &self.encrypted_signature
    }
}

impl TryFrom<&InfraCredential> for Credential {
    type Error = tls_codec::Error;

    fn try_from(value: &InfraCredential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::new(value.tls_serialize_detached()?);
        Ok(basic_credential.into())
    }
}

impl TryFrom<Credential> for InfraCredential {
    type Error = BasicCredentialError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::try_from(value)?;
        let infra_credential =
            InfraCredential::tls_deserialize_exact_bytes(basic_credential.identity())?;
        Ok(infra_credential)
    }
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Debug, Clone)]
pub struct InfraCredentialPlaintext {
    pub(crate) payload: InfraCredentialTbs,
    pub(crate) signature: Signature,
}

impl InfraCredentialPlaintext {
    pub fn decrypt(
        credential: &InfraCredential,
        ear_key: &SignatureEarKey,
    ) -> Result<Self, InfraCredentialDecryptionError> {
        let encrypted_signature =
            Ciphertext::tls_deserialize_exact_bytes(credential.encrypted_signature().as_slice())?
                .into();
        let signature = Signature::decrypt(ear_key, &encrypted_signature)
            .map_err(|_| InfraCredentialDecryptionError::SignatureDecryptionError)?;
        let payload = InfraCredentialTbs {
            identity: credential.identity().to_vec(),
            expiration_data: credential.expiration_data(),
            signature_scheme: credential.signature_scheme(),
            verifying_key: credential.verifying_key().clone(),
        };
        Ok(Self { payload, signature })
    }
}

#[derive(Debug, Error)]
pub enum InfraCredentialDecryptionError {
    #[error(transparent)]
    DeserializationError(#[from] tls_codec::Error),
    #[error("Error decrypting signature")]
    SignatureDecryptionError,
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
pub struct InfraCredentialTbs {
    pub(crate) identity: Vec<u8>,
    pub(crate) expiration_data: Lifetime,
    pub(crate) signature_scheme: SignatureScheme,
    pub(crate) verifying_key: SignaturePublicKey,
}

impl Verifiable for InfraCredentialPlaintext {
    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "InfraCredential"
    }
}

impl VerifiedStruct<InfraCredentialPlaintext> for InfraCredentialTbs {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: InfraCredentialPlaintext, _seal: Self::SealingType) -> Self {
        verifiable.payload
    }
}

impl SignedStruct<InfraCredentialTbs> for InfraCredentialPlaintext {
    fn from_payload(payload: InfraCredentialTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

impl Signable for InfraCredentialTbs {
    type SignedOutput = InfraCredentialPlaintext;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "InfraCredential"
    }
}
