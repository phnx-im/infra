// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    HashType, OpenMlsCrypto, OpenMlsCryptoProvider, OpenMlsRustCrypto, SignaturePublicKey,
    SignatureScheme,
};
use privacypass::Serialize;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use keys::{
    generate_signature_keypair, AsIntermediateVerifyingKey, AsSigningKey, AsVerifyingKey,
    KeyGenerationError,
};

use crate::{
    crypto::{
        ear::{keys::SignatureEncryptionKey, Ciphertext, EarEncryptable},
        signatures::{
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

mod keys;

// Re-export signing keys for storage provider.
pub(crate) use keys::AsIntermediateSigningKey;

use self::keys::ClientVerifyingKey;

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

fn fingerprint_with_label(
    credential: &impl Serialize,
    label: &str,
) -> Result<CredentialFingerprint, LibraryError> {
    let backend = OpenMlsRustCrypto::default();
    let payload = credential
        .tls_serialize_detached()
        .map_err(LibraryError::missing_bound_check)?;
    let input = [label.as_bytes().to_vec(), payload].concat();
    let value = backend
        .crypto()
        .hash(HashType::Sha2_256, &input)
        .map_err(|e| LibraryError::unexpected_crypto_error(&e.to_string()))?;
    Ok(CredentialFingerprint { value })
}

const DEFAULT_AS_CREDENTIAL_LIFETIME: i64 = 5 * 365;
const AS_CREDENTIAL_LABEL: &str = "MLS Infra AS Credential";

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize, Clone)]
pub struct AsCredential {
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
        let expiration_data =
            expiration_data_option.unwrap_or(ExpirationData::new(DEFAULT_AS_CREDENTIAL_LIFETIME));
        let (signing_key_bytes, verifying_key_bytes) = generate_signature_keypair()?;
        let verifying_key = verifying_key_bytes.into();
        let credential = Self {
            version,
            as_domain,
            expiration_data,
            signature_scheme,
            verifying_key,
        };
        let signing_key =
            AsSigningKey::from_bytes_and_credential(signing_key_bytes, credential.clone());
        Ok((credential, signing_key))
    }

    fn fingerprint(&self) -> Result<CredentialFingerprint, LibraryError> {
        fingerprint_with_label(self, AS_CREDENTIAL_LABEL)
    }
}

const DEFAULT_AS_INTERMEDIATE_CREDENTIAL_LIFETIME: i64 = 365;

pub(crate) struct PreliminaryAsSigningKey {
    signing_key_bytes: Vec<u8>,
    verifying_key: AsIntermediateVerifyingKey,
}

