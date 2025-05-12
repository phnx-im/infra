// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use credentials::{
    CredentialGenerationError, intermediate_signing_key::IntermediateSigningKey,
    signing_key::StorableSigningKey,
};
use phnxtypes::{
    crypto::signatures::DEFAULT_SIGNATURE_SCHEME,
    errors::{auth_service::AsProcessingError, version::VersionError},
    identifiers::Fqdn,
    messages::{
        ApiVersion,
        client_as::{
            AsCredentialsResponse, IssueTokensResponse, RegisterUserResponse,
            SUPPORTED_AS_API_VERSIONS, UserClientsResponse, UserConnectionPackagesResponse,
            VerifiedAsRequestParams,
        },
        client_as_out::GetUserProfileResponse,
        client_qs::DequeueMessagesResponse,
    },
};
use sqlx::PgPool;
use thiserror::Error;
use tls_codec::{TlsSerialize, TlsSize};

use crate::{
    errors::StorageError,
    infra_service::{InfraService, ServiceCreationError},
};

pub mod client_api;
mod client_record;
mod connection_package;
mod credentials;
pub mod grpc;
mod privacy_pass;
mod queue;
mod user_record;
mod verification;

pub use verification::VerifiableClientToAsMessage;

/*
Actions:
ACTION_AS_INITIATE_2FA_AUTHENTICATION

User:
ACTION_AS_INIT_USER_REGISTRATION
ACTION_AS_DELETE_USER

Client:
ACTION_AS_DEQUEUE_MESSAGES
ACTION_AS_PUBLISH_KEY_PACKAGES
ACTION_AS_CLIENT_KEY_PACKAGE

Anonymous:
ACTION_AS_USER_CLIENTS
ACTION_AS_USER_KEY_PACKAGES
ACTION_AS_ENQUEUE_MESSAGE
ACTION_AS_CREDENTIALS
*/

#[derive(Clone)]
pub struct AuthService {
    db_pool: PgPool,
}

#[derive(Debug, Error)]
pub enum AuthServiceCreationError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("Error generating initial credentials")]
    Credential(#[from] CredentialGenerationError),
}

impl<T: Into<sqlx::Error>> From<T> for AuthServiceCreationError {
    fn from(e: T) -> Self {
        Self::Storage(StorageError::from(e.into()))
    }
}

impl InfraService for AuthService {
    async fn initialize(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError> {
        let auth_service = Self { db_pool };

        // Check if there is an active AS signing key
        let mut transaction = auth_service.db_pool.begin().await?;
        let active_signing_key_exists =
            StorableSigningKey::load(&mut *transaction).await?.is_some();

        if !active_signing_key_exists {
            let signature_scheme = DEFAULT_SIGNATURE_SCHEME;
            // Generate a new AS signing key
            StorableSigningKey::generate_store_and_activate(
                &mut transaction,
                domain.clone(),
                signature_scheme,
            )
            .await
            .map_err(ServiceCreationError::init_error)?;
            // Generate and sign an intermediate signing key
            IntermediateSigningKey::generate_sign_and_activate(
                &mut transaction,
                domain,
                signature_scheme,
            )
            .await
            .map_err(ServiceCreationError::init_error)?;
        }
        transaction.commit().await?;

        Ok(auth_service)
    }
}

impl AuthService {
    pub async fn process(
        &self,
        message: VerifiableClientToAsMessage,
    ) -> Result<AsVersionedProcessResponse, AsProcessingError> {
        let (verified_params, from_version) = self.verify(message).await?;

        let response: AsProcessResponse = match verified_params {
            VerifiedAsRequestParams::DeleteUser(params) => {
                self.as_delete_user(params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::DequeueMessages(params) => self
                .as_dequeue_messages(params)
                .await
                .map(AsProcessResponse::DequeueMessages)?,
            VerifiedAsRequestParams::PublishConnectionPackages(params) => {
                self.as_publish_connection_packages(params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::IssueTokens(params) => self
                .as_issue_tokens(params)
                .await
                .map(AsProcessResponse::IssueTokens)?,
            VerifiedAsRequestParams::UserConnectionPackages(params) => self
                .as_user_connection_packages(params)
                .await
                .map(AsProcessResponse::UserKeyPackages)?,
            VerifiedAsRequestParams::UserClients(params) => self
                .as_user_clients(params)
                .await
                .map(AsProcessResponse::UserClients)?,
            VerifiedAsRequestParams::AsCredentials(params) => self
                .as_credentials(params)
                .await
                .map(AsProcessResponse::AsCredentials)?,
            VerifiedAsRequestParams::EnqueueMessage(params) => {
                self.as_enqueue_message(params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::RegisterUser(params) => self
                .as_init_user_registration(params)
                .await
                .map(AsProcessResponse::UserRegistration)?,
            VerifiedAsRequestParams::GetUserProfile(params) => self
                .as_get_user_profile(params)
                .await
                .map(AsProcessResponse::GetUserProfile)?,
            VerifiedAsRequestParams::StageUserProfile(params) => {
                self.as_stage_user_profile(params).await?;
                AsProcessResponse::Ok
            }
            VerifiedAsRequestParams::MergeUserProfile(params) => {
                self.as_merge_user_profile(params).await?;
                AsProcessResponse::Ok
            }
        };

        Ok(AsVersionedProcessResponse::with_version(
            response,
            from_version,
        )?)
    }
}

#[derive(Debug)]
pub enum AsVersionedProcessResponse {
    Alpha(AsProcessResponse),
}

impl AsVersionedProcessResponse {
    pub(crate) fn version(&self) -> ApiVersion {
        match self {
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }

    pub(crate) fn with_version(
        response: AsProcessResponse,
        version: ApiVersion,
    ) -> Result<Self, VersionError> {
        match version.value() {
            1 => Ok(Self::Alpha(response)),
            _ => Err(VersionError::new(version, SUPPORTED_AS_API_VERSIONS)),
        }
    }
}

impl tls_codec::Size for AsVersionedProcessResponse {
    fn tls_serialized_len(&self) -> usize {
        match self {
            Self::Alpha(response) => {
                self.version().tls_value().tls_serialized_len() + response.tls_serialized_len()
            }
        }
    }
}

// Note: Manual implementation because `TlsSerialize` does not support custom variant tags.
impl tls_codec::Serialize for AsVersionedProcessResponse {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            Self::Alpha(response) => Ok(self.version().tls_value().tls_serialize(writer)?
                + response.tls_serialize(writer)?),
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsProcessResponse {
    Ok,
    DequeueMessages(DequeueMessagesResponse),
    IssueTokens(IssueTokensResponse),
    UserKeyPackages(UserConnectionPackagesResponse),
    UserClients(UserClientsResponse),
    AsCredentials(AsCredentialsResponse),
    UserRegistration(RegisterUserResponse),
    GetUserProfile(GetUserProfileResponse),
}
