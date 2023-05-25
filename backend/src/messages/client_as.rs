// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{KeyPackage, KeyPackageIn};
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
        signatures::signable::{Signature, Verifiable, VerifiedStruct},
        QueueRatchet, RatchetPublicKey,
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
pub(crate) struct Init2FactorAuthParamsTbs {
    pub(crate) client_id: AsClientId,
    pub(crate) opaque_ke1: OpaqueLoginRequest,
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
    pub(crate) client_payload: ClientCredentialPayload,
    pub(crate) opaque_registration_request: OpaqueRegistrationRequest,
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
    pub(crate) client_id: AsClientId,
    pub(crate) user_name: UserName,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) initial_ratchet_key: QueueRatchet,
    pub(crate) connection_key_packages: Vec<KeyPackageIn>,
    pub(crate) opaque_registration_record: OpaqueRegistrationRecord,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParams {
    pub(crate) payload: FinishUserRegistrationParamsTbs,
    pub(crate) signature: Signature,
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
pub struct FinishUserRegistrationResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteUserParamsTbs {
    pub(crate) user_name: UserName,
    pub(crate) client_id: AsClientId,
    pub(crate) opaque_finish: OpaqueLoginFinish,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteUserParams {
    pub(crate) payload: DeleteUserParamsTbs,
    pub(crate) signature: Signature,
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

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteUserResponse {}

// === Client ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct InitiateClientAdditionParams {
    pub(crate) client_credential_payload: ClientCredentialPayload,
    pub(crate) opaque_login_request: OpaqueLoginRequest,
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
    pub(crate) client_id: AsClientId,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) initial_ratchet_key: QueueRatchet,
    pub(crate) connection_key_package: KeyPackageIn,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionParams {
    pub(crate) payload: FinishClientAdditionParamsTbs,
    pub(crate) opaque_login_finish: OpaqueLoginFinish,
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
pub struct FinishClientAdditionResponse {}

pub(crate) type DeleteClientParamsTbs = AsClientId;

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteClientParams {
    pub(crate) payload: DeleteClientParamsTbs,
    pub(crate) signature: Signature,
}

impl ClientCredentialAuthenticator for DeleteClientParams {
    type Tbs = DeleteClientParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.clone()
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
pub struct DeleteClientResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct DequeueMessagesParamsTbs {
    pub(crate) sender: AsClientId,
    pub(crate) sequence_number_start: u64,
    pub(crate) max_message_number: u64,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DequeueMessagesParams {
    pub(crate) payload: DequeueMessagesParamsTbs,
    pub(crate) signature: Signature,
}

impl ClientCredentialAuthenticator for DequeueMessagesParams {
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
pub struct DequeueMessagesResponse {
    pub(crate) messages: Vec<QueueMessage>,
    pub(crate) remaining_messages_number: u64,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct PublishKeyPackagesParamsTbs {
    pub(crate) client_id: AsClientId,
    pub(crate) key_packages: Vec<KeyPackageIn>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParams {
    pub(crate) payload: PublishKeyPackagesParamsTbs,
    pub(crate) signature: Signature,
}

impl ClientCredentialAuthenticator for PublishKeyPackagesParams {
    type Tbs = PublishKeyPackagesParamsTbs;

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
pub struct PublishKeyPackagesResponse {}

pub(crate) type ClientKeyPackageParamsTbs = AsClientId;

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientKeyPackageParams {
    pub(crate) payload: ClientKeyPackageParamsTbs,
    pub(crate) signature: Signature,
}

impl ClientCredentialAuthenticator for ClientKeyPackageParams {
    type Tbs = AsClientId;

    fn client_id(&self) -> AsClientId {
        self.payload.clone()
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
pub struct ClientKeyPackageResponse {
    pub(crate) key_package: Option<KeyPackage>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientKeyPackageResponseIn {
    pub(crate) key_package: Option<KeyPackageIn>,
}

// === Anonymous requests ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserClientsParams {
    pub(crate) user_name: UserName,
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
    pub(crate) user_name: UserName,
}

impl NoAuth for UserKeyPackagesParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::UserKeyPackages(self)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesResponse {
    pub(crate) key_packages: Vec<KeyPackage>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesResponseIn {
    pub(crate) key_packages: Vec<KeyPackageIn>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct EnqueueMessageParams {
    pub(crate) client_id: AsClientId,
    pub(crate) connection_establishment_ctxt: QueueMessagePayload,
}

impl NoAuth for EnqueueMessageParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::EnqueueMessage(self)
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsEnqueueMessageResponse {}

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
    pub(crate) client_id: AsClientId,
    pub(crate) token_type: AsTokenType,
    pub(crate) token_request: TokenRequest,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct IssueTokensParams {
    pub(crate) payload: IssueTokensParamsTbs,
    pub(crate) signature: Signature,
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
            AsAuthMethod::NoAuth(params) => params,
            // Authentication via client credential. We load the client
            // credential from the client's record and use it to verify the
            // request.
            AsAuthMethod::ClientCredentialAuth(cca) => {
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
            AsAuthMethod::Client2FaAuth(auth_info) => {
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
            AsAuthMethod::UserAuth(user_auth) => {
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
pub(crate) struct ClientToAsMessage {
    _version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: AsRequestParams,
}

impl ClientToAsMessage {
    pub(crate) fn auth_method(self) -> AsAuthMethod {
        self.body.auth_method()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
#[repr(u8)]
pub(crate) enum AsRequestParams {
    Initiate2FaAuthentication(Initiate2FaAuthenticationParams),
    InitUserRegistration(InitUserRegistrationParams),
    FinishUserRegistration(FinishUserRegistrationParams),
    DeleteUser(DeleteUserParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    FinishClientAddition(FinishClientAdditionParams),
    DeleteClient(DeleteClientParams),
    DequeueMessages(DequeueMessagesParams),
    PublishKeyPackages(PublishKeyPackagesParams),
    ClientKeyPackage(ClientKeyPackageParams),
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
            AsRequestParams::FinishClientAddition(params) => {
                AsAuthMethod::UserAuth(params.user_auth())
            }
            // Requests authenticated using two factors
            AsRequestParams::DeleteUser(params) => {
                AsAuthMethod::Client2FaAuth(params.two_factor_auth_info())
            }
            // Requests signed by the client's client credential
            AsRequestParams::Initiate2FaAuthentication(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            AsRequestParams::DeleteClient(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            AsRequestParams::DequeueMessages(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            AsRequestParams::PublishKeyPackages(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            AsRequestParams::ClientKeyPackage(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            AsRequestParams::IssueTokens(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            // We verify user registration finish requests like a
            // ClientCredentialAuth request and then additionally complete the
            // OPAQUE registration afterwards.
            AsRequestParams::FinishUserRegistration(params) => {
                AsAuthMethod::ClientCredentialAuth(params.credential_auth_info())
            }
            // Requests not requiring any authentication
            AsRequestParams::UserClients(params) => AsAuthMethod::NoAuth(params.into_verified()),
            AsRequestParams::UserKeyPackages(params) => {
                AsAuthMethod::NoAuth(params.into_verified())
            }
            AsRequestParams::EnqueueMessage(params) => AsAuthMethod::NoAuth(params.into_verified()),
            AsRequestParams::InitUserRegistration(params) => {
                AsAuthMethod::NoAuth(params.into_verified())
            }
            AsRequestParams::InitiateClientAddition(params) => {
                AsAuthMethod::NoAuth(params.into_verified())
            }
            AsRequestParams::AsCredentials(params) => AsAuthMethod::NoAuth(params.into_verified()),
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
    PublishKeyPackages(PublishKeyPackagesParamsTbs),
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
    NoAuth(VerifiedAsRequestParams),
    ClientCredentialAuth(ClientCredentialAuth),
    Client2FaAuth(Client2FaAuth),
    UserAuth(UserAuth),
}
