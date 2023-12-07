// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{HashType, OpenMlsCrypto, OpenMlsProvider, SignatureScheme},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use keys::{AsIntermediateVerifyingKey, AsSigningKey, AsVerifyingKey};

use crate::{
    crypto::{
        ear::{keys::ClientCredentialEarKey, Ciphertext, EarDecryptable, EarEncryptable},
        errors::KeyGenerationError,
        signatures::{
            keys::generate_signature_keypair,
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        },
    },
    identifiers::{AsClientId, Fqdn},
    messages::MlsInfraVersion,
    time::ExpirationData,
    LibraryError,
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

pub mod keys;

use self::keys::ClientVerifyingKey;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    TlsDeserializeBytes,
    TlsSerialize,
    TlsSize,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct CredentialFingerprint {
    value: Vec<u8>,
}

impl std::fmt::Display for CredentialFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fp = hex::encode(&self.value);
        write!(f, "{}", fp)
    }
}

impl CredentialFingerprint {
    fn with_label(credential: &impl TlsSerialize, label: &str) -> Self {
        let rust_crypto = OpenMlsRustCrypto::default();
        let payload = credential.tls_serialize_detached().unwrap_or_default();
        let input = [label.as_bytes().to_vec(), payload].concat();
        let value = rust_crypto
            .crypto()
            .hash(HashType::Sha2_256, &input)
            .unwrap_or_default();
        Self { value }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.value
    }
}

const DEFAULT_AS_CREDENTIAL_LIFETIME: i64 = 5 * 365;
const AS_CREDENTIAL_LABEL: &str = "MLS Infra AS Credential";

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Clone, Serialize, Deserialize)]
pub struct AsCredential {
    body: AsCredentialBody,
    fingerprint: CredentialFingerprint,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Clone, Serialize, Deserialize)]
struct AsCredentialBody {
    version: MlsInfraVersion,
    as_domain: Fqdn,
    expiration_data: ExpirationData,
    signature_scheme: SignatureScheme,
    verifying_key: AsVerifyingKey,
}

impl AsCredentialBody {
    fn hash(&self) -> CredentialFingerprint {
        CredentialFingerprint::with_label(self, AS_CREDENTIAL_LABEL)
    }
}

impl AsCredential {
    /// Generate a new [`AsCredential`] with the given data and a freshly
    /// generated signature keypair.
    ///
    /// The default [`ExpirationData`] for an [`AsCredential`] is five years.
    pub fn new(
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
        let body = AsCredentialBody {
            version,
            as_domain,
            expiration_data,
            signature_scheme,
            verifying_key,
        };
        let fingerprint = body.hash();
        let credential = Self { body, fingerprint };
        let signing_key =
            AsSigningKey::from_bytes_and_credential(signing_key_bytes, credential.clone());
        Ok((credential, signing_key))
    }

    pub fn fingerprint(&self) -> &CredentialFingerprint {
        &self.fingerprint
    }

    pub fn verifying_key(&self) -> &AsVerifyingKey {
        &self.body.verifying_key
    }

    pub fn domain(&self) -> &Fqdn {
        &self.body.as_domain
    }
}

const DEFAULT_AS_INTERMEDIATE_CREDENTIAL_LIFETIME: i64 = 365;

pub struct PreliminaryAsSigningKey {
    signing_key_bytes: Vec<u8>,
    verifying_key: AsIntermediateVerifyingKey,
}

impl PreliminaryAsSigningKey {
    pub(crate) fn into_signing_key_bytes(self) -> Vec<u8> {
        self.signing_key_bytes
    }
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct AsIntermediateCredentialCsr {
    version: MlsInfraVersion,
    as_domain: Fqdn,
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
    pub fn new(
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
            as_domain,
        };
        Ok((credential, prelim_signing_key))
    }

    /// Sign the CSR with the given signing key to obtain an
    /// [`AsIntermediateCredential`] with the given expiration data.
    ///
    /// If no expiration data is given, the default [`ExpirationData`] of one
    /// year is set.
    pub fn sign(
        self,
        as_signing_key: &AsSigningKey,
        expiration_data_option: Option<ExpirationData>,
    ) -> Result<AsIntermediateCredential, LibraryError> {
        // Create lifetime valid until 5 years in the future.
        let expiration_data = expiration_data_option.unwrap_or(ExpirationData::new(
            DEFAULT_AS_INTERMEDIATE_CREDENTIAL_LIFETIME,
        ));
        let signer_fingerprint = as_signing_key.credential().fingerprint().clone();
        let credential = AsIntermediateCredentialPayload {
            csr: self,
            expiration_data,
            signer_fingerprint,
        };
        credential.sign(as_signing_key)
    }
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
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

#[derive(Debug, Clone, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct AsIntermediateCredentialBody {
    credential: AsIntermediateCredentialPayload,
    signature: Signature,
}

