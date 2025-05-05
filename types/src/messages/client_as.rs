// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls_traits::types::HpkeCiphertext;
use privacypass::batched_tokens_ristretto255::{TokenRequest, TokenResponse};

use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, Size, TlsDeserializeBytes, TlsSerialize,
    TlsSize,
};

use serde::{Deserialize, Serialize};

use crate::{
    credentials::{
        AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialPayload,
        CredentialFingerprint,
    },
    crypto::{
        ConnectionEncryptionKey, RatchetEncryptionKey,
        ear::{
            Ciphertext, EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
            keys::RatchetKey,
        },
        kdf::keys::RatchetSecret,
        ratchet::QueueRatchet,
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
    },
    errors::version::VersionError,
    identifiers::{AsClientId, QualifiedUserName},
    time::ExpirationData,
};

use super::{
    ApiVersion, AsTokenType, EncryptedAsQueueMessageCtype, MlsInfraVersion,
    client_as_out::{
        AsPublishConnectionPackagesParamsIn, AsPublishConnectionPackagesParamsTbsIn,
        EncryptedUserProfile, GetUserProfileParams, RegisterUserParamsIn, UpdateUserProfileParams,
        UpdateUserProfileParamsTbs, VerifiableConnectionPackage,
    },
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

pub const CURRENT_AS_API_VERSION: ApiVersion = ApiVersion::new(1).unwrap();

pub const SUPPORTED_AS_API_VERSIONS: &[ApiVersion] = &[CURRENT_AS_API_VERSION];

// === Authentication ===

pub(super) trait ClientCredentialAuthenticator
where
    Self: Sized,
{
    type Tbs: TlsSerializeTrait;

    fn client_id(&self) -> AsClientId;
    fn into_payload(self) -> VerifiedAsRequestParams;
    fn signature(&self) -> &Signature;

    const LABEL: &'static str;

    fn credential_auth_info(self) -> ClientCredentialAuth {
        let signature = self.signature().clone();
        ClientCredentialAuth {
            client_id: self.client_id(),
            payload: Box::new(self.into_payload()),
            label: Self::LABEL,
            signature,
        }
    }
}

pub(super) trait NoAuth
where
    Self: Sized,
{
    fn into_verified(self) -> VerifiedAsRequestParams;
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackageTbs {
    pub protocol_version: MlsInfraVersion,
    pub encryption_key: ConnectionEncryptionKey,
    pub lifetime: ExpirationData,
    pub client_credential: ClientCredential,
}

impl ConnectionPackageTbs {
    pub fn new(
        protocol_version: MlsInfraVersion,
        encryption_key: ConnectionEncryptionKey,
        lifetime: ExpirationData,
        client_credential: ClientCredential,
    ) -> Self {
        Self {
            protocol_version,
            encryption_key,
            lifetime,
            client_credential,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackage {
    payload: ConnectionPackageTbs,
    signature: Signature,
}

impl ConnectionPackage {
    pub fn new(payload: ConnectionPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }

    pub fn into_parts(self) -> (ConnectionPackageTbs, Signature) {
        (self.payload, self.signature)
    }

    pub fn client_credential(&self) -> &ClientCredential {
        &self.payload.client_credential
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
    }

    pub fn client_credential_signer_fingerprint(&self) -> &CredentialFingerprint {
        self.payload.client_credential.signer_fingerprint()
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

impl VerifiedStruct<VerifiableConnectionPackage> for ConnectionPackage {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableConnectionPackage, _seal: Self::SealingType) -> Self {
        Self {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

impl Signable for ConnectionPackageTbs {
    type SignedOutput = ConnectionPackage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ConnectionPackage"
    }
}

impl SignedStruct<ConnectionPackageTbs> for ConnectionPackage {
    fn from_payload(payload: ConnectionPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

// === User ===

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct RegisterUserParams {
    pub client_payload: ClientCredentialPayload,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct RegisterUserResponse {
    pub client_credential: ClientCredential,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DeleteUserParamsTbs {
    pub user_name: QualifiedUserName,
    pub client_id: AsClientId,
}

impl Signable for DeleteUserParamsTbs {
    type SignedOutput = DeleteUserParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        DeleteUserParams::LABEL
    }
}

impl SignedStruct<DeleteUserParamsTbs> for DeleteUserParams {
    fn from_payload(payload: DeleteUserParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DeleteUserParams {
    payload: DeleteUserParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for DeleteUserParams {
    type Tbs = DeleteUserParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::DeleteUser(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Delete User Parameters";
}

// === Client ===

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DequeueMessagesParamsTbs {
    pub sender: AsClientId,
    pub sequence_number_start: u64,
    pub max_message_number: u64,
}

impl Signable for DequeueMessagesParamsTbs {
    type SignedOutput = AsDequeueMessagesParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsDequeueMessagesParams::LABEL
    }
}

impl SignedStruct<DequeueMessagesParamsTbs> for AsDequeueMessagesParams {
    fn from_payload(payload: DequeueMessagesParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct AsDequeueMessagesParams {
    payload: DequeueMessagesParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsDequeueMessagesParams {
    type Tbs = DequeueMessagesParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.sender.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::DequeueMessages(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Dequeue Messages Parameters";
}

#[derive(Debug)]
pub struct EncryptedFriendshipPackageCtype;
pub type EncryptedFriendshipPackage = Ciphertext<EncryptedFriendshipPackageCtype>;

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct EncryptedConnectionEstablishmentPackage {
    ciphertext: HpkeCiphertext,
}

impl EncryptedConnectionEstablishmentPackage {
    pub fn into_ciphertext(self) -> HpkeCiphertext {
        self.ciphertext
    }
}

impl AsRef<HpkeCiphertext> for EncryptedConnectionEstablishmentPackage {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

impl From<HpkeCiphertext> for EncryptedConnectionEstablishmentPackage {
    fn from(ciphertext: HpkeCiphertext) -> Self {
        Self { ciphertext }
    }
}

pub type AsQueueRatchet = QueueRatchet<EncryptedAsQueueMessageCtype, AsQueueMessagePayload>;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
#[repr(u8)]
pub enum AsQueueMessageType {
    EncryptedConnectionEstablishmentPackage,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct AsQueueMessagePayload {
    pub message_type: AsQueueMessageType,
    pub payload: Vec<u8>,
}

impl AsQueueMessagePayload {
    pub fn extract(self) -> Result<ExtractedAsQueueMessagePayload, tls_codec::Error> {
        let message = match self.message_type {
            AsQueueMessageType::EncryptedConnectionEstablishmentPackage => {
                let cep = EncryptedConnectionEstablishmentPackage::tls_deserialize_exact_bytes(
                    &self.payload,
                )?;
                ExtractedAsQueueMessagePayload::EncryptedConnectionEstablishmentPackage(cep)
            }
        };
        Ok(message)
    }
}

impl TryFrom<EncryptedConnectionEstablishmentPackage> for AsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn try_from(value: EncryptedConnectionEstablishmentPackage) -> Result<Self, Self::Error> {
        Ok(Self {
            message_type: AsQueueMessageType::EncryptedConnectionEstablishmentPackage,
            payload: value.tls_serialize_detached()?,
        })
    }
}

impl GenericDeserializable for AsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

impl GenericSerializable for AsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

pub enum ExtractedAsQueueMessagePayload {
    EncryptedConnectionEstablishmentPackage(EncryptedConnectionEstablishmentPackage),
}

impl EarEncryptable<RatchetKey, EncryptedAsQueueMessageCtype> for AsQueueMessagePayload {}
impl EarDecryptable<RatchetKey, EncryptedAsQueueMessageCtype> for AsQueueMessagePayload {}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsPublishConnectionPackagesParamsTbs {
    pub client_id: AsClientId,
    pub connection_packages: Vec<ConnectionPackage>,
}

impl Signable for AsPublishConnectionPackagesParamsTbs {
    type SignedOutput = AsPublishConnectionPackagesParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsPublishConnectionPackagesParamsIn::LABEL
    }
}

impl SignedStruct<AsPublishConnectionPackagesParamsTbs> for AsPublishConnectionPackagesParams {
    fn from_payload(payload: AsPublishConnectionPackagesParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsPublishConnectionPackagesParams {
    payload: AsPublishConnectionPackagesParamsTbs,
    signature: Signature,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct ClientConnectionPackageParamsTbs(pub AsClientId);

impl Signable for ClientConnectionPackageParamsTbs {
    type SignedOutput = AsClientConnectionPackageParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsClientConnectionPackageParams::LABEL
    }
}

impl SignedStruct<ClientConnectionPackageParamsTbs> for AsClientConnectionPackageParams {
    fn from_payload(payload: ClientConnectionPackageParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct AsClientConnectionPackageParams {
    payload: ClientConnectionPackageParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsClientConnectionPackageParams {
    type Tbs = AsClientId;

    fn client_id(&self) -> AsClientId {
        self.payload.0.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::ClientConnectionPackage(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Client ConnectionPackage Parameters";
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsClientConnectionPackageResponse {
    pub connection_package: Option<ConnectionPackage>,
}

// === Anonymous requests ===

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct UserClientsParams {
    pub user_name: QualifiedUserName,
}

impl NoAuth for UserClientsParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::UserClients(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UserClientsResponse {
    pub client_credentials: Vec<ClientCredential>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct UserConnectionPackagesParams {
    pub user_name: QualifiedUserName,
}

impl NoAuth for UserConnectionPackagesParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::UserConnectionPackages(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UserConnectionPackagesResponse {
    pub key_packages: Vec<ConnectionPackage>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct EnqueueMessageParams {
    pub client_id: AsClientId,
    pub connection_establishment_ctxt: EncryptedConnectionEstablishmentPackage,
}

impl NoAuth for EnqueueMessageParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::EnqueueMessage(self)
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct AsCredentialsParams {}

impl NoAuth for AsCredentialsParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::AsCredentials(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsCredentialsResponse {
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<AsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}

// === Privacy Pass ===

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct IssueTokensParamsTbs {
    pub client_id: AsClientId,
    pub token_type: AsTokenType,
    pub token_request: TokenRequest,
}

impl DeserializeBytes for IssueTokensParamsTbs {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (client_id, bytes) = AsClientId::tls_deserialize_bytes(bytes)?;
        let (token_type, bytes) = AsTokenType::tls_deserialize_bytes(bytes)?;
        let mut bytes_reader = bytes;
        let token_request =
            <TokenRequest as tls_codec::Deserialize>::tls_deserialize(&mut bytes_reader)?;
        let bytes = bytes
            .get(token_request.tls_serialized_len()..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((
            Self {
                client_id,
                token_type,
                token_request,
            },
            bytes,
        ))
    }
}

impl Signable for IssueTokensParamsTbs {
    type SignedOutput = IssueTokensParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        IssueTokensParams::LABEL
    }
}

impl SignedStruct<IssueTokensParamsTbs> for IssueTokensParams {
    fn from_payload(payload: IssueTokensParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct IssueTokensParams {
    payload: IssueTokensParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for IssueTokensParams {
    type Tbs = IssueTokensParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::IssueTokens(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Issue Tokens Parameters";
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct IssueTokensResponse {
    pub tokens: TokenResponse,
}

impl DeserializeBytes for IssueTokensResponse {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let mut bytes_reader = bytes;
        let tokens = <TokenResponse as tls_codec::Deserialize>::tls_deserialize(&mut bytes_reader)?;
        let bytes = bytes
            .get(tokens.tls_serialized_len()..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { tokens }, bytes))
    }
}

// === Auth & Framing ===

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ClientToAsMessageOut {
    // This essentially includes the wire format.
    body: AsVersionedRequestParamsOut,
}

impl ClientToAsMessageOut {
    pub fn new(body: AsVersionedRequestParamsOut) -> Self {
        Self { body }
    }

    pub fn into_body(self) -> AsVersionedRequestParamsOut {
        self.body
    }
}

#[derive(Debug)]
pub enum AsVersionedRequestParamsOut {
    Alpha(AsRequestParamsOut),
}

impl AsVersionedRequestParamsOut {
    pub fn with_version(
        params: AsRequestParamsOut,
        version: ApiVersion,
    ) -> Result<Self, VersionError> {
        match version.value() {
            1 => Ok(Self::Alpha(params)),
            _ => Err(VersionError::new(version, SUPPORTED_AS_API_VERSIONS)),
        }
    }

    pub fn change_version(
        self,
        to_version: ApiVersion,
    ) -> Result<(Self, ApiVersion), VersionError> {
        let from_version = self.version();
        match (to_version.value(), self) {
            (1, Self::Alpha(params)) => Ok((Self::Alpha(params), from_version)),
            (_, _) => Err(VersionError::new(to_version, SUPPORTED_AS_API_VERSIONS)),
        }
    }

    fn version(&self) -> ApiVersion {
        match self {
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }
}

impl tls_codec::Size for AsVersionedRequestParamsOut {
    fn tls_serialized_len(&self) -> usize {
        match self {
            Self::Alpha(params) => {
                self.version().tls_value().tls_serialized_len() + params.tls_serialized_len()
            }
        }
    }
}

// Note: Manual implementation because `TlsSerialize` does not support custom variant tags.
impl tls_codec::Serialize for AsVersionedRequestParamsOut {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            Self::Alpha(params) => {
                Ok(self.version().tls_value().tls_serialize(writer)?
                    + params.tls_serialize(writer)?)
            }
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsRequestParamsOut {
    RegisterUser(RegisterUserParams),
    DeleteUser(DeleteUserParams),
    DequeueMessages(AsDequeueMessagesParams),
    PublishConnectionPackages(AsPublishConnectionPackagesParams),
    ClientConnectionPackage(AsClientConnectionPackageParams),
    UserClients(UserClientsParams),
    UserConnectionPackages(UserConnectionPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
    IssueTokens(IssueTokensParams),
    GetUserProfile(GetUserProfileParams),
    UpdateUserProfile(UpdateUserProfileParams),
}

#[derive(Debug)]
#[repr(u8)]
pub enum VerifiedAsRequestParams {
    DeleteUser(DeleteUserParamsTbs),
    DequeueMessages(DequeueMessagesParamsTbs),
    PublishConnectionPackages(AsPublishConnectionPackagesParamsTbsIn),
    ClientConnectionPackage(ClientConnectionPackageParamsTbs),
    IssueTokens(IssueTokensParamsTbs),
    UserConnectionPackages(UserConnectionPackagesParams),
    UserClients(UserClientsParams),
    AsCredentials(AsCredentialsParams),
    EnqueueMessage(EnqueueMessageParams),
    RegisterUser(RegisterUserParamsIn),
    GetUserProfile(GetUserProfileParams),
    UpdateUserProfile(UpdateUserProfileParamsTbs),
}

#[derive(Debug)]
pub struct ClientCredentialAuth {
    client_id: AsClientId,
    payload: Box<VerifiedAsRequestParams>,
    label: &'static str,
    signature: Signature,
}

impl ClientCredentialAuth {
    pub fn client_id(&self) -> &AsClientId {
        &self.client_id
    }
}

impl Verifiable for ClientCredentialAuth {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        match self.payload.as_ref() {
            VerifiedAsRequestParams::DequeueMessages(params) => params.tls_serialize_detached(),
            VerifiedAsRequestParams::PublishConnectionPackages(params) => {
                params.tls_serialize_detached()
            }
            VerifiedAsRequestParams::ClientConnectionPackage(params) => {
                params.tls_serialize_detached()
            }
            VerifiedAsRequestParams::IssueTokens(params) => params.tls_serialize_detached(),
            VerifiedAsRequestParams::UpdateUserProfile(params) => params.tls_serialize_detached(),
            // All other endpoints aren't authenticated via client credential signatures.
            VerifiedAsRequestParams::DeleteUser(_)
            | VerifiedAsRequestParams::UserConnectionPackages(_)
            | VerifiedAsRequestParams::UserClients(_)
            | VerifiedAsRequestParams::AsCredentials(_)
            | VerifiedAsRequestParams::EnqueueMessage(_)
            | VerifiedAsRequestParams::RegisterUser(_)
            | VerifiedAsRequestParams::GetUserProfile(_) => Ok(vec![]),
        }
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        &self.signature
    }

    fn label(&self) -> &str {
        self.label
    }
}

impl VerifiedStruct<ClientCredentialAuth> for VerifiedAsRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: ClientCredentialAuth, _seal: Self::SealingType) -> Self {
        *verifiable.payload
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum AsAuthMethod {
    None(VerifiedAsRequestParams),
    ClientCredential(ClientCredentialAuth),
}
