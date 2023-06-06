// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::{KeyPackage, KeyPackageIn};
use privacypass::{
    batched_tokens::{TokenRequest, TokenResponse},
    Serialize,
};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    auth_service::{
        client_api::privacypass::AsTokenType,
        credentials::{
            AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialPayload,
            CredentialFingerprint, ExpirationData,
        },
        *,
    },
    crypto::{
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        ConnectionEncryptionKey, QueueRatchet, RatchetEncryptionKey,
    },
};

use super::{
    client_as_out::{
        FinishUserRegistrationParamsIn, FinishUserRegistrationParamsTbsIn,
        VerifiableConnectionPackage,
    },
    client_ds::QueueMessagePayload,
    MlsInfraVersion, QueueMessage,
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
    type Tbs: Serialize;

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
    type Tbs: Serialize;

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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct Init2FactorAuthResponse {
    pub(crate) opaque_ke2: OpaqueLoginResponse,
}

#[derive(Debug, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ConnectionPackage {
    payload: ConnectionPackageTbs,
    signature: Signature,
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
        "Connection Package"
    }
}

impl SignedStruct<ConnectionPackageTbs> for ConnectionPackage {
    fn from_payload(payload: ConnectionPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

// === User ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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
    pub(crate) client_credential: ClientCredential,
    pub(crate) opaque_registration_response: OpaqueRegistrationResponse,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParamsTbs {
    pub client_id: AsClientId,
    pub user_name: UserName,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_key: QueueRatchet,
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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
    pub(crate) client_credential: ClientCredential,
    pub(crate) opaque_login_response: OpaqueLoginResponse,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionParamsTbs {
    pub client_id: AsClientId,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_key: QueueRatchet,
    pub connection_key_package: KeyPackageIn,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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
            user_name: self.payload.client_id.username(),
            opaque_finish: self.opaque_login_finish.clone(),
            payload: Box::new(VerifiedAsRequestParams::FinishClientAddition(self.payload)),
        }
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsDequeueMessagesResponse {
    pub(crate) messages: Vec<QueueMessage>,
    pub(crate) remaining_messages_number: u64,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsPublishKeyPackagesParamsTbs {
    pub client_id: AsClientId,
    pub key_packages: Vec<KeyPackageIn>,
}

impl Signable for AsPublishKeyPackagesParamsTbs {
    type SignedOutput = AsPublishKeyPackagesParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsPublishKeyPackagesParams::LABEL
    }
}

impl SignedStruct<AsPublishKeyPackagesParamsTbs> for AsPublishKeyPackagesParams {
    fn from_payload(payload: AsPublishKeyPackagesParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsPublishKeyPackagesParams {
    payload: AsPublishKeyPackagesParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsPublishKeyPackagesParams {
    type Tbs = AsPublishKeyPackagesParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::PublishKeyPackages(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Publish KeyPackages Parameters";
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientKeyPackageParamsTbs(pub AsClientId);

impl Signable for ClientKeyPackageParamsTbs {
    type SignedOutput = AsClientKeyPackageParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        AsClientKeyPackageParams::LABEL
    }
}

impl SignedStruct<ClientKeyPackageParamsTbs> for AsClientKeyPackageParams {
    fn from_payload(payload: ClientKeyPackageParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsClientKeyPackageParams {
    payload: ClientKeyPackageParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsClientKeyPackageParams {
    type Tbs = AsClientId;

    fn client_id(&self) -> AsClientId {
        self.payload.0.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::ClientKeyPackage(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Client KeyPackage Parameters";
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsClientKeyPackageResponse {
    pub(crate) key_package: Option<KeyPackage>,
}

// === Anonymous requests ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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
    pub(crate) client_credentials: Vec<ClientCredential>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesParams {
    pub user_name: UserName,
}

impl NoAuth for UserKeyPackagesParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::UserKeyPackages(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesResponse {
    pub key_packages: Vec<KeyPackage>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct EnqueueMessageParams {
    pub client_id: AsClientId,
    pub connection_establishment_ctxt: QueueMessagePayload,
}

impl NoAuth for EnqueueMessageParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::EnqueueMessage(self)
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsCredentialsParams {}

impl NoAuth for AsCredentialsParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::AsCredentials(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AsCredentialsResponse {
    pub(crate) as_credentials: Vec<AsCredential>,
    pub(crate) as_intermediate_credentials: Vec<AsIntermediateCredential>,
    pub(crate) revoked_credentials: Vec<CredentialFingerprint>,
}

// === Privacy Pass ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct IssueTokensParamsTbs {
    pub client_id: AsClientId,
    pub token_type: AsTokenType,
    pub token_request: TokenRequest,
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct IssueTokensResponse {
    pub(crate) tokens: TokenResponse,
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
    PublishKeyPackages(AsPublishKeyPackagesParams),
    ClientKeyPackage(AsClientKeyPackageParams),
    UserClients(UserClientsParams),
    UserKeyPackages(UserKeyPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
    IssueTokens(IssueTokensParams),
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub(crate) enum VerifiedAsRequestParams {
    Initiate2FaAuthentication(Init2FactorAuthParamsTbs),
    FinishUserRegistration(FinishUserRegistrationParamsTbsIn),
    DeleteUser(DeleteUserParamsTbs),
    FinishClientAddition(FinishClientAdditionParamsTbs),
    DeleteClient(DeleteClientParamsTbs),
    DequeueMessages(DequeueMessagesParamsTbs),
    PublishKeyPackages(AsPublishKeyPackagesParamsTbs),
    ClientKeyPackage(ClientKeyPackageParamsTbs),
    IssueTokens(IssueTokensParamsTbs),
    // Endpoints that don't require authentication
    UserKeyPackages(UserKeyPackagesParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    UserClients(UserClientsParams),
    AsCredentials(AsCredentialsParams),
    EnqueueMessage(EnqueueMessageParams),
    InitUserRegistration(InitUserRegistrationParams),
}

#[derive(Debug)]
pub struct ClientCredentialAuth {
    pub(crate) client_id: AsClientId,
    pub(crate) payload: Box<VerifiedAsRequestParams>,
    pub(crate) label: &'static str,
    pub(crate) signature: Signature,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ClientToAsMessageOut {
    _version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: VerifiedAsRequestParams,
}

impl Signable for ClientCredentialAuth {
    type SignedOutput = ClientToAsMessageOut;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        self.label
    }
}

impl Verifiable for ClientCredentialAuth {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
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
    pub(crate) client_credential_auth: ClientCredentialAuth,
    pub(crate) opaque_finish: OpaqueLoginFinish,
}

#[derive(Debug)]
pub struct UserAuth {
    pub(super) user_name: UserName,
    pub(super) opaque_finish: OpaqueLoginFinish,
    pub(super) payload: Box<VerifiedAsRequestParams>,
}

#[derive(Debug)]
#[repr(u8)]
pub(crate) enum AsAuthMethod {
    None(VerifiedAsRequestParams),
    ClientCredential(ClientCredentialAuth),
    Client2Fa(Client2FaAuth),
    User(UserAuth),
}
