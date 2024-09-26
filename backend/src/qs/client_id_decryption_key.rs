// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::crypto::hpke::ClientIdDecryptionKey;

pub(super) struct StorableClientIdDecryptionKey(ClientIdDecryptionKey);

impl Deref for StorableClientIdDecryptionKey {
    type Target = ClientIdDecryptionKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::PgExecutor;

    use crate::persistence::StorageError;

    use super::StorableClientIdDecryptionKey;

    impl StorableClientIdDecryptionKey {
        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query!("SELECT * FROM qs_decryption_key",)
                .fetch_optional(connection)
                .await?
                .map(|record| {
                    let decryption_key = PhnxCodec::from_slice(&record.decryption_key)?;
                    Ok(Self(decryption_key))
                })
                .transpose()
        }
    }
}
