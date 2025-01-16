// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use phnxapiclient::as_api::AsRequestError;
use phnxtypes::{
    credentials::{
        AsCredential, AsCredentialBody, AsIntermediateCredential, AsIntermediateCredentialBody,
        ClientCredential, CredentialFingerprint, VerifiableClientCredential,
    },
    crypto::signatures::{signable::Verifiable, traits::SignatureVerificationError},
    identifiers::Fqdn,
};
use rusqlite::{params, OptionalExtension, ToSql};
use thiserror::Error;
use tracing::info;

use crate::{
    clients::api_clients::ApiClientsError,
    utils::persistence::{SqliteConnection, Storable},
};

use super::*;

pub(crate) enum AsCredentials {
    AsCredential(AsCredential),
    AsIntermediateCredential(AsIntermediateCredential),
}

impl Storable for AsCredentials {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS as_credentials (
            fingerprint TEXT PRIMARY KEY,
            domain TEXT NOT NULL,
            credential_type TEXT NOT NULL CHECK (credential_type IN ('as_credential', 'as_intermediate_credential')),
            credential BLOB NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let credential_type: String = row.get(0)?;
        match credential_type.as_str() {
            "as_credential" => {
                let body: AsCredentialBody = row.get(1)?;
                Ok(AsCredentials::AsCredential(AsCredential::from(body)))
            }
            "as_intermediate_credential" => {
                let body: AsIntermediateCredentialBody = row.get(1)?;
                Ok(AsCredentials::AsIntermediateCredential(
                    AsIntermediateCredential::from(body),
                ))
            }
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
}

impl AsCredentials {
    fn domain(&self) -> &Fqdn {
        match self {
            AsCredentials::AsCredential(credential) => credential.domain(),
            AsCredentials::AsIntermediateCredential(credential) => credential.domain(),
        }
    }

    fn fingerprint(&self) -> &CredentialFingerprint {
        match self {
            AsCredentials::AsCredential(credential) => credential.fingerprint(),
            AsCredentials::AsIntermediateCredential(credential) => credential.fingerprint(),
        }
    }

    fn credential_type(&self) -> &str {
        match self {
            AsCredentials::AsCredential(_) => "as_credential",
            AsCredentials::AsIntermediateCredential(_) => "as_intermediate_credential",
        }
    }

    fn body(&self) -> &dyn ToSql {
        match self {
            AsCredentials::AsCredential(credential) => credential.body(),
            AsCredentials::AsIntermediateCredential(credential) => credential.body(),
        }
    }

    fn store(&self, connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "INSERT OR REPLACE INTO as_credentials (fingerprint, domain, credential_type, credential) VALUES (?, ?, ?, ?)",
            params![self.fingerprint(), self.domain(), self.credential_type(), self.body()],
        )?;
        Ok(())
    }

    fn load_intermediate(
        connection: &Connection,
        fingerprint_option: Option<&CredentialFingerprint>,
        domain: &Fqdn,
    ) -> Result<Option<AsIntermediateCredential>, rusqlite::Error> {
        let mut query_string =
            "SELECT credential_type, credential FROM as_credentials WHERE domain = ? AND credential_type = 'as_intermediate_credential'".to_owned();
        if fingerprint_option.is_some() {
            query_string.push_str(" AND fingerprint = ?");
        }
        let mut statement = connection.prepare(&query_string)?;
        if let Some(fingerprint) = fingerprint_option {
            statement.query_row(params![domain, fingerprint], Self::from_row)
        } else {
            statement.query_row(params![domain], Self::from_row)
        }
        .optional()
        .map(|credential_type_option| {
            credential_type_option.and_then(|credential| {
                if let AsCredentials::AsIntermediateCredential(credential) = credential {
                    Some(credential)
                } else {
                    None
                }
            })
        })
    }

    async fn fetch_credentials(
        domain: &Fqdn,
        api_clients: &ApiClients,
    ) -> Result<Vec<AsIntermediateCredential>, AsCredentialStoreError> {
        let as_credentials_response = api_clients.get(domain)?.as_as_credentials().await?;
        let as_credentials: HashMap<CredentialFingerprint, AsCredential> = as_credentials_response
            .as_credentials
            .into_iter()
            .map(|credential| (credential.fingerprint().clone(), credential))
            .collect::<HashMap<_, _>>();
        let mut as_inter_creds = vec![];
        for as_inter_cred in as_credentials_response.as_intermediate_credentials {
            let as_credential = as_credentials
                .get(as_inter_cred.signer_fingerprint())
                .ok_or(AsCredentialStoreError::AsCredentialNotFound)?;
            let verified_credential = as_inter_cred.verify(as_credential.verifying_key())?;
            as_inter_creds.push(verified_credential);
        }
        Ok(as_inter_creds)
    }

    /// Fetches the credentials of the AS with the given `domain` if they are
    /// not already present in the store.
    pub(crate) async fn get(
        connection_mutex: SqliteConnection,
        api_clients: &ApiClients,
        domain: &Fqdn,
        fingerprint: &CredentialFingerprint,
    ) -> Result<AsIntermediateCredential, AsCredentialStoreError> {
        info!("Loading AS credential from db");
        // Phase 1: Check if there is a credential in the database.
        let connection = connection_mutex.lock().await;
        let credential_option =
            AsCredentials::load_intermediate(&connection, Some(fingerprint), domain)?;
        drop(connection);

        // Phase 2: If there is no credential in the database, fetch it from the AS.
        let credential = if let Some(credential) = credential_option {
            credential
        } else {
            // Phase 2a: Fetch the credential.
            let credential = Self::fetch_credentials(domain, api_clients)
                .await?
                .into_iter()
                .find(|credential| credential.fingerprint() == fingerprint)
                .ok_or(AsCredentialStoreError::AsIntermediateCredentialNotFound)?;

            // Phase 2b: Store it in the database.
            let credential_type = AsCredentials::AsIntermediateCredential(credential);
            let connection = connection_mutex.lock().await;
            credential_type.store(&connection)?;
            drop(connection);
            let AsCredentials::AsIntermediateCredential(credential) = credential_type else {
                unreachable!()
            };
            credential
        };
        if credential.domain() != domain {
            return Err(AsCredentialStoreError::AsIntermediateCredentialNotFound);
        }
        Ok(credential)
    }

    pub(crate) async fn get_intermediate_credential(
        connection: SqliteConnection,
        api_clients: &ApiClients,
        domain: &Fqdn,
    ) -> Result<AsIntermediateCredential, AsCredentialStoreError> {
        let connection = connection.lock().await;
        let credential_option = AsCredentials::load_intermediate(&connection, None, domain)?;
        drop(connection);
        match credential_option {
            Some(credential) => Ok(credential),
            None => {
                let mut credentials = Self::fetch_credentials(domain, api_clients).await?;
                let credential = credentials
                    .pop()
                    .ok_or(AsCredentialStoreError::AsIntermediateCredentialNotFound)?;
                Ok(credential)
            }
        }
    }

    pub async fn verify_client_credential(
        connection_mutex: SqliteConnection,
        api_clients: &ApiClients,
        verifiable_client_credential: VerifiableClientCredential,
    ) -> Result<ClientCredential, AsCredentialStoreError> {
        let as_intermediate_credential = Self::get(
            connection_mutex,
            api_clients,
            &verifiable_client_credential.domain(),
            verifiable_client_credential.signer_fingerprint(),
        )
        .await?;
        let client_credential =
            verifiable_client_credential.verify(as_intermediate_credential.verifying_key())?;
        Ok(client_credential)
    }
}

#[derive(Debug, Error)]
pub(crate) enum AsCredentialStoreError {
    #[error("Can't find AS credential for the given fingerprint")]
    AsCredentialNotFound,
    #[error("Can't find AS intermediate credential for the given fingerprint")]
    AsIntermediateCredentialNotFound,
    #[error(transparent)]
    VerificationError(#[from] SignatureVerificationError),
    #[error(transparent)]
    PersistenceError(#[from] rusqlite::Error),
    #[error(transparent)]
    ApiClientsError(#[from] ApiClientsError),
    #[error(transparent)]
    AsRequestError(#[from] AsRequestError),
}
