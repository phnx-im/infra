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
            keys::{
                AsIntermediateKeypair, AsIntermediateSigningKey, AsIntermediateVerifyingKey,
                AsKeypair, AsSigningKey, AsVerifyingKey, KeyGenerationError,
            },
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
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

impl ExpirationData {
    /// Create a new instance of [`ExpirationData`] that expires in `lifetime`
    /// days and the validity of which starts now.
    pub(crate) fn new(lifetime: i64) -> Self {
        Self {
            not_before: TimeStamp::now(),
            not_after: TimeStamp::in_days(lifetime),
        }
    }

    /// Return false either if the `not_after` date has passed, or if the
    /// `not_before` date has not passed yet.
    pub(crate) fn validate(&self) -> bool {
        self.not_after.has_passed() && !self.not_before.has_passed()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsCredential {
    version: MlsInfraVersion,
    as_domain: Fqdn,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    verifying_key: AsVerifyingKey,
}

impl AsCredential {
    /// Generate a new [`AsCredential`] with the given data and a freshly
    /// generated signature keypair.
    ///
    /// The default [`ExpirationData`] for an [`AsCredential`] is five years.
    pub(crate) fn new(
        signature_scheme: SignatureScheme,
        as_domain: Fqdn,
        expiration_data_option: Option<ExpirationData>,
    ) -> Result<(Self, AsSigningKey), KeyGenerationError> {
        let version = MlsInfraVersion::default();
        // Create lifetime valid until 5 years in the future.
        let expiration_data = expiration_data_option.unwrap_or(ExpirationData::new(5 * 365));
        let as_keypair = AsKeypair::new()?;
        let credential = Self {
            version,
            as_domain,
            expiration_data,
            signature_scheme,
            verifying_key: as_keypair.verifying_key,
        };
        Ok((credential, as_keypair.signing_key))
    }

    // TODO: This function should be generalized to work for all credentials.
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsIntermediateCredentialPayload {
    version: MlsInfraVersion,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    verifying_key: AsIntermediateVerifyingKey, // PK used to sign client credentials
    signer_fingerprint: CredentialFingerprint, // fingerprint of the signing AsCredential
}

impl AsIntermediateCredentialPayload {
    /// Generate a new [`AsIntermediateCredential`] with the given data and a freshly
    /// generated signature keypair.
    ///
    /// The default [`ExpirationData`] for an [`AsIntermediateCredential`] is
    /// one year.
    pub(crate) fn new(
        signature_scheme: SignatureScheme,
        as_domain: Fqdn,
        expiration_data_option: Option<ExpirationData>,
        signer_fingerprint: CredentialFingerprint,
    ) -> Result<(Self, AsIntermediateSigningKey), KeyGenerationError> {
        let version = MlsInfraVersion::default();
        // Create lifetime valid until 1 year in the future.
        let expiration_data = expiration_data_option.unwrap_or(ExpirationData::new(365));
        let as_keypair = AsIntermediateKeypair::new()?;
        let credential = Self {
            version,
            expiration_data,
            signature_scheme,
            verifying_key: as_keypair.verifying_key,
            signer_fingerprint,
        };
        Ok((credential, as_keypair.signing_key))
    }
}

pub const AS_CREDENTIAL_LABEL: &str = "MLS Infra AS Intermediate Credential";

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
pub struct AsIntermediateCredential {
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
    verifying_key: ClientVerifyingKey,
    signer_fingerprint: CredentialFingerprint,
}

impl ClientCredentialPayload {
    pub(crate) fn validate(&self) -> bool {
        // TODO: Check uniqueness of client id
        self.expiration_data.validate()
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

impl SignedStruct<ClientCredentialPayload> for ClientCredential {
    fn from_payload(payload: ClientCredentialPayload, signature: Signature) -> Self {
        Self { payload, signature }
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
    /// Verify this credential using the given [`ClientCredential`]. The
    /// [`SignatureEncryptionKey`] is required to decrypt the signature on this
    /// [`ObfuscatedLeafCredential`].
    ///
    /// Note that type-based verification enforces that the [`ClientCredential`]
    /// was already validated, thus guaranteeing verification of the whole
    /// chain.
    pub fn verify(
        &self,
        client_credential: &ClientCredential,
        signature_encryption_key: &SignatureEncryptionKey,
    ) -> Result<LeafCredential, SignatureVerificationError> {
        // TODO: We might want to throw a more specific error here.
        let signature = Signature::decrypt(signature_encryption_key, &self.encrypted_signature)
            .map_err(|_| SignatureVerificationError::VerificationFailure)?;
        VerifiableLeafCredential {
            payload: self.payload.clone(),
            signature,
        }
        .verify(&client_credential.payload.verifying_key)
    }
}
