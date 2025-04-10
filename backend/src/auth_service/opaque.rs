// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use opaque_ke::{
    ServerSetup,
    rand::{CryptoRng, RngCore},
};
use phnxtypes::crypto::OpaqueCiphersuite;
use sqlx::PgExecutor;

use crate::errors::StorageError;

pub(super) struct OpaqueSetup(ServerSetup<OpaqueCiphersuite>);

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
    use phnxtypes::codec::{BlobDecoded, BlobEncoded};
    use sqlx::query_scalar;

    use super::*;

    impl OpaqueSetup {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO opaque_setup (opaque_setup) VALUES ($1)",
                BlobEncoded(&self.0) as _,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<ServerSetup<OpaqueCiphersuite>, StorageError> {
            // There is currently only one OPAQUE setup.
            query_scalar!(r#"SELECT opaque_setup AS "opaque_setup: _" FROM opaque_setup"#)
                .fetch_one(connection)
                .await
                .map(|BlobDecoded(setup)| setup)
                .map_err(From::from)
        }
    }

    #[cfg(test)]
    mod tests {
        use sqlx::PgPool;

        use super::*;

        #[sqlx::test]
        async fn load(pool: PgPool) -> anyhow::Result<()> {
            let mut rng = rand::thread_rng();
            let opaque_setup = OpaqueSetup(ServerSetup::<OpaqueCiphersuite>::new(&mut rng));

            opaque_setup.store(&pool).await?;
            let loaded = OpaqueSetup::load(&pool).await?;
            assert_eq!(loaded, opaque_setup.0);

            Ok(())
        }
    }
}
