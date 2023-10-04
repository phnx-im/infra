// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{openmls::prelude::GroupId, openmls_traits::types::HpkeCiphertext};
use privacypass::batched_tokens::{TokenRequest, TokenResponse};

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
        ear::{
            keys::{
                AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey,
                GroupStateEarKey, RatchetKey, SignatureEarKeyWrapperKey,
                WelcomeAttributionInfoEarKey,
            },
            Ciphertext, EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
        },
        hpke::HpkeEncryptable,
        kdf::keys::RatchetSecret,
        opaque::{
            OpaqueLoginFinish, OpaqueLoginRequest, OpaqueLoginResponse, OpaqueRegistrationRecord,
            OpaqueRegistrationRequest, OpaqueRegistrationResponse,
        },
        ratchet::QueueRatchet,
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        ConnectionEncryptionKey, RatchetEncryptionKey,
    },
    identifiers::{AsClientId, UserName},
    time::ExpirationData,
};

use super::{
    client_as_out::{
        ConnectionPackageIn, FinishUserRegistrationParamsIn, FinishUserRegistrationParamsTbsIn,
        VerifiableConnectionPackage,
    },
    AsTokenType, EncryptedAsQueueMessage, FriendshipToken, MlsInfraVersion,
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

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

pub(super) trait TwoFactorAuthenticator
where
    Self: Sized,
{
    type Tbs: TlsSerializeTrait;

    fn client_id(&self) -> AsClientId;
    fn into_payload(self) -> VerifiedAsRequestParams;
    fn signature(&self) -> &Signature;
    fn opaque_finish(&self) -> &OpaqueLoginFinish;

    const LABEL: &'static str;

    fn two_factor_auth_info(self) -> Client2FaAuth {
        let signature = self.signature().clone();
        let opaque_finish = self.opaque_finish().clone();
        let client_credential_auth = ClientCredentialAuth {
            client_id: self.client_id(),
            payload: Box::new(self.into_payload()),
            label: Self::LABEL,
            signature,
        };
        Client2FaAuth {
            client_credential_auth,
            opaque_finish,
        }
    }
}

pub(super) trait NoAuth
where
    Self: Sized,
{
    fn into_verified(self) -> VerifiedAsRequestParams;
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct Init2FactorAuthParamsTbs {
    pub client_id: AsClientId,
    pub opaque_ke1: OpaqueLoginRequest,
}

impl Signable for Init2FactorAuthParamsTbs {
    type SignedOutput = Initiate2FaAuthenticationParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        Initiate2FaAuthenticationParams::LABEL
    }
}

impl SignedStruct<Init2FactorAuthParamsTbs> for Initiate2FaAuthenticationParams {
    fn from_payload(payload: Init2FactorAuthParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct Initiate2FaAuthenticationParams {
    payload: Init2FactorAuthParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for Initiate2FaAuthenticationParams {
    type Tbs = Init2FactorAuthParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::Initiate2FaAuthentication(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Initiate 2FA Authentication Parameters";
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct Init2FactorAuthResponse {
    pub opaque_ke2: OpaqueLoginResponse,
}

#[derive(Debug, Clone, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackageTbs {
    pub(super) protocol_version: MlsInfraVersion,
    pub(super) encryption_key: ConnectionEncryptionKey,
    pub(super) lifetime: ExpirationData,
    pub(super) client_credential: ClientCredential,
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

#[derive(Debug, Clone, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackage {
    payload: ConnectionPackageTbs,
    signature: Signature,
}

impl ConnectionPackage {
    pub fn client_credential(&self) -> &ClientCredential {
        &self.payload.client_credential
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
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

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct InitUserRegistrationParams {
    pub client_payload: ClientCredentialPayload,
    pub opaque_registration_request: OpaqueRegistrationRequest,
}

impl NoAuth for InitUserRegistrationParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::InitUserRegistration(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct InitUserRegistrationResponse {
    pub client_credential: ClientCredential,
    pub opaque_registration_response: OpaqueRegistrationResponse,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParamsTbs {
    pub client_id: AsClientId,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub connection_packages: Vec<ConnectionPackage>,
    pub opaque_registration_record: OpaqueRegistrationRecord,
}

impl Signable for FinishUserRegistrationParamsTbs {
    type SignedOutput = FinishUserRegistrationParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        FinishUserRegistrationParamsIn::LABEL
    }
}

impl SignedStruct<FinishUserRegistrationParamsTbs> for FinishUserRegistrationParams {
    fn from_payload(payload: FinishUserRegistrationParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParams {
    payload: FinishUserRegistrationParamsTbs,
    signature: Signature,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DeleteUserParamsTbs {
    pub user_name: UserName,
    pub client_id: AsClientId,
    pub opaque_finish: OpaqueLoginFinish,
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

impl TwoFactorAuthenticator for DeleteUserParams {
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

    fn opaque_finish(&self) -> &OpaqueLoginFinish {
        &self.payload.opaque_finish
    }

    const LABEL: &'static str = "Delete User Parameters";
}

// === Client ===

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct InitiateClientAdditionParams {
    pub client_credential_payload: ClientCredentialPayload,
    pub opaque_login_request: OpaqueLoginRequest,
}

impl NoAuth for InitiateClientAdditionParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::InitiateClientAddition(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct InitClientAdditionResponse {
    pub client_credential: ClientCredential,
    pub opaque_login_response: OpaqueLoginResponse,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionParamsTbs {
    pub client_id: AsClientId,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub connection_package: ConnectionPackageIn,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionParams {
    pub payload: FinishClientAdditionParamsTbs,
    pub opaque_login_finish: OpaqueLoginFinish,
}

impl FinishClientAdditionParams {
    // TODO: This is currently implemented manually since this is the only
    // request that needs user auth. We might want to generalize this into a
    // trait later on.
    pub(super) fn user_auth(self) -> UserAuth {
        UserAuth {
            user_name: self.payload.client_id.user_name(),
            opaque_finish: self.opaque_login_finish.clone(),
            payload: Box::new(VerifiedAsRequestParams::FinishClientAddition(self.payload)),
        }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DeleteClientParamsTbs(pub AsClientId);

impl Signable for DeleteClientParamsTbs {
    type SignedOutput = DeleteClientParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        DeleteClientParams::LABEL
    }
}

impl SignedStruct<DeleteClientParamsTbs> for DeleteClientParams {
    fn from_payload(payload: DeleteClientParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct DeleteClientParams {
    payload: DeleteClientParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for DeleteClientParams {
    type Tbs = DeleteClientParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.0.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::DeleteClient(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Delete Client Parameters";
}

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

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct FriendshipPackage {
    pub friendship_token: FriendshipToken,
    pub add_package_ear_key: AddPackageEarKey,
    pub client_credential_ear_key: ClientCredentialEarKey,
    pub signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub wai_ear_key: WelcomeAttributionInfoEarKey,
}

impl GenericSerializable for FriendshipPackage {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl GenericDeserializable for FriendshipPackage {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact(bytes)
    }
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct EncryptedFriendshipPackage {
    ciphertext: Ciphertext,
}

impl AsRef<Ciphertext> for EncryptedFriendshipPackage {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

impl From<Ciphertext> for EncryptedFriendshipPackage {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl EarEncryptable<FriendshipPackageEarKey, EncryptedFriendshipPackage> for FriendshipPackage {}
impl EarDecryptable<FriendshipPackageEarKey, EncryptedFriendshipPackage> for FriendshipPackage {}

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageTbs {
    pub sender_client_credential: ClientCredential,
    pub connection_group_id: GroupId,
    pub connection_group_ear_key: GroupStateEarKey,
    pub connection_group_credential_key: ClientCredentialEarKey,
    pub connection_group_signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
    pub friendship_package: FriendshipPackage,
}

impl Signable for ConnectionEstablishmentPackageTbs {
    type SignedOutput = ConnectionEstablishmentPackage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ConnectionEstablishmentPackageTBS"
    }
}

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackage {
    payload: ConnectionEstablishmentPackageTbs,
    // TBS: All information above signed by the ClientCredential.
    signature: Signature,
}

impl GenericSerializable for ConnectionEstablishmentPackage {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl HpkeEncryptable<ConnectionEncryptionKey, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackage
{
}

impl SignedStruct<ConnectionEstablishmentPackageTbs> for ConnectionEstablishmentPackage {
    fn from_payload(payload: ConnectionEstablishmentPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct EncryptedConnectionEstablishmentPackage {
    ciphertext: HpkeCiphertext,
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

pub type AsQueueRatchet = QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>;

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
                let cep =
                    EncryptedConnectionEstablishmentPackage::tls_deserialize_exact(&self.payload)?;
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
        Self::tls_deserialize_exact(bytes)
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

impl EarEncryptable<RatchetKey, EncryptedAsQueueMessage> for AsQueueMessagePayload {}
impl EarDecryptable<RatchetKey, EncryptedAsQueueMessage> for AsQueueMessagePayload {}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct AsPublishConnectionPackagesParamsTbs {
    pub client_id: AsClientId,
    pub connection_packages: Vec<ConnectionPackageIn>,
}

impl Signable for AsPublishConnectionPackagesParamsTbs {
    type SignedOutput = AsPublishConnectionPackagesParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsPublishConnectionPackagesParams::LABEL
    }
}

impl SignedStruct<AsPublishConnectionPackagesParamsTbs> for AsPublishConnectionPackagesParams {
    fn from_payload(payload: AsPublishConnectionPackagesParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct AsPublishConnectionPackagesParams {
    payload: AsPublishConnectionPackagesParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsPublishConnectionPackagesParams {
    type Tbs = AsPublishConnectionPackagesParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::PublishConnectionPackages(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Publish ConnectionPackages Parameters";
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
    pub user_name: UserName,
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
    pub user_name: UserName,
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
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (client_id, bytes) = AsClientId::tls_deserialize(bytes)?;
        let (token_type, bytes) = AsTokenType::tls_deserialize(bytes)?;
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
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
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
pub struct ClientToAsMessage {
    _version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: AsRequestParams,
}

impl ClientToAsMessage {
    pub fn new(body: AsRequestParams) -> Self {
        Self {
            _version: MlsInfraVersion::default(),
            body,
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsRequestParams {
    Initiate2FaAuthentication(Initiate2FaAuthenticationParams),
    InitUserRegistration(InitUserRegistrationParams),
    FinishUserRegistration(FinishUserRegistrationParams),
    DeleteUser(DeleteUserParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    FinishClientAddition(FinishClientAdditionParams),
    DeleteClient(DeleteClientParams),
    DequeueMessages(AsDequeueMessagesParams),
    PublishConnectionPackages(AsPublishConnectionPackagesParams),
    ClientConnectionPackage(AsClientConnectionPackageParams),
    UserClients(UserClientsParams),
    UserConnectionPackages(UserConnectionPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
    IssueTokens(IssueTokensParams),
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum VerifiedAsRequestParams {
    Initiate2FaAuthentication(Init2FactorAuthParamsTbs),
    FinishUserRegistration(FinishUserRegistrationParamsTbsIn),
    DeleteUser(DeleteUserParamsTbs),
    FinishClientAddition(FinishClientAdditionParamsTbs),
    DeleteClient(DeleteClientParamsTbs),
    DequeueMessages(DequeueMessagesParamsTbs),
    PublishConnectionPackages(AsPublishConnectionPackagesParamsTbs),
    ClientConnectionPackage(ClientConnectionPackageParamsTbs),
    IssueTokens(IssueTokensParamsTbs),
    // Endpoints that don't require authentication
    UserConnectionPackages(UserConnectionPackagesParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    UserClients(UserClientsParams),
    AsCredentials(AsCredentialsParams),
    EnqueueMessage(EnqueueMessageParams),
    InitUserRegistration(InitUserRegistrationParams),
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

    pub fn is_finish_user_registration_request(&self) -> bool {
        matches!(
            self.payload.as_ref(),
            VerifiedAsRequestParams::FinishUserRegistration(_)
        )
    }
}

impl Verifiable for ClientCredentialAuth {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        match self.payload.as_ref() {
            VerifiedAsRequestParams::Initiate2FaAuthentication(params) => {
                params.tls_serialize_detached()
            }
            VerifiedAsRequestParams::DeleteClient(params) => params.tls_serialize_detached(),
            VerifiedAsRequestParams::DequeueMessages(params) => params.tls_serialize_detached(),
            VerifiedAsRequestParams::PublishConnectionPackages(params) => {
                params.tls_serialize_detached()
            }
            VerifiedAsRequestParams::ClientConnectionPackage(params) => {
                params.tls_serialize_detached()
            }
            VerifiedAsRequestParams::IssueTokens(params) => params.tls_serialize_detached(),
            VerifiedAsRequestParams::FinishUserRegistration(params) => {
                params.tls_serialize_detached()
            }
            // All other endpoints aren't authenticated via client credential signatures.
            VerifiedAsRequestParams::DeleteUser(_)
            | VerifiedAsRequestParams::FinishClientAddition(_)
            | VerifiedAsRequestParams::UserConnectionPackages(_)
            | VerifiedAsRequestParams::InitiateClientAddition(_)
            | VerifiedAsRequestParams::UserClients(_)
            | VerifiedAsRequestParams::AsCredentials(_)
            | VerifiedAsRequestParams::EnqueueMessage(_)
            | VerifiedAsRequestParams::InitUserRegistration(_) => Ok(vec![]),
        }
    }

    fn signature(&self) -> &Signature {
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
pub struct Client2FaAuth {
    pub client_credential_auth: ClientCredentialAuth,
    pub opaque_finish: OpaqueLoginFinish,
}

#[derive(Debug)]
pub struct UserAuth {
    pub user_name: UserName,
    pub opaque_finish: OpaqueLoginFinish,
    pub payload: Box<VerifiedAsRequestParams>,
}

#[derive(Debug)]
#[repr(u8)]
#[allow(clippy::large_enum_variant)]
pub enum AsAuthMethod {
    None(VerifiedAsRequestParams),
    ClientCredential(ClientCredentialAuth),
    Client2Fa(Client2FaAuth),
    User(UserAuth),
}
