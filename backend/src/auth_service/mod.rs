// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_variables)]

use opaque_ke::{
    CredentialFinalization, CredentialRequest, CredentialResponse, RegistrationRequest,
    RegistrationResponse, RegistrationUpload, ServerRegistration,
};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes as TlsDeserializeTrait, Serialize as TlsSerialize, TlsDeserializeBytes,
    TlsSerialize, TlsSize,
};

use crate::{
    crypto::{ratchet::QueueRatchet, OpaqueCiphersuite, RandomnessError, RatchetEncryptionKey},
    ds::group_state::TimeStamp,
    messages::{
        client_as::{
            AsClientConnectionPackageResponse, AsCredentialsResponse, AsQueueMessagePayload,
            Init2FactorAuthResponse, InitClientAdditionResponse, InitUserRegistrationResponse,
            IssueTokensResponse, UserClientsResponse, UserConnectionPackagesResponse,
            VerifiedAsRequestParams,
        },
        client_as_out::VerifiableClientToAsMessage,
        client_qs::DequeueMessagesResponse,
        EncryptedAsQueueMessage,
    },
    qs::Fqdn,
};

use self::{
    credentials::ClientCredential,
    errors::AsProcessingError,
    storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
};

pub mod client_api;
pub mod codec;
pub mod credentials;
pub mod devices;
pub mod errors;
pub mod invitations;
pub mod key_packages;
pub mod registration;
pub mod storage_provider_trait;

/*
Actions:
ACTION_AS_INITIATE_2FA_AUTHENTICATION

User:
ACTION_AS_INIT_USER_REGISTRATION
ACTION_AS_FINISH_USER_REGISTRATION
ACTION_AS_DELETE_USER

Client:
ACTION_AS_INITIATE_CLIENT_ADDITION
ACTION_AS_FINISH_CLIENT_ADDITION
ACTION_AS_DELETE_CLIENT
ACTION_AS_DEQUEUE_MESSAGES
ACTION_AS_PUBLISH_KEY_PACKAGES
ACTION_AS_CLIENT_KEY_PACKAGE

Anonymous:
ACTION_AS_USER_CLIENTS
ACTION_AS_USER_KEY_PACKAGES
ACTION_AS_ENQUEUE_MESSAGE
ACTION_AS_CREDENTIALS
*/

// === Authentication ===

#[derive(Debug)]
pub struct OpaqueLoginRequest {
    client_message: CredentialRequest<OpaqueCiphersuite>,
}

#[derive(Debug)]
pub struct OpaqueLoginResponse {
    server_message: CredentialResponse<OpaqueCiphersuite>,
}

#[derive(Clone, Debug)]
pub struct OpaqueLoginFinish {
    pub(crate) client_message: CredentialFinalization<OpaqueCiphersuite>,
}

/// Registration request containing the OPAQUE payload.
///
/// The TLS serialization implementation of this
#[derive(Debug)]
pub struct OpaqueRegistrationRequest {
    pub client_message: RegistrationRequest<OpaqueCiphersuite>,
}

#[derive(Debug)]
pub struct OpaqueRegistrationResponse {
    pub server_message: RegistrationResponse<OpaqueCiphersuite>,
}

impl From<RegistrationResponse<OpaqueCiphersuite>> for OpaqueRegistrationResponse {
    fn from(value: RegistrationResponse<OpaqueCiphersuite>) -> Self {
        Self {
            server_message: value,
        }
    }
}

#[derive(Debug)]
pub struct OpaqueRegistrationRecord {
    pub client_message: RegistrationUpload<OpaqueCiphersuite>,
}

// === User ===

pub struct AsUserId {
    pub client_id: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AsUserRecord {
    _user_name: UserName,
    password_file: ServerRegistration<OpaqueCiphersuite>,
}

impl AsUserRecord {
    pub fn new(user_name: UserName, password_file: ServerRegistration<OpaqueCiphersuite>) -> Self {
        Self {
            _user_name: user_name,
            password_file,
        }
    }
}

#[derive(
    Clone,
    Debug,
    TlsDeserializeBytes,
    TlsSerialize,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct UserName {
    pub(crate) user_name: Vec<u8>,
    pub(crate) domain: Fqdn,
}

impl From<Vec<u8>> for UserName {
    fn from(value: Vec<u8>) -> Self {
        Self::tls_deserialize_exact(&value).unwrap()
    }
}

// TODO: This string processing is way too simplistic, but it should do for now.
impl From<&str> for UserName {
    fn from(value: &str) -> Self {
        let mut split_name = value.split('@');
        let name = split_name.next().unwrap();
        // UserNames MUST be qualified
        let domain = split_name.next().unwrap();
        assert!(split_name.next().is_none());
        let domain = domain.into();
        let user_name = name.as_bytes().to_vec();
        Self { user_name, domain }
    }
}

impl UserName {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.tls_serialize_detached().unwrap()
    }

    pub fn domain(&self) -> Fqdn {
        self.domain.clone()
    }
}

impl From<String> for UserName {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&String> for UserName {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}",
            String::from_utf8_lossy(&self.user_name),
            self.domain
        )
    }
}

// === Client ===

