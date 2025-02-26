// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::openmls::prelude::SignatureScheme;
use phnxtypes::{
    credentials::{
        AsIntermediateCredential, AsIntermediateCredentialCsr, CredentialFingerprint,
        keys::AsIntermediateSigningKey,
    },
    identifiers::Fqdn,
};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection};

use crate::errors::StorageError;

use super::{CredentialGenerationError, signing_key::StorableSigningKey};

#[derive(Serialize, Deserialize)]
pub(in crate::auth_service) enum IntermediateSigningKey {
    V1(AsIntermediateSigningKey),
}

impl From<IntermediateSigningKey> for AsIntermediateSigningKey {
    fn from(signing_key: IntermediateSigningKey) -> Self {
        match signing_key {
            IntermediateSigningKey::V1(signing_key) => signing_key,
        }
    }
}

impl From<AsIntermediateSigningKey> for IntermediateSigningKey {
    fn from(signing_key: AsIntermediateSigningKey) -> Self {
        IntermediateSigningKey::V1(signing_key)
    }
}

pub(in crate::auth_service) struct IntermediateCredential(AsIntermediateCredential);

impl Deref for IntermediateCredential {
    type Target = AsIntermediateCredential;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntermediateSigningKey {
    pub(in crate::auth_service) async fn generate_sign_and_activate(
        connection: &mut PgConnection,
        domain: Fqdn,
        signature_scheme: SignatureScheme,
    ) -> Result<Self, CredentialGenerationError> {
        // Start the transaction
        let mut transaction = connection.begin().await.map_err(StorageError::from)?;

        // Load the currently active (root) signing key
        let signing_key = StorableSigningKey::load(&mut *transaction)
            .await?
            .ok_or(CredentialGenerationError::NoActiveCredential)?;

        // Generate an intermediate credential CSR and sign it
        let (csr, prelim_signing_key) = AsIntermediateCredentialCsr::new(signature_scheme, domain)?;
        let as_intermediate_credential = csr.sign(&signing_key, None).map_err(|e| {
            tracing::error!("Failed to sign intermediate credential: {:?}", e);
            CredentialGenerationError::SigningError
        })?;
        // We unwrap here, because we just created both the signing key and the credential, so we know they match.
        let as_intermediate_signing_key = AsIntermediateSigningKey::from_prelim_key(
            prelim_signing_key,
            as_intermediate_credential,
        )
        .unwrap();
        let intermediate_signing_key = IntermediateSigningKey::from(as_intermediate_signing_key);

        // Store the intermediate signing key
        intermediate_signing_key.store(&mut *transaction).await?;

        // Activate the intermediate signing key
        intermediate_signing_key.activate(&mut *transaction).await?;

        // Commit the transaction
        transaction.commit().await.map_err(StorageError::from)?;

        Ok(intermediate_signing_key)
    }

    fn fingerprint(&self) -> &CredentialFingerprint {
        match self {
            IntermediateSigningKey::V1(signing_key) => signing_key.credential().fingerprint(),
        }
    }
}

mod persistence {
    use phnxtypes::{
        codec::PhnxCodec,
        credentials::{AsIntermediateCredential, keys::AsIntermediateSigningKey},
    };
    use sqlx::PgExecutor;

    use crate::{auth_service::credentials::CredentialType, errors::StorageError};

    use super::{IntermediateCredential, IntermediateSigningKey};

    impl IntermediateSigningKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO
                    as_signing_keys
                    (cred_type, credential_fingerprint, signing_key, currently_active)
                VALUES 
                    ($1, $2, $3, $4)",
                CredentialType::Intermediate as _,
                self.fingerprint().as_bytes(),
                PhnxCodec::to_vec(&self)?,
                false,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<AsIntermediateSigningKey>, StorageError> {
            sqlx::query!("SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = 'intermediate'")
                .fetch_optional(connection)
                .await?.map(|record| {
                    let signing_key: IntermediateSigningKey = PhnxCodec::from_slice(&record.signing_key)?;
                    Ok(signing_key.into())
                }).transpose()
        }

        pub(super) async fn activate(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "UPDATE as_signing_keys
                SET currently_active = CASE
                    WHEN credential_fingerprint = $1 THEN true
                    ELSE false
                END
                WHERE cred_type = 'intermediate'",
                self.fingerprint().as_bytes(),
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }

    impl IntermediateCredential {
        pub(in crate::auth_service) async fn load_all(
            connection: impl PgExecutor<'_>,
        ) -> Result<Vec<AsIntermediateCredential>, StorageError> {
            let records = sqlx::query!(
                "SELECT signing_key FROM as_signing_keys WHERE cred_type = $1",
                CredentialType::Intermediate as _,
            )
            .fetch_all(connection)
            .await?;
            let credentials = records
                .into_iter()
                .map(|record| {
                    let signing_key: IntermediateSigningKey =
                        PhnxCodec::from_slice(&record.signing_key)?;
                    let as_signing_key = AsIntermediateSigningKey::from(signing_key);
                    Ok(as_signing_key.credential().clone())
                })
                .collect::<Result<Vec<_>, StorageError>>()?;
            Ok(credentials)
        }
    }
}