impl PreliminaryAsSigningKey {
    pub(crate) fn into_signing_key_bytes(self) -> Vec<u8> {
        self.signing_key_bytes
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsIntermediateCredentialCsr {
    version: MlsInfraVersion,
    signature_scheme: SignatureScheme,
    verifying_key: AsIntermediateVerifyingKey, // PK used to sign client credentials
}

impl AsIntermediateCredentialCsr {
    /// Generate a new [`AsIntermediateCredentialCsr`] with the given data and a freshly
    /// generated signature keypair.
    ///
    /// Returns the CSR and a preliminary signing key. The preliminary signing
    /// key can be turned into a [`AsIntermediateSigningKey`] once the CSR is
    /// signed.
    pub(crate) fn new(
        signature_scheme: SignatureScheme,
        as_domain: Fqdn,
    ) -> Result<(Self, PreliminaryAsSigningKey), KeyGenerationError> {
        let version = MlsInfraVersion::default();
        let (signing_key_bytes, verifying_key_bytes) = generate_signature_keypair()?;
        let verifying_key = AsIntermediateVerifyingKey {
            verifying_key_bytes: verifying_key_bytes.into(),
        };
        let prelim_signing_key = PreliminaryAsSigningKey {
            signing_key_bytes,
            verifying_key: verifying_key.clone(),
        };
        let credential = Self {
            version,
            signature_scheme,
            verifying_key,
        };
        Ok((credential, prelim_signing_key))
    }

    /// Sign the CSR with the given signing key to obtain an
    /// [`AsIntermediateCredential`] with the given expiration data.
    ///
    /// If no expiration data is given, the default [`ExpirationData`] of one
    /// year is set.
    pub(crate) fn sign(
        self,
        as_signing_key: &AsSigningKey,
        expiration_data_option: Option<ExpirationData>,
    ) -> Result<AsIntermediateCredential, LibraryError> {
        // Create lifetime valid until 5 years in the future.
        let expiration_data = expiration_data_option.unwrap_or(ExpirationData::new(
            DEFAULT_AS_INTERMEDIATE_CREDENTIAL_LIFETIME,
        ));
        let signer_fingerprint = as_signing_key.credential().fingerprint()?;
        let credential = AsIntermediateCredentialPayload {
            csr: self,
            expiration_data,
            signer_fingerprint,
        };
        credential.sign(as_signing_key)
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
struct AsIntermediateCredentialPayload {
    csr: AsIntermediateCredentialCsr,
    expiration_data: ExpirationData,
    signer_fingerprint: CredentialFingerprint, // fingerprint of the signing AsCredential
}

pub const AS_INTERMEDIATE_CREDENTIAL_LABEL: &str = "MLS Infra AS Intermediate Credential";

impl Signable for AsIntermediateCredentialPayload {
    type SignedOutput = AsIntermediateCredential;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AS_INTERMEDIATE_CREDENTIAL_LABEL
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsIntermediateCredential {
    credential: AsIntermediateCredentialPayload,
    signature: Signature,
}

impl AsIntermediateCredential {
    fn verifying_key(&self) -> &AsIntermediateVerifyingKey {
        &self.credential.csr.verifying_key
    }

    fn fingerprint(&self) -> Result<CredentialFingerprint, LibraryError> {
        fingerprint_with_label(self, AS_INTERMEDIATE_CREDENTIAL_LABEL)
    }
}

impl SignedStruct<AsIntermediateCredentialPayload> for AsIntermediateCredential {
    fn from_payload(payload: AsIntermediateCredentialPayload, signature: Signature) -> Self {
        Self {
            credential: payload,
            signature,
        }
    }
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
        AS_INTERMEDIATE_CREDENTIAL_LABEL
    }
}

const CLIENT_CREDENTIAL_LABEL: &str = "MLS Infra Client Credential";
const DEFAULT_CLIENT_CREDENTIAL_LIFETIME: i64 = 90;

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCredentialCsr {
    version: MlsInfraVersion,
    client_id: AsClientId,
    signature_scheme: SignatureScheme,
    verifying_key: ClientVerifyingKey,
}

impl ClientCredentialCsr {
    /// Generate a new [`ClientCredentialCsr`] with the given data and a freshly
    /// generated signature keypair.
    ///
    /// Returns the CSR and a preliminary signing key. The preliminary signing
    /// key can be turned into a [`AsIntermediateSigningKey`] once the CSR is
    /// signed.
    pub(crate) fn new(
        client_id: AsClientId,
        signature_scheme: SignatureScheme,
    ) -> Result<(Self, PreliminaryClientSigningKey), KeyGenerationError> {
        let version = MlsInfraVersion::default();
        let (signing_key_bytes, verifying_key_bytes) = generate_signature_keypair()?;
        let verifying_key = ClientVerifyingKey {
            verifying_key_bytes: verifying_key_bytes.into(),
        };
        let prelim_signing_key = PreliminaryClientSigningKey {
            signing_key_bytes,
            verifying_key: verifying_key.clone(),
        };
        let credential = Self {
            version,
            signature_scheme,
            verifying_key,
            client_id,
        };
        Ok((credential, prelim_signing_key))
    }

    /// Sign the CSR with the given signing key to obtain a [`ClientCredential`]
    /// with the given expiration data.
    ///
    /// If no expiration data is given, the default [`ExpirationData`] of 90
    /// days is set.
    pub(crate) fn sign(
        self,
        as_intermediate_signing_key: &AsIntermediateSigningKey,
        expiration_data_option: Option<ExpirationData>,
    ) -> Result<ClientCredential, LibraryError> {
        // Create lifetime valid until 5 years in the future.
        let expiration_data = expiration_data_option.unwrap_or(ExpirationData::new(
            DEFAULT_AS_INTERMEDIATE_CREDENTIAL_LIFETIME,
        ));
        let signer_fingerprint = as_intermediate_signing_key.credential().fingerprint()?;
        let credential = ClientCredentialPayload {
            csr: self,
            expiration_data,
            signer_fingerprint,
        };
        credential.sign(as_intermediate_signing_key)
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCredentialPayload {
    csr: ClientCredentialCsr,
    expiration_data: ExpirationData,
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
        self.csr.client_id.clone()
    }
}

pub(crate) struct PreliminaryClientSigningKey {
    signing_key_bytes: Vec<u8>,
    verifying_key: ClientVerifyingKey,
}

impl PreliminaryClientSigningKey {
    pub(crate) fn into_signing_key_bytes(self) -> Vec<u8> {
        self.signing_key_bytes
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

    pub(crate) fn verifying_key(&self) -> &ClientVerifyingKey {
        &self.payload.csr.verifying_key
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
        Self {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
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
pub struct UnlinkableLeafCredential {
    payload: LeafCredentialPayload,
    encrypted_signature: EncryptedSignature,
}

impl UnlinkableLeafCredential {
    /// Verify this credential using the given [`ClientCredential`]. The
    /// [`SignatureEncryptionKey`] is required to decrypt the signature on this
    /// [`UnlinkableLeafCredential`].
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
        .verify(&client_credential.payload.csr.verifying_key)
    }
}
