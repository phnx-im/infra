// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::crypto::indexed_aead::keys::{
    BaseSecret, Deletable, Index, IndexedAeadKey, Key, KeyType, KeyTypeInstance,
};
use sqlx::{Connection, SqliteConnection, SqliteExecutor, query, query_as};

pub(crate) struct SqlIndexedAeadKey<KT> {
    base_secret: BaseSecret<KT>,
    key: Key<KT>,
    index: Index<KT>,
}

impl<KT: KeyType> From<SqlIndexedAeadKey<KT>> for IndexedAeadKey<KT> {
    fn from(sql_key: SqlIndexedAeadKey<KT>) -> Self {
        IndexedAeadKey::from_parts(sql_key.base_secret, sql_key.key, sql_key.index)
    }
}

impl<KT: KeyType + Send + Unpin> StorableIndexedKey<KT> for IndexedAeadKey<KT> {
    fn base_secret(&self) -> &BaseSecret<KT> {
        self.base_secret()
    }

    fn key(&self) -> &Key<KT> {
        self.key()
    }

    fn index(&self) -> &Index<KT> {
        self.index()
    }
}

pub(crate) trait StorableIndexedKey<KT: KeyType + Send + Unpin>:
    From<SqlIndexedAeadKey<KT>>
{
    fn base_secret(&self) -> &BaseSecret<KT>;
    fn key(&self) -> &Key<KT>;
    fn index(&self) -> &Index<KT>;

    async fn store(&self, connection: impl SqliteExecutor<'_>) -> Result<(), sqlx::Error> {
        let base_secret = self.base_secret();
        let key = self.key();
        let index = self.index();
        query!(
            "INSERT OR IGNORE INTO indexed_keys (base_secret, key_value, key_index)
                VALUES ($1, $2, $3)",
            base_secret,
            key,
            index
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn store_own(&self, connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
        let base_secret = self.base_secret();
        let key = self.key();
        let index = self.index();
        let key_type = KeyTypeInstance::<KT>::new();
        let mut transaction = connection.begin().await?;
        query!(
            "INSERT OR IGNORE INTO indexed_keys (base_secret, key_value, key_index)
                VALUES ($1, $2, $3)",
            base_secret,
            key,
            index
        )
        .execute(&mut *transaction)
        .await?;
        query!(
            "INSERT OR IGNORE INTO own_key_indices (key_index, key_type) VALUES ($1, $2)",
            index,
            key_type
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn load(
        connection: impl SqliteExecutor<'_>,
        index: &Index<KT>,
    ) -> Result<Self, sqlx::Error> {
        query_as!(
            SqlIndexedAeadKey,
            r#"
                SELECT
                    base_secret AS "base_secret: _",
                    key_value AS "key: _",
                    key_index AS "index: _"
                FROM indexed_keys
                WHERE key_index = ?
                LIMIT 1"#,
            index,
        )
        .fetch_one(connection)
        .await
        .map(From::from)
    }

    async fn load_own(connection: impl SqliteExecutor<'_>) -> Result<Self, sqlx::Error> {
        let key_type = KeyTypeInstance::<KT>::new();
        query_as!(
            SqlIndexedAeadKey,
            r#"SELECT
                    ik.key_index as "index: _",
                    ik.key_value as "key: _",
                    ik.base_secret as "base_secret: _"
                FROM own_key_indices oki
                JOIN indexed_keys ik ON oki.key_index = ik.key_index
                WHERE oki.key_type = ?"#,
            key_type
        )
        .fetch_one(connection)
        .await
        .map(From::from)
    }
}

trait DeletableIndexedKey<KT: Deletable + Send + Unpin>: StorableIndexedKey<KT> {
    #[allow(dead_code)]
    async fn delete(&self, connection: impl SqliteExecutor<'_>) -> Result<(), sqlx::Error> {
        let index = self.index();
        query!("DELETE FROM indexed_keys WHERE key_index = $1", index)
            .execute(connection)
            .await?;
        Ok(())
    }
}
