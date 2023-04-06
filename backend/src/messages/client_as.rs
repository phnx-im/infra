// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::KeyPackage;
use privacypass::batched_tokens::{TokenRequest, TokenResponse};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    auth_service::{
        client_api::privacypass::AsTokenType,
        credentials::{
            AsCredential, ClientCredential, ClientCredentialPayload, CredentialFingerprint,
            VerifiableAsIntermediateCredential,
        },
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
    pub struct Seal;
}

// === Authentication ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct Initiate2FaAuthenticationParams {
    auth_method: ClientCredentialAuth,
    client_id: AsClientId,
    opaque_ke1: OpaqueKe1,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct Initiate2FaAuthenticationResponse {
    opaque_ke2: OpaqueKe2,
}

// === User ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct InitUserRegistrationParams {
    pub(crate) auth_method: NoAuth,
    pub(crate) client_csr: ClientCredentialPayload,
    pub(crate) opaque_registration_request: OpaqueRegistrationRequest,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct InitUserRegistrationResponse {
    pub(crate) client_credential: ClientCredential,
    pub(crate) opaque_registration_response: OpaqueRegistrationResponse,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationParams {
    pub(crate) auth_method: Client2FaAuth,
    pub(crate) user_name: UserName,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) initial_ratchet_key: QueueRatchet,
    pub(crate) connection_key_packages: Vec<KeyPackage>,
    pub(crate) opaque_registration_record: OpaqueRegistrationRecord,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishUserRegistrationResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteUserParams {
    pub(crate) auth_method: Client2FaAuth,
    pub(crate) user_name: UserName,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteUserResponse {}

// === Client ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct InitiateClientAdditionParams {
    pub(crate) auth_method: UserAuth,
    pub(crate) client_credential_payload: ClientCredentialPayload,
    pub(crate) opaque_ke1: OpaqueKe1,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct InitClientAdditionResponse {
    pub(crate) client_credential: ClientCredential,
    pub(crate) opaque_ke2: OpaqueKe2,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionParams {
    pub(crate) auth_method: Client2FaAuth,
    pub(crate) client_id: AsClientId,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) initial_ratchet_key: QueueRatchet,
    pub(crate) connection_key_package: KeyPackage,
    pub(crate) opaque_ke3: OpaqueKe3,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct FinishClientAdditionResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteClientParams {
    pub(crate) auth_method: ClientCredentialAuth,
    pub(crate) client_id: AsClientId,
}
#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DeleteClientResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DequeueMessagesParams {
    pub auth_method: ClientCredentialAuth,
    pub sender: AsClientId,
    pub sequence_number_start: u64,
    pub max_message_number: u64,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct DequeueMessagesResponse {
    pub(crate) messages: Vec<QueueMessage>,
    pub(crate) remaining_messages_number: u64,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParams {
    pub(crate) auth_method: ClientCredentialAuth,
    pub(crate) key_packages: Vec<KeyPackage>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientKeyPackageParams {
    pub(crate) auth_method: ClientCredentialAuth,
    pub(crate) client_id: AsClientId,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientKeyPackageResponse {
    pub(crate) key_package: Option<KeyPackage>,
}

// === Anonymous requests ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserClientsParams {
    pub(crate) auth_method: NoAuth,
    pub(crate) user_name: UserName,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UserClientsResponse {
    pub(crate) client_credentials: Vec<ClientCredential>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesParams {
    pub(crate) auth_method: NoAuth,
    pub(crate) user_name: UserName,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserKeyPackagesResponse {
    pub(crate) key_packages: Vec<KeyPackage>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct EnqueueMessageParams {
    pub(crate) auth_method: NoAuth,
    pub(crate) client_id: AsClientId,
    pub(crate) connection_establishment_ctxt: QueueMessagePayload,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsEnqueueMessageResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsCredentialsParams {
    auth_method: NoAuth,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsCredentialsResponse {
    pub(crate) auth_method: NoAuth,
    pub(crate) as_credentials: Vec<AsCredential>,
    pub(crate) as_intermediate_credentials: Vec<VerifiableAsIntermediateCredential>,
    pub(crate) revoked_certs: Vec<CredentialFingerprint>,
}

// === Privacy Pass ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct IssueTokensParams {
    pub(crate) auth_method: ClientCredentialAuth,
    pub(crate) token_type: AsTokenType,
    pub(crate) token_request: TokenRequest,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct IssueTokensResponse {
    pub(crate) tokens: Vec<TokenResponse>,
}

// === Auth & Framing ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct VerifiableClientToAsMessage {
    message: ClientToAsMessage,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToAsMessage {
    pub fn auth_method(&self) -> AsAuthMethod {
        self.message.auth_method()
    }
}

impl Verifiable for VerifiableClientToAsMessage {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.serialized_payload.clone())
    }

    fn signature(&self) -> &Signature {
        &self.message.signature
    }

    fn label(&self) -> &str {
        "ClientToQsMessage"
    }
}

impl VerifiedStruct<VerifiableClientToAsMessage> for AsRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToAsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct ClientToAsMessage {
    payload: ClientToQsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToAsMessage {
    pub(crate) fn auth_method(&self) -> AsAuthMethod {
        self.payload.auth_method()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct ClientToQsMessageTbs {
    version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: AsRequestParams,
}

impl ClientToQsMessageTbs {
    pub(crate) fn auth_method(&self) -> AsAuthMethod {
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
    DequeueMessages(DequeueMessagesParams),
    PublishKeyPackages(PublishKeyPackagesParams),
    ClientKeyPackage(ClientKeyPackageParams),
    UserClients(UserClientsParams),
    UserKeyPackages(UserKeyPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
}

impl AsRequestParams {
    pub(crate) fn auth_method(&self) -> AsAuthMethod {
        match self {
            AsRequestParams::Initiate2FaAuthentication(params) => {
                AsAuthMethod::ClientCredentialAuth(params.auth_method.clone())
            }
            AsRequestParams::InitUserRegistration(params) => {
                AsAuthMethod::NoAuth(params.auth_method.clone())
            }
            AsRequestParams::FinishUserRegistration(params) => {
                AsAuthMethod::Client2FaAuth(params.auth_method.clone())
            }
            AsRequestParams::DeleteUser(params) => {
                AsAuthMethod::Client2FaAuth(params.auth_method.clone())
            }
            AsRequestParams::InitiateClientAddition(params) => {
                AsAuthMethod::UserAuth(params.auth_method.clone())
            }
            AsRequestParams::FinishClientAddition(params) => {
                AsAuthMethod::Client2FaAuth(params.auth_method.clone())
            }
            AsRequestParams::DeleteClient(params) => {
                AsAuthMethod::ClientCredentialAuth(params.auth_method.clone())
            }
            AsRequestParams::DequeueMessages(params) => {
                AsAuthMethod::ClientCredentialAuth(params.auth_method.clone())
            }
            AsRequestParams::PublishKeyPackages(params) => {
                AsAuthMethod::ClientCredentialAuth(params.auth_method.clone())
            }
            AsRequestParams::ClientKeyPackage(params) => {
                AsAuthMethod::ClientCredentialAuth(params.auth_method.clone())
            }
            AsRequestParams::UserClients(params) => {
                AsAuthMethod::NoAuth(params.auth_method.clone())
            }
            AsRequestParams::UserKeyPackages(params) => {
                AsAuthMethod::NoAuth(params.auth_method.clone())
            }
            AsRequestParams::EnqueueMessage(params) => {
                AsAuthMethod::NoAuth(params.auth_method.clone())
            }
            AsRequestParams::AsCredentials(params) => {
                AsAuthMethod::NoAuth(params.auth_method.clone())
            }
        }
    }
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct NoAuth {}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCredentialAuth {
    pub(crate) client_id: AsClientId,
    pub(crate) signature: Signature,
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct Client2FaAuth {
    pub(crate) client_id: AsClientId,
    pub(crate) password: OpaqueKe3,
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserAuth {
    user_name: UserName,
    password: OpaqueKe3,
}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsAuthMethod {
    NoAuth(NoAuth),
    ClientCredentialAuth(ClientCredentialAuth),
    Client2FaAuth(Client2FaAuth),
    UserAuth(UserAuth),
}
