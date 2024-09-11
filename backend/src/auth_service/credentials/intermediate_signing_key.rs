// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::openmls::prelude::SignatureScheme;
use phnxtypes::{
    credentials::{keys::AsIntermediateSigningKey, AsIntermediateCredentialCsr},
    identifiers::Fqdn,
};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection};

use crate::persistence::StorageError;

use super::{signing_key::SigningKey, CredentialGenerationError};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::auth_service) struct IntermediateSigningKey(AsIntermediateSigningKey);

impl Deref for IntermediateSigningKey {
    type Target = AsIntermediateSigningKey;

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
        let mut transaction = connection
            .begin()
            .await
            .map_err(StorageError::DatabaseError)?;

        // Load the currently active (root) signing key
        let signing_key = SigningKey::load(&mut *transaction)
            .await?
            .ok_or(CredentialGenerationError::NoActiveCredential)?;

        // Generate an intermediate credential CSR and sign it
        let (csr, prelim_signing_key) = AsIntermediateCredentialCsr::new(signature_scheme, domain)?;
        let as_intermediate_credential = csr.sign(&*signing_key, None).map_err(|e| {
            tracing::error!("Failed to sign intermediate credential: {:?}", e);
            CredentialGenerationError::SigningError
        })?;
        // We unwrap here, because we just created both the signing key and the credential, so we know they match.
        let as_intermediate_signing_key = AsIntermediateSigningKey::from_prelim_key(
            prelim_signing_key,
            as_intermediate_credential,
        )
        .unwrap();
        let intermediate_signing_key = IntermediateSigningKey(as_intermediate_signing_key);

        // Store the intermediate signing key
        intermediate_signing_key.store(&mut *transaction).await?;

        // Activate the intermediate signing key
        intermediate_signing_key.activate(&mut *transaction).await?;

        // Commit the transaction
        transaction
            .commit()
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(intermediate_signing_key)
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::PgExecutor;

    use crate::{auth_service::credentials::CredentialType, persistence::StorageError};

    use super::IntermediateSigningKey;

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
                self.0.credential().fingerprint().as_bytes(),
                PhnxCodec::to_vec(&self)?,
                false,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<IntermediateSigningKey>, StorageError> {
            sqlx::query!("SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = 'intermediate'")
                .fetch_optional(connection)
                .await?.map(|record| {
                    let signing_key = PhnxCodec::from_slice(&record.signing_key)?;
                    Ok(IntermediateSigningKey(signing_key))
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
                self.0.credential().fingerprint().as_bytes(),
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }
}
