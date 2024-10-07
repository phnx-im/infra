// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use opaque_ke::{
    rand::{CryptoRng, RngCore},
    ServerSetup,
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
    use phnxtypes::codec::PhnxCodec;

    use super::*;

    impl OpaqueSetup {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO opaque_setup (opaque_setup) VALUES ($1)",
                PhnxCodec::to_vec(&self.0)?
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<ServerSetup<OpaqueCiphersuite>, StorageError> {
            // There is currently only one OPAQUE setup.
            let opaque_setup_record = sqlx::query!("SELECT opaque_setup FROM opaque_setup")
                .fetch_one(connection)
                .await?;
            let opaque_setup = PhnxCodec::from_slice(&opaque_setup_record.opaque_setup)?;
            Ok(opaque_setup)
        }
    }
}
