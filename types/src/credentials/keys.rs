// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::{
    InfraCredential, Lifetime, OpenMlsProvider, SignaturePublicKey, SignatureScheme,
};
use mls_assist::openmls_rust_crypto::OpenMlsRustCrypto;
use mls_assist::openmls_traits::random::OpenMlsRand;
use mls_assist::openmls_traits::signatures::{Signer, SignerError};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize,
};

use super::{AsCredential, AsIntermediateCredential, PreliminaryAsSigningKey};

use crate::crypto::ear::keys::SignatureEarKey;
use crate::crypto::ear::{Ciphertext, EarDecryptable, EarEncryptable};
use crate::crypto::signatures::keys::generate_signature_keypair;
use crate::crypto::signatures::signable::{
    Signable, Signature, SignedStruct, Verifiable, VerifiedStruct,
};
use crate::crypto::signatures::traits::{SigningKey, VerifyingKey};

use thiserror::Error;

use super::{private_mod, ClientCredential, PreliminaryClientSigningKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsIntermediateSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: AsIntermediateCredential,
}

impl AsRef<[u8]> for AsIntermediateSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl SigningKey for AsIntermediateSigningKey {}

impl AsIntermediateSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryAsSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key_bytes: prelim_key.into_signing_key_bytes(),
            credential,
        })
    }

    pub fn credential(&self) -> &AsIntermediateCredential {
        &self.credential
    }
}

#[derive(Debug, Error)]
pub enum SigningKeyCreationError {
    #[error("Public key mismatch")]
    PublicKeyMismatch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AsSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: AsCredential,
}

impl AsRef<[u8]> for AsSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl AsSigningKey {
    pub(super) fn from_bytes_and_credential(
        signing_key_bytes: Vec<u8>,
        credential: AsCredential,
    ) -> Self {
        Self {
            signing_key_bytes,
            credential,
        }
    }

    pub fn credential(&self) -> &AsCredential {
        &self.credential
    }
}

impl SigningKey for AsSigningKey {}

#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct AsVerifyingKey {
    verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsVerifyingKey {}

impl AsRef<[u8]> for AsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

impl From<Vec<u8>> for AsVerifyingKey {
    fn from(value: Vec<u8>) -> Self {
        Self {
            verifying_key_bytes: value.into(),
        }
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct AsIntermediateVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsIntermediateVerifyingKey {}

impl AsRef<[u8]> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: ClientCredential,
}

impl AsRef<[u8]> for ClientSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl SigningKey for ClientSigningKey {}

impl ClientSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key_bytes: prelim_key.into_signing_key_bytes(),
            credential,
        })
    }

    pub fn credential(&self) -> &ClientCredential {
        &self.credential
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct ClientVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for ClientVerifyingKey {}

impl AsRef<[u8]> for ClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfraCredentialSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: InfraCredential,
}

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

impl InfraCredentialSigningKey {
    pub fn generate(client_signer: &ClientSigningKey, ear_key: &SignatureEarKey) -> Self {
        let keypair = generate_signature_keypair().unwrap();
        let identity = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        let tbs = InfraCredentialTbs {
            identity,
            lifetime: Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            signature_scheme: SignatureScheme::ED25519,
            verifying_key: keypair.1.clone().into(),
        };
        let plaintext_credential = tbs.sign(client_signer).unwrap();
        let encrypted_signature = plaintext_credential.signature.encrypt(ear_key).unwrap();
        let credential = InfraCredential::new(
            plaintext_credential.payload.identity,
            plaintext_credential.payload.lifetime,
            plaintext_credential.payload.signature_scheme,
            plaintext_credential.payload.verifying_key,
            encrypted_signature.tls_serialize_detached().unwrap().into(),
        );
        Self {
            signing_key_bytes: keypair.0,
            credential,
        }
    }

    pub fn credential(&self) -> &InfraCredential {
        &self.credential
    }
}

impl SigningKey for InfraCredentialSigningKey {}
impl SigningKey for &InfraCredentialSigningKey {}

impl AsRef<[u8]> for InfraCredentialSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl Signer for InfraCredentialSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        <Self as SigningKey>::sign(self, payload)
            .map_err(|_| SignerError::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.credential_ciphersuite()
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
        let signature = Signature::decrypt(&ear_key, &encrypted_signature)
            .map_err(|_| InfraCredentialDecryptionError::SignatureDecryptionError)?;
        let payload = InfraCredentialTbs {
            identity: credential.identity().to_vec(),
            lifetime: credential.expiration_data(),
            signature_scheme: credential.credential_ciphersuite(),
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

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Debug, Clone)]
pub struct InfraCredentialTbs {
    pub(crate) identity: Vec<u8>,
    pub(crate) lifetime: Lifetime,
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
