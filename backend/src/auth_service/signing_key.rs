// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::credentials::keys::AsIntermediateSigningKey;

pub(super) struct IntermediateSigningKey(AsIntermediateSigningKey);

impl Deref for IntermediateSigningKey {
    type Target = AsIntermediateSigningKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::PgExecutor;

    use crate::persistence::StorageError;

    use super::IntermediateSigningKey;

    impl IntermediateSigningKey {
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
    }
}
