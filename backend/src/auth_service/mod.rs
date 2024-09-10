// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_variables)]

use std::{collections::HashMap, sync::Arc};

use opaque_ke::ServerLogin;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{ratchet::QueueRatchet, OpaqueCiphersuite, RatchetEncryptionKey},
    errors::auth_service::AsProcessingError,
    identifiers::{AsClientId, QualifiedUserName},
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
use sqlx::{Executor, PgPool};
use tls_codec::{TlsSerialize, TlsSize};
use tokio::sync::Mutex;

use self::{storage_provider_trait::AsStorageProvider, verification::VerifiableClientToAsMessage};

pub mod client_api;
pub mod devices;
pub mod invitations;
pub mod key_packages;
mod privacy_pass;
pub mod registration;
mod signing_key;
pub mod storage_provider_trait;
mod user_record;
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

#[derive(Clone)]
pub struct AuthService {
    ephemeral_client_credentials: Arc<Mutex<HashMap<AsClientId, ClientCredential>>>,
    ephemeral_user_logins: Arc<Mutex<HashMap<QualifiedUserName, ServerLogin<OpaqueCiphersuite>>>>,
    ephemeral_client_logins: Arc<Mutex<HashMap<AsClientId, ServerLogin<OpaqueCiphersuite>>>>,
    db_pool: PgPool,
}

impl AuthService {
    pub async fn new(connection_string: &str, db_name: &str) -> Result<Self, sqlx::Error> {
        let connection = PgPool::connect(connection_string).await?;

        let db_exists = sqlx::query!(
            "select exists (
            SELECT datname FROM pg_catalog.pg_database WHERE datname = $1
        )",
            db_name,
        )
        .fetch_one(&connection)
        .await?;

        if !db_exists.exists.unwrap_or(false) {
            connection
                .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
                .await?;
        }

        let connection_string_with_db = format!("{}/{}", connection_string, db_name);

        let db_pool = PgPool::connect(&connection_string_with_db).await?;

        // Migrate database
        Self::new_from_pool(db_pool).await
    }

    async fn new_from_pool(db_pool: PgPool) -> Result<Self, sqlx::Error> {
        sqlx::migrate!("./migrations").run(&db_pool).await?;

        let ds = Self {
            db_pool,
            ephemeral_client_credentials: Arc::new(Mutex::new(HashMap::new())),
            ephemeral_user_logins: Arc::new(Mutex::new(HashMap::new())),
            ephemeral_client_logins: Arc::new(Mutex::new(HashMap::new())),
        };

        Ok(ds)
    }

    pub async fn process<Asp: AsStorageProvider>(
        &self,
        storage_provider: &Asp,
        message: VerifiableClientToAsMessage,
    ) -> Result<AsProcessResponse, AsProcessingError> {
        let verified_params = self.verify(storage_provider, message).await?;

        let response: AsProcessResponse = match verified_params {
            VerifiedAsRequestParams::Initiate2FaAuthentication(params) => self
                .as_init_two_factor_auth(storage_provider, params)
                .await
                .map(AsProcessResponse::Init2FactorAuth)?,
            VerifiedAsRequestParams::FinishUserRegistration(params) => {
                self.as_finish_user_registration(storage_provider, params)
                    .await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::DeleteUser(params) => {
                self.as_delete_user(params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::FinishClientAddition(params) => {
                self.as_finish_client_addition(storage_provider, params)
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
            VerifiedAsRequestParams::InitiateClientAddition(params) => self
                .as_init_client_addition(storage_provider, params)
                .await
                .map(AsProcessResponse::InitiateClientAddition)?,
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
            VerifiedAsRequestParams::InitUserRegistration(params) => self
                .as_init_user_registration(storage_provider, params)
                .await
                .map(AsProcessResponse::InitUserRegistration)?,
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
