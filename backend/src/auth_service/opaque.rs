// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use opaque_ke::{
    rand::{CryptoRng, RngCore},
    ServerSetup,
};
use phnxtypes::{codec::persist::BlobPersist, crypto::OpaqueCiphersuite, mark_as_blob_persist};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

use crate::errors::StorageError;

#[derive(Serialize, Deserialize)]
pub(super) struct OpaqueSetup(ServerSetup<OpaqueCiphersuite>);

mark_as_blob_persist!(OpaqueSetup);

impl Deref for OpaqueSetup {
    type Target = ServerSetup<OpaqueCiphersuite>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OpaqueSetup {
    pub(super) async fn new_and_store<'a>(
        connection: impl PgExecutor<'a>,
        rng: &mut (impl CryptoRng + RngCore),
    ) -> Result<Self, StorageError> {
        let opaque_setup = OpaqueSetup(ServerSetup::<OpaqueCiphersuite>::new(rng));
        opaque_setup.store(connection).await?;
        Ok(opaque_setup)
    }
}

mod persistence {
    use phnxtypes::codec::persist::BlobPersisted;

    use super::*;

    impl OpaqueSetup {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO opaque_setup (opaque_setup) VALUES ($1)",
                self.persisting() as _
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<ServerSetup<OpaqueCiphersuite>, StorageError> {
            // There is currently only one OPAQUE setup.
            let BlobPersisted(OpaqueSetup(value)) = sqlx::query_scalar!(
                r#"SELECT opaque_setup AS "opaque_setup: _" FROM opaque_setup"#
            )
            .fetch_one(connection)
            .await?;
            Ok(value)
        }
    }
}