impl AsIntermediateCredentialBody {
    fn hash(&self) -> CredentialFingerprint {
        CredentialFingerprint::with_label(self, AS_INTERMEDIATE_CREDENTIAL_LABEL)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsIntermediateCredential {
    body: AsIntermediateCredentialBody,
    fingerprint: CredentialFingerprint,
}

impl tls_codec::Serialize for AsIntermediateCredential {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.body.tls_serialize(writer)
    }
}

impl tls_codec::Size for AsIntermediateCredential {
    fn tls_serialized_len(&self) -> usize {
        self.body.tls_serialized_len()
    }
}

impl AsIntermediateCredential {
    pub fn verifying_key(&self) -> &AsIntermediateVerifyingKey {
        &self.body.credential.csr.verifying_key
    }

    pub fn fingerprint(&self) -> &CredentialFingerprint {
        &self.fingerprint
    }

    pub fn domain(&self) -> &Fqdn {
        &self.body.credential.csr.as_domain
    }
}

impl SignedStruct<AsIntermediateCredentialPayload> for AsIntermediateCredential {
    fn from_payload(payload: AsIntermediateCredentialPayload, signature: Signature) -> Self {
        let body = AsIntermediateCredentialBody {
            credential: payload,
            signature,
        };
        let fingerprint = body.hash();
        Self { body, fingerprint }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct VerifiableAsIntermediateCredential {
    credential: AsIntermediateCredentialPayload,
    signature: Signature,
}

impl VerifiableAsIntermediateCredential {
    pub fn signer_fingerprint(&self) -> &CredentialFingerprint {
        &self.credential.signer_fingerprint
    }
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

impl VerifiedStruct<VerifiableAsIntermediateCredential> for AsIntermediateCredential {
    type SealingType = private_mod::Seal;

    fn from_verifiable(
        verifiable: VerifiableAsIntermediateCredential,
        _seal: Self::SealingType,
    ) -> Self {
        Self::from_payload(verifiable.credential, verifiable.signature)
    }
}

const CLIENT_CREDENTIAL_LABEL: &str = "MLS Infra Client Credential";
const DEFAULT_CLIENT_CREDENTIAL_LIFETIME: i64 = 90;

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
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
    pub fn new(
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
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ClientCredentialPayload {
    csr: ClientCredentialCsr,
    expiration_data: ExpirationData,
    signer_fingerprint: CredentialFingerprint,
}

impl ClientCredentialPayload {
    pub fn new(
        csr: ClientCredentialCsr,
        expiration_data_option: Option<ExpirationData>,
        signer_fingerprint: CredentialFingerprint,
    ) -> Self {
        let expiration_data = expiration_data_option
            .unwrap_or(ExpirationData::new(DEFAULT_CLIENT_CREDENTIAL_LIFETIME));
        Self {
            csr,
            expiration_data,
            signer_fingerprint,
        }
    }

    pub fn validate(&self) -> bool {
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

    pub fn identity_ref(&self) -> &AsClientId {
        &self.csr.client_id
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PreliminaryClientSigningKey {
    signing_key_bytes: Vec<u8>,
    verifying_key: ClientVerifyingKey,
}

impl PreliminaryClientSigningKey {
    pub(crate) fn into_signing_key_bytes(self) -> Vec<u8> {
        self.signing_key_bytes
    }
}

#[derive(Debug, Clone, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ClientCredential {
    payload: ClientCredentialPayload,
    signature: Signature,
}

impl ClientCredential {
    pub fn identity(&self) -> AsClientId {
        self.payload.identity()
    }

    pub fn identity_ref(&self) -> &AsClientId {
        self.payload.identity_ref()
    }

    pub fn verifying_key(&self) -> &ClientVerifyingKey {
        &self.payload.csr.verifying_key
    }
}

impl VerifiedStruct<VerifiableClientCredential> for ClientCredential {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientCredential, _seal: Self::SealingType) -> Self {
        Self {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

impl SignedStruct<ClientCredentialPayload> for ClientCredential {
    fn from_payload(payload: ClientCredentialPayload, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

impl EarEncryptable<ClientCredentialEarKey, EncryptedClientCredential> for ClientCredential {}

impl EarDecryptable<ClientCredentialEarKey, EncryptedClientCredential>
    for VerifiableClientCredential
{
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Clone, Serialize, Deserialize)]
pub struct VerifiableClientCredential {
    payload: ClientCredentialPayload,
    signature: Signature,
}

impl VerifiableClientCredential {
    pub fn domain(&self) -> Fqdn {
        self.payload.csr.client_id.user_name.domain()
    }

    pub fn signer_fingerprint(&self) -> &CredentialFingerprint {
        &self.payload.signer_fingerprint
    }

    pub fn client_id(&self) -> &AsClientId {
        &self.payload.csr.client_id
    }
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

#[derive(Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct EncryptedClientCredential {
    pub(super) encrypted_client_credential: Ciphertext,
}

impl From<Ciphertext> for EncryptedClientCredential {
    fn from(value: Ciphertext) -> Self {
        Self {
            encrypted_client_credential: value,
        }
    }
}

impl AsRef<Ciphertext> for EncryptedClientCredential {
    fn as_ref(&self) -> &Ciphertext {
        &self.encrypted_client_credential
    }
}
