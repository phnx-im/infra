// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_variables)]

use mls_assist::{openmls::prelude::KeyPackage, openmls_traits::random::OpenMlsRand};
use opaque_ke::{
    CredentialFinalization, CredentialRequest, CredentialResponse, RegistrationRequest,
    RegistrationResponse, RegistrationUpload, ServerRegistration,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{OpaqueCiphersuite, QueueRatchet, RandomnessError, RatchetEncryptionKey},
    ds::group_state::TimeStamp,
    messages::{
        client_as::{
            AsClientKeyPackageResponse, AsCredentialsResponse, AsDequeueMessagesResponse,
            Init2FactorAuthResponse, InitClientAdditionResponse, InitUserRegistrationResponse,
            IssueTokensResponse, UserClientsResponse, UserKeyPackagesResponse,
            VerifiedAsRequestParams,
        },
        client_as_out::VerifiableClientToAsMessage,
    },
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
pub mod username;

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
    client_message: RegistrationUpload<OpaqueCiphersuite>,
}

// === User ===

pub struct AsUserId {
    pub client_id: Vec<u8>,
}

pub struct AsUserRecord {
    user_name: UserName,
    password_file: ServerRegistration<OpaqueCiphersuite>,
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
}

impl From<Vec<u8>> for UserName {
    fn from(value: Vec<u8>) -> Self {
        Self { user_name: value }
    }
}

impl UserName {
    pub fn as_bytes(&self) -> &[u8] {
        &self.user_name
    }
}

impl From<String> for UserName {
    fn from(value: String) -> Self {
        Self {
            user_name: value.into_bytes(),
        }
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

impl AsClientId {
    pub fn random(
        backend: &impl OpenMlsRand,
        user_name: UserName,
    ) -> Result<Self, RandomnessError> {
        let client_id = backend
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self {
            user_name,
            client_id,
        })
    }

    pub fn username(&self) -> UserName {
        self.user_name.clone()
    }
}

pub struct AsClientRecord {
    pub queue_encryption_key: RatchetEncryptionKey,
    pub ratchet_key: QueueRatchet,
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
            VerifiedAsRequestParams::PublishKeyPackages(params) => {
                AuthService::as_publish_key_packages(storage_provider, params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::ClientKeyPackage(params) => {
                AuthService::as_client_key_package(storage_provider, params)
                    .await
                    .map(AsProcessResponse::ClientKeyPackage)?
            }
            VerifiedAsRequestParams::IssueTokens(params) => {
                AuthService::as_issue_tokens(storage_provider, params)
                    .await
                    .map(AsProcessResponse::IssueTokens)?
            }
            VerifiedAsRequestParams::UserKeyPackages(params) => {
                AuthService::as_user_key_package(storage_provider, params)
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
    DequeueMessages(AsDequeueMessagesResponse),
    ClientKeyPackage(AsClientKeyPackageResponse),
    IssueTokens(IssueTokensResponse),
    UserKeyPackages(UserKeyPackagesResponse),
    InitiateClientAddition(InitClientAdditionResponse),
    UserClients(UserClientsResponse),
    AsCredentials(AsCredentialsResponse),
    InitUserRegistration(InitUserRegistrationResponse),
}
