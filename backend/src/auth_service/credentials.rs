// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    HashType, OpenMlsCrypto, OpenMlsCryptoProvider, OpenMlsRustCrypto, SignaturePublicKey,
    SignatureScheme,
};
use privacypass::Serialize;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::{keys::SignatureEncryptionKey, Ciphertext, EarEncryptable},
        signatures::{
            signable::{Signable, Signature, Verifiable, VerifiedStruct},
            traits::{SignatureVerificationError, VerifyingKey},
        },
    },
    ds::group_state::TimeStamp,
    messages::MlsInfraVersion,
    qs::Fqdn,
    LibraryError,
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

use super::AsClientId;

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct CredentialFingerprint {
    value: Vec<u8>,
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ExpirationData {
    not_before: TimeStamp,
    not_after: TimeStamp,
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct AsVerifyingKey {
    signature_key: SignaturePublicKey,
}

impl VerifyingKey for AsVerifyingKey {}

impl AsRef<[u8]> for AsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsCredential {
    version: MlsInfraVersion,
    as_domain: Fqdn,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    public_key: AsVerifyingKey,
}

impl AsCredential {
    fn fingerprint(&self) -> Result<CredentialFingerprint, LibraryError> {
        let backend = OpenMlsRustCrypto::default();
        let payload = self
            .tls_serialize_detached()
            .map_err(LibraryError::missing_bound_check)?;
        let input = [AS_CREDENTIAL_LABEL.as_bytes().to_vec(), payload].concat();
        let value = backend
            .crypto()
            .hash(HashType::Sha2_256, &input)
            .map_err(|e| LibraryError::unexpected_crypto_error(&e.to_string()))?;
        Ok(CredentialFingerprint { value })
    }
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct AsIntermediateVerifyingKey {
    signature_key: SignaturePublicKey,
}

impl VerifyingKey for AsIntermediateVerifyingKey {}

impl AsRef<[u8]> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsIntermediateCredentialPayload {
    version: MlsInfraVersion,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    public_key: AsIntermediateVerifyingKey, // PK used to sign client credentials
    signer_fingerprint: CredentialFingerprint, // fingerprint of the signing AsCredential
}

pub const AS_CREDENTIAL_LABEL: &str = "MLS Infra AS Intermediate Credential"; // format!("{credential_label} AS Intermediate Credential");

impl Signable for AsIntermediateCredentialPayload {
    type SignedOutput = AsIntermediateCredential;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AS_CREDENTIAL_LABEL
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub(crate) struct AsIntermediateCredential {
    credential: AsIntermediateCredentialPayload,
    signature: Signature,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct VerifiableAsIntermediateCredential {
    credential: AsIntermediateCredentialPayload,
    signature: Signature,
}

impl Verifiable for VerifiableAsIntermediateCredential {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.credential.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        AS_CREDENTIAL_LABEL
    }
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct ClientVerifyingKey {
    signature_key: SignaturePublicKey,
}

impl VerifyingKey for ClientVerifyingKey {}

impl AsRef<[u8]> for ClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

pub const CLIENT_CREDENTIAL_LABEL: &str = "MLS Infra Client Credential";

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCredentialPayload {
    client_id: AsClientId,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    public_key: ClientVerifyingKey,
    signer_fingerprint: CredentialFingerprint,
}

impl ClientCredentialPayload {
    pub(crate) fn validate(&self) -> bool {
        // TODO: Check expiration date and uniqueness of client id
        todo!()
    }
}

impl Signable for ClientCredentialPayload {
    type SignedOutput = ClientCredential;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        CLIENT_CREDENTIAL_LABEL
    }
}

impl ClientCredentialPayload {
    pub fn identity(&self) -> AsClientId {
        self.client_id.clone()
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ClientCredential {
    payload: ClientCredentialPayload,
    signature: Signature,
}

impl ClientCredential {
    pub fn identity(&self) -> AsClientId {
        self.payload.identity()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct VerifiableClientCredential {
    payload: ClientCredentialPayload,
    signature: Signature,
}

impl Verifiable for VerifiableClientCredential {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        CLIENT_CREDENTIAL_LABEL
    }
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct LeafVerifyingKey {
    signature_key: SignaturePublicKey,
}

impl VerifyingKey for LeafVerifyingKey {}

impl AsRef<[u8]> for LeafVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.signature_key.as_slice()
    }
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct LeafCredentialPayload {
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    public_key: LeafVerifyingKey,
    signer_fingerprint: CredentialFingerprint,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct LeafCredential {
    payload: LeafCredentialPayload,
    signature: Signature,
}

pub const LEAF_CREDENTIAL_LABEL: &str = "Leaf Intermediate Credential";

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct VerifiableLeafCredential {
    payload: LeafCredentialPayload,
    signature: Signature,
}

impl Verifiable for VerifiableLeafCredential {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        LEAF_CREDENTIAL_LABEL
    }
}

impl VerifiedStruct<VerifiableLeafCredential> for LeafCredential {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableLeafCredential, _seal: Self::SealingType) -> Self {
        todo!()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct EncryptedSignature {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedSignature {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl AsRef<Ciphertext> for EncryptedSignature {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

impl EarEncryptable<SignatureEncryptionKey, EncryptedSignature> for Signature {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ObfuscatedLeafCredential {
    payload: LeafCredentialPayload,
    encrypted_signature: EncryptedSignature,
}

impl ObfuscatedLeafCredential {
    pub fn verify(
        &self,
        verifying_key: &ClientVerifyingKey,
        signature_encryption_key: &SignatureEncryptionKey,
    ) -> Result<LeafCredential, SignatureVerificationError> {
        // TODO: We might want to throw a more specific error here.
        let signature = Signature::decrypt(signature_encryption_key, &self.encrypted_signature)
            .map_err(|_| SignatureVerificationError::VerificationFailure)?;
        VerifiableLeafCredential {
            payload: self.payload.clone(),
            signature,
        }
        .verify(verifying_key)
    }
}
