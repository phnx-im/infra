// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use phnxapiclient::as_api::AsRequestError;
use phnxtypes::{
    credentials::{
        AsCredential, AsIntermediateCredential, ClientCredential, CredentialFingerprint,
        VerifiableClientCredential,
    },
    crypto::signatures::{signable::Verifiable, traits::SignatureVerificationError},
    identifiers::Fqdn,
};
use thiserror::Error;

use crate::{
    clients::api_clients::ApiClientsError,
    utils::persistence::{PersistableStruct, PersistenceError, SqlKey},
};

use super::*;

pub(crate) type PersistableAsCredential<'a> = PersistableStruct<'a, AsCredential>;

impl SqlKey for CredentialFingerprint {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl SqlKey for Fqdn {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl Persistable for AsCredential {
    type Key = CredentialFingerprint;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::AsCredential;

    fn key(&self) -> &Self::Key {
        &self.fingerprint()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.domain()
    }
}

pub(crate) type PersistableAsIntermediateCredential<'a> =
    PersistableStruct<'a, AsIntermediateCredential>;

impl PersistableAsIntermediateCredential<'_> {
    pub(crate) fn into_credential(self) -> AsIntermediateCredential {
        self.payload
    }
}

pub(crate) struct AsCredentialStore<'a> {
    db_connection: &'a Connection,
    api_clients: ApiClients,
}

impl<'a> AsCredentialStore<'a> {
    pub(crate) fn new(db_connection: &'a Connection, api_clients: ApiClients) -> Self {
        Self {
            db_connection,
            api_clients,
        }
    }

    /// Fetches the credentials of the AS with the given `domain` if they are
    /// not already present in the store.
    async fn fetch_credentials(
        &self,
        domain: &Fqdn,
    ) -> Result<Vec<PersistableAsIntermediateCredential<'a>>, AsCredentialStoreError> {
        let as_credentials_response = self.api_clients.get(&domain)?.as_as_credentials().await?;
        let as_credentials: HashMap<CredentialFingerprint, AsCredential> = as_credentials_response
            .as_credentials
            .into_iter()
            .filter_map(|credential| Some((credential.fingerprint().clone(), credential)))
            .collect::<HashMap<_, _>>();
        let mut as_inter_creds = vec![];
        for as_inter_cred in as_credentials_response.as_intermediate_credentials {
            let as_credential = as_credentials
                .get(as_inter_cred.signer_fingerprint())
                .ok_or(AsCredentialStoreError::AsCredentialNotFound)?;
            let verified_credential: AsIntermediateCredential =
                as_inter_cred.verify(as_credential.verifying_key())?;
            let p_as_inter_cred = PersistableAsIntermediateCredential::from_connection_and_payload(
                self.db_connection,
                verified_credential,
            );
            p_as_inter_cred.persist()?;
            as_inter_creds.push(p_as_inter_cred);
        }
        for as_credential in as_credentials.into_values() {
            let p_credential = PersistableAsCredential::from_connection_and_payload(
                self.db_connection,
                as_credential,
            );
            p_credential.persist()?;
        }
        Ok(as_inter_creds)
    }

    pub(crate) async fn get(
        &'a self,
        domain: &Fqdn,
        fingerprint: &CredentialFingerprint,
    ) -> Result<PersistableAsIntermediateCredential<'a>, AsCredentialStoreError> {
        log::info!("Loading AS credential from db.");
        let credential_option = PersistableAsIntermediateCredential::load_one(
            self.db_connection,
            Some(fingerprint),
            None,
        )?;
        let credential = if let Some(credential) = credential_option {
            credential
        } else {
            self.fetch_credentials(domain)
                .await?
                .into_iter()
                .find(|credential| credential.fingerprint() == fingerprint)
                .ok_or(AsCredentialStoreError::AsIntermediateCredentialNotFound)?
        };
        if credential.domain() != domain {
            return Err(AsCredentialStoreError::AsIntermediateCredentialNotFound);
        }
        Ok(credential)
    }

    pub async fn verify_client_credential<'b>(
        &'b self,
        verifiable_client_credential: VerifiableClientCredential,
    ) -> Result<ClientCredential, AsCredentialStoreError> {
        let as_intermediate_credential: PersistableAsIntermediateCredential<'b> = self
            .get(
                &verifiable_client_credential.domain(),
                verifiable_client_credential.signer_fingerprint(),
            )
            .await?;
        let client_credential =
            verifiable_client_credential.verify(as_intermediate_credential.verifying_key())?;
        Ok(client_credential)
    }

    pub(crate) async fn get_intermediate_credential(
        &self,
        domain: &Fqdn,
    ) -> Result<PersistableAsIntermediateCredential> {
        let credential = if let Some(credential) =
            PersistableAsIntermediateCredential::load_one(self.db_connection, None, None)?
        {
            credential
        } else {
            self.fetch_credentials(domain)
                .await?
                .pop()
                .ok_or(AsCredentialStoreError::AsIntermediateCredentialNotFound)?
        };
        Ok(credential)
    }
}

impl Persistable for AsIntermediateCredential {
    type Key = CredentialFingerprint;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::AsIntermediateCredential;

    fn key(&self) -> &Self::Key {
        self.fingerprint()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.domain()
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
    PersistenceError(#[from] PersistenceError),
    #[error(transparent)]
    ApiClientsError(#[from] ApiClientsError),
    #[error(transparent)]
    AsRequestError(#[from] AsRequestError),
}
