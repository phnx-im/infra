// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::keypackage_batch::QsEncryptedAddPackage;

pub(super) struct StorableEncryptedAddPackage(QsEncryptedAddPackage);

impl Deref for StorableEncryptedAddPackage {
    type Target = QsEncryptedAddPackage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<QsEncryptedAddPackage> for StorableEncryptedAddPackage {
    fn from(encrypted_add_package: QsEncryptedAddPackage) -> Self {
        Self(encrypted_add_package)
    }
}

mod persistence {
    use phnxtypes::identifiers::QsClientId;
    use sqlx::{postgres::PgArguments, Arguments, PgExecutor};
    use uuid::Uuid;

    use super::*;
    impl StorableEncryptedAddPackage {
        pub(in crate::qs) async fn store_multiple(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_add_packages: impl IntoIterator<Item = impl Deref<Target = QsEncryptedAddPackage>>,
        ) -> Result<(), sqlx::Error> {
            Self::store_multiple_internal(connection, client_id, encrypted_add_packages, false)
                .await
        }

        pub(in crate::qs) async fn store_last_resort(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_add_package: impl Deref<Target = QsEncryptedAddPackage>,
        ) -> Result<(), sqlx::Error> {
            Self::store_multiple_internal(connection, client_id, [encrypted_add_package], true)
                .await
        }

        async fn store_multiple_internal(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_add_packages: impl IntoIterator<Item = impl Deref<Target = QsEncryptedAddPackage>>,
            is_last_resort: bool,
        ) -> Result<(), sqlx::Error> {
            // TODO: This can probably be improved. For now, we insert each key
            // package individually.

            let mut query_args = PgArguments::default();
            let mut query_string = String::from(
                "INSERT INTO key_packages (id, client_id, encrypted_add_package, is_last_resort) VALUES",
            );

            for (i, encrypted_add_package) in encrypted_add_packages.into_iter().enumerate() {
                let id = Uuid::new_v4();
                //let encoded_add_package = PhnxCodec::to_vec(encrypted_add_package)?;

                // Add values to the query arguments. None of these should throw an error.
                let _ = query_args.add(id);
                let _ = query_args.add(&client_id);
                let _ = query_args.add(&*encrypted_add_package);
                let _ = query_args.add(is_last_resort);

                if i > 0 {
                    query_string.push(',');
                }

                // Add placeholders for each value
                query_string.push_str(&format!(
                    " (${}, ${}, ${}, ${})",
                    i * 4 + 1,
                    i * 4 + 2,
                    i * 4 + 3,
                    i * 4 + 4
                ));
            }

            // Finalize the query string
            query_string.push(';');

            // Execute the query
            sqlx::query_with(&query_string, query_args)
                .execute(connection)
                .await?;

            Ok(())
        }
    }
}
