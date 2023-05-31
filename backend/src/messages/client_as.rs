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
            CredentialFingerprint,
        },
        errors::AsVerificationError,
        storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
        *,
    },
    crypto::{
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        QueueRatchet, RatchetEncryptionKey,
    },
};

use super::{client_ds::QueueMessagePayload, MlsInfraVersion, QueueMessage};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

// === Authentication ===

trait ClientCredentialAuthenticator
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

trait TwoFactorAuthenticator
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

trait NoAuth
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParamsTbs {
    pub client_id: AsClientId,
    pub user_name: UserName,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_key: QueueRatchet,
    pub connection_key_packages: Vec<KeyPackageIn>,
    pub opaque_registration_record: OpaqueRegistrationRecord,
}

impl Signable for FinishUserRegistrationParamsTbs {
    type SignedOutput = FinishUserRegistrationParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        FinishUserRegistrationParams::LABEL
    }
}

impl SignedStruct<FinishUserRegistrationParamsTbs> for FinishUserRegistrationParams {
    fn from_payload(payload: FinishUserRegistrationParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParams {
    payload: FinishUserRegistrationParamsTbs,
    signature: Signature,
}

impl ClientCredentialAuthenticator for FinishUserRegistrationParams {
    type Tbs = FinishUserRegistrationParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::FinishUserRegistration(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Finish User Registration Parameters";
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
    fn user_auth(self) -> UserAuth {
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

/// Wrapper struct around a message from a client to the AS. It does not
/// implement the [`Verifiable`] trait, but instead is verified depending on the
/// verification method of the individual payload.
#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct VerifiableClientToAsMessage {
    message: ClientToAsMessage,
}

impl VerifiableClientToAsMessage {
    /// Verify/authenticate the message. The authentication method depends on
    /// the request type and is specified for each request in `auth_method`.
    pub(crate) async fn verify<Asp: AsStorageProvider, Eph: AsEphemeralStorageProvider>(
        self,
        as_storage_provider: &Asp,
        ephemeral_storage_provider: &Eph,
    ) -> Result<VerifiedAsRequestParams, AsVerificationError> {
        let parameters = match self.message.auth_method() {
            // No authentication at all. We just return the parameters without
            // verification.
            AsAuthMethod::None(params) => params,
            // Authentication via client credential. We load the client
            // credential from the client's record and use it to verify the
            // request.
            AsAuthMethod::ClientCredential(cca) => {
                let client_record = as_storage_provider
                    .load_client(&cca.client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownClient)?;
                cca.verify(client_record.credential.verifying_key())
                    .map_err(|_| AsVerificationError::AuthenticationFailed)?
            }
            // 2-Factor authentication using a signature by the client
            // credential, as well as an OPAQUE login flow. This requires that
            // the client has first called the endpoint to initiate the OPAQUE
            // login flow.
            // We load the pending OPAQUE login state from the ephemeral
            // database and complete the OPAQUE flow. If that is successful, we
            // verify the signature (which spans the OPAQUE data sent by the
            // client).
            // After successful verification, we delete the entry from the
            // ephemeral DB.
            // TODO: We currently store the credential of the client to be added
            // along with the OPAQUE entry. This is not great, since we can't
            // really return it from here. For now, we just load it again from
            // the processing function.
            AsAuthMethod::Client2Fa(auth_info) => {
                // We authenticate opaque first.
                let client_id = &auth_info.client_credential_auth.client_id.clone();
                let (_client_credential, opaque_state) = ephemeral_storage_provider
                    .load_client_login_state(client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownClient)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(auth_info.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                let client_record = as_storage_provider
                    .load_client(client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownClient)?;
                let verified_params = auth_info
                    .client_credential_auth
                    .verify(client_record.credential.verifying_key())
                    .map_err(|_| AsVerificationError::AuthenticationFailed)?;
                ephemeral_storage_provider
                    .delete_client_login_state(client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                verified_params
            }
            // Authentication using only the user's password via an OPAQUE login flow.
            AsAuthMethod::User(user_auth) => {
                let opaque_state = ephemeral_storage_provider
                    .load_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownUser)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(user_auth.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                ephemeral_storage_provider
                    .delete_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                *user_auth.payload
            }
        };
        Ok(parameters)
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

    pub(crate) fn auth_method(self) -> AsAuthMethod {
        self.body.auth_method()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
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

impl AsRequestParams {
    pub(crate) fn auth_method(self) -> AsAuthMethod {
        match self {
            // Requests authenticated only by the user's password.
            // TODO: We should probably sign/verify the CSR with the verifying
            // key inside to prove ownership of the key.
            // TODO: For now, client addition is only verified by the user's
            // password, not with an additional client credential.
            AsRequestParams::FinishClientAddition(params) => AsAuthMethod::User(params.user_auth()),
            // Requests authenticated using two factors
            AsRequestParams::DeleteUser(params) => {
                AsAuthMethod::Client2Fa(params.two_factor_auth_info())
            }
            // Requests signed by the client's client credential
            AsRequestParams::Initiate2FaAuthentication(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            AsRequestParams::DeleteClient(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            AsRequestParams::DequeueMessages(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            AsRequestParams::PublishKeyPackages(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            AsRequestParams::ClientKeyPackage(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            AsRequestParams::IssueTokens(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            // We verify user registration finish requests like a
            // ClientCredentialAuth request and then additionally complete the
            // OPAQUE registration afterwards.
            AsRequestParams::FinishUserRegistration(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            // Requests not requiring any authentication
            AsRequestParams::UserClients(params) => AsAuthMethod::None(params.into_verified()),
            AsRequestParams::UserKeyPackages(params) => AsAuthMethod::None(params.into_verified()),
            AsRequestParams::EnqueueMessage(params) => AsAuthMethod::None(params.into_verified()),
            AsRequestParams::InitUserRegistration(params) => {
                AsAuthMethod::None(params.into_verified())
            }
            AsRequestParams::InitiateClientAddition(params) => {
                AsAuthMethod::None(params.into_verified())
            }
            AsRequestParams::AsCredentials(params) => AsAuthMethod::None(params.into_verified()),
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub(crate) enum VerifiedAsRequestParams {
    Initiate2FaAuthentication(Init2FactorAuthParamsTbs),
    FinishUserRegistration(FinishUserRegistrationParamsTbs),
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
    user_name: UserName,
    opaque_finish: OpaqueLoginFinish,
    payload: Box<VerifiedAsRequestParams>,
}

#[derive(Debug)]
#[repr(u8)]
pub(crate) enum AsAuthMethod {
    None(VerifiedAsRequestParams),
    ClientCredential(ClientCredentialAuth),
    Client2Fa(Client2FaAuth),
    User(UserAuth),
}