#[derive(
    Clone,
    Debug,
    TlsDeserializeBytes,
    TlsSerialize,
    TlsSize,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
)]
pub struct AsClientId {
    pub(crate) user_name: UserName,
    pub(crate) client_id: Vec<u8>,
}

impl AsRef<[u8]> for AsClientId {
    fn as_ref(&self) -> &[u8] {
        &self.client_id
    }
}

impl std::fmt::Display for AsClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let client_id_str = String::from_utf8_lossy(&self.client_id);
        write!(f, "{}.{}", client_id_str, self.user_name)
    }
}

impl AsClientId {
    pub fn random(user_name: UserName) -> Result<Self, RandomnessError> {
        // TODO: Use a proper rng provider.
        let mut rng = rand_chacha::ChaCha20Rng::from_entropy();
        let valid_characters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

        let length = 16;

        // Generate a random string as client id
        let client_id: String = (0..length)
            .map(|_| {
                let index = rng.gen_range(0..valid_characters.len());
                valid_characters.chars().nth(index).unwrap_or('a')
            })
            .collect();
        Ok(Self {
            user_name,
            client_id: client_id.into_bytes(),
        })
    }

    pub fn user_name(&self) -> UserName {
        self.user_name.clone()
    }
}

#[derive(Debug, Clone)]
pub struct AsClientRecord {
    pub queue_encryption_key: RatchetEncryptionKey,
    pub ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
    pub activity_time: TimeStamp,
    pub credential: ClientCredential,
}

impl AsClientRecord {}

pub struct AuthService {}

impl AuthService {
    pub async fn process<Asp: AsStorageProvider, Eph: AsEphemeralStorageProvider>(
        storage_provider: &Asp,
        ephemeral_storage_provider: &Eph,
        message: VerifiableClientToAsMessage,
    ) -> Result<AsProcessResponse, AsProcessingError> {
        let verified_params = message
            .verify(storage_provider, ephemeral_storage_provider)
            .await?;

        let response: AsProcessResponse = match verified_params {
            VerifiedAsRequestParams::Initiate2FaAuthentication(params) => {
                AuthService::as_init_two_factor_auth(
                    storage_provider,
                    ephemeral_storage_provider,
                    params,
                )
                .await
                .map(AsProcessResponse::Init2FactorAuth)?
            }
            VerifiedAsRequestParams::FinishUserRegistration(params) => {
                AuthService::as_finish_user_registration(
                    storage_provider,
                    ephemeral_storage_provider,
                    params,
                )
                .await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::DeleteUser(params) => {
                AuthService::as_delete_user(storage_provider, params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::FinishClientAddition(params) => {
                AuthService::as_finish_client_addition(
                    storage_provider,
                    ephemeral_storage_provider,
                    params,
                )
                .await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::DeleteClient(params) => {
                AuthService::as_delete_client(storage_provider, params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::DequeueMessages(params) => {
                AuthService::as_dequeue_messages(storage_provider, params)
                    .await
                    .map(AsProcessResponse::DequeueMessages)?
            }
            VerifiedAsRequestParams::PublishConnectionPackages(params) => {
                AuthService::as_publish_connection_packages(storage_provider, params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::ClientConnectionPackage(params) => {
                AuthService::as_client_key_package(storage_provider, params)
                    .await
                    .map(AsProcessResponse::ClientKeyPackage)?
            }
            VerifiedAsRequestParams::IssueTokens(params) => {
                AuthService::as_issue_tokens(storage_provider, params)
                    .await
                    .map(AsProcessResponse::IssueTokens)?
            }
            VerifiedAsRequestParams::UserConnectionPackages(params) => {
                AuthService::as_user_connection_packages(storage_provider, params)
                    .await
                    .map(AsProcessResponse::UserKeyPackages)?
            }
            VerifiedAsRequestParams::InitiateClientAddition(params) => {
                AuthService::as_init_client_addition(
                    storage_provider,
                    ephemeral_storage_provider,
                    params,
                )
                .await
                .map(AsProcessResponse::InitiateClientAddition)?
            }
            VerifiedAsRequestParams::UserClients(params) => {
                AuthService::as_user_clients(storage_provider, params)
                    .await
                    .map(AsProcessResponse::UserClients)?
            }
            VerifiedAsRequestParams::AsCredentials(params) => {
                AuthService::as_credentials(storage_provider, params)
                    .await
                    .map(AsProcessResponse::AsCredentials)?
            }
            VerifiedAsRequestParams::EnqueueMessage(params) => {
                AuthService::as_enqueue_message(storage_provider, params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::InitUserRegistration(params) => {
                AuthService::as_init_user_registration(
                    storage_provider,
                    ephemeral_storage_provider,
                    params,
                )
                .await
                .map(AsProcessResponse::InitUserRegistration)?
            }
        };
        Ok(response)
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsProcessResponse {
    Ok,
    Init2FactorAuth(Init2FactorAuthResponse),
    DequeueMessages(DequeueMessagesResponse),
    ClientKeyPackage(AsClientConnectionPackageResponse),
    IssueTokens(IssueTokensResponse),
    UserKeyPackages(UserConnectionPackagesResponse),
    InitiateClientAddition(InitClientAdditionResponse),
    UserClients(UserClientsResponse),
    AsCredentials(AsCredentialsResponse),
    InitUserRegistration(InitUserRegistrationResponse),
}
