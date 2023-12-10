// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_variables)]

use opaque_ke::ServerRegistration;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{ratchet::QueueRatchet, OpaqueCiphersuite, RatchetEncryptionKey},
    errors::auth_service::AsProcessingError,
    identifiers::UserName,
    messages::{
        client_as::{
            AsClientConnectionPackageResponse, AsCredentialsResponse, AsQueueMessagePayload,
            Init2FactorAuthResponse, InitClientAdditionResponse, InitUserRegistrationResponse,
            IssueTokensResponse, UserClientsResponse, UserConnectionPackagesResponse,
            VerifiedAsRequestParams,
        },
        client_qs::DequeueMessagesResponse,
        EncryptedAsQueueMessage,
    },
    time::TimeStamp,
};
use tls_codec::{TlsSerialize, TlsSize};

use self::{
    storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
    verification::VerifiableClientToAsMessage,
};

pub mod client_api;
pub mod devices;
pub mod invitations;
pub mod key_packages;
pub mod registration;
pub mod storage_provider_trait;
pub mod verification;

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

// === User ===

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

// === Client ===

#[derive(Debug, Clone)]
pub struct AsClientRecord {
    pub queue_encryption_key: RatchetEncryptionKey,
    pub ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
    pub activity_time: TimeStamp,
    pub credential: ClientCredential,
}

impl AsClientRecord {
    pub fn new(
        queue_encryption_key: RatchetEncryptionKey,
        ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
        activity_time: TimeStamp,
        credential: ClientCredential,
    ) -> Self {
        Self {
            queue_encryption_key,
            ratchet_key,
            activity_time,
            credential,
        }
    }
}

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
