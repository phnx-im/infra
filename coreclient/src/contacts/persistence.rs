// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::ear::keys::{
        AddPackageEarKey, ClientCredentialEarKey, SignatureEarKeyWrapperKey,
        WelcomeAttributionInfoEarKey,
    },
    identifiers::{AsClientId, QualifiedUserName},
    messages::FriendshipToken,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sqlx::{
    error::BoxDynError, query, query_as, Database, Decode, Executor, Sqlite, SqlitePool, Type,
};
use tokio_stream::StreamExt;

use crate::{
    clients::connection_establishment::FriendshipPackage, store::StoreNotifier,
    utils::persistence::Storable, Contact, ConversationId, PartialContact,
};

pub(crate) const CONTACT_INSERT_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_contact_overlap_on_insert;

    CREATE TRIGGER no_contact_overlap_on_insert
    BEFORE INSERT ON partial_contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t insert PartialContact: There already exists a contact with this user_name')
        END;
    END;";
pub(crate) const CONTACT_UPDATE_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_contact_overlap_on_update;

    CREATE TRIGGER no_contact_overlap_on_update
    BEFORE UPDATE ON partial_contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t update PartialContact: There already exists a contact with this user_name')
        END;
    END;";

impl Storable for Contact {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS contacts (
            user_name TEXT PRIMARY KEY,
            conversation_id BLOB NOT NULL,
            clients TEXT NOT NULL,
            wai_ear_key BLOB NOT NULL,
            friendship_token BLOB NOT NULL,
            add_package_ear_key BLOB NOT NULL,
            client_credential_ear_key BLOB NOT NULL,
            signature_ear_key_wrapper_key BLOB NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id)
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let user_name = row.get(0)?;
        let conversation_id = row.get(1)?;
        let clients_str: String = row.get(2)?;
        let clients = clients_str
            .split(',')
            .map(|s| AsClientId::try_from(s.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        let wai_ear_key = row.get(3)?;
        let friendship_token = row.get(4)?;
        let add_package_ear_key = row.get(5)?;
        let client_credential_ear_key = row.get(6)?;
        let signature_ear_key_wrapper_key = row.get(7)?;

        Ok(Contact {
            user_name,
            clients,
            wai_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key_wrapper_key,
            conversation_id,
        })
    }
}

/// Comma-separated list of [`AsClientId`]'s
struct SqlAsClientIds(Vec<AsClientId>);

impl Type<Sqlite> for SqlAsClientIds {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for SqlAsClientIds {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let clients_str = <&str as Decode<Sqlite>>::decode(value)?;
        let clients = clients_str
            .split(',')
            .map(|s| AsClientId::try_from(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(clients))
    }
}

struct SqlContact {
    user_name: QualifiedUserName,
    clients: SqlAsClientIds,
    wai_ear_key: WelcomeAttributionInfoEarKey,
    friendship_token: FriendshipToken,
    add_package_ear_key: AddPackageEarKey,
    client_credential_ear_key: ClientCredentialEarKey,
    signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    conversation_id: ConversationId,
}

impl From<SqlContact> for Contact {
    fn from(
        SqlContact {
            user_name,
            clients: SqlAsClientIds(clients),
            wai_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key_wrapper_key,
            conversation_id,
        }: SqlContact,
    ) -> Self {
        Self {
            user_name,
            clients,
            wai_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key_wrapper_key,
            conversation_id,
        }
    }
}

impl Contact {
    pub(crate) fn load(
        connection: &Connection,
        user_name: &QualifiedUserName,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM contacts WHERE user_name = ?")?;
        stmt.query_row([user_name], Self::from_row).optional()
    }

    pub(crate) async fn load_2(
        db: &SqlitePool,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlContact,
            r#"SELECT
                user_name AS "user_name: _",
                clients AS "clients: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                add_package_ear_key AS "add_package_ear_key: _",
                client_credential_ear_key AS "client_credential_ear_key: _",
                signature_ear_key_wrapper_key AS "signature_ear_key_wrapper_key: _",
                conversation_id AS "conversation_id: _"
            FROM contacts WHERE user_name = ?"#,
            user_name
        )
        .fetch_optional(db)
        .await
        .map(|res| res.map(From::from))
    }

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM contacts")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }

    pub(crate) async fn load_all_2(db: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        query_as!(
            SqlContact,
            r#"SELECT
                user_name AS "user_name: _",
                clients AS "clients: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                add_package_ear_key AS "add_package_ear_key: _",
                client_credential_ear_key AS "client_credential_ear_key: _",
                signature_ear_key_wrapper_key AS "signature_ear_key_wrapper_key: _",
                conversation_id AS "conversation_id: _"
            FROM contacts"#
        )
        .fetch(db)
        .map(|res| res.map(From::from))
        .collect()
        .await
    }

    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        let clients_str = self
            .clients
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        connection.execute(
            "INSERT INTO contacts (user_name, conversation_id, clients, wai_ear_key, friendship_token, add_package_ear_key, client_credential_ear_key, signature_ear_key_wrapper_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                self.user_name,
                self.conversation_id,
                clients_str,
                self.wai_ear_key,
                self.friendship_token,
                self.add_package_ear_key,
                self.client_credential_ear_key,
                self.signature_ear_key_wrapper_key,
            ],
        )?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn store_2(
        &self,
        executor: impl Executor<'_, Database = Sqlite>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        // TODO: Avoid creating Strings and collecting into a Vec.
        let clients_str = self
            .clients
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        query!(
            "INSERT INTO contacts
                (user_name, conversation_id, clients, wai_ear_key, friendship_token,
                add_package_ear_key, client_credential_ear_key, signature_ear_key_wrapper_key)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            self.user_name,
            self.conversation_id,
            clients_str,
            self.wai_ear_key,
            self.friendship_token,
            self.add_package_ear_key,
            self.client_credential_ear_key,
            self.signature_ear_key_wrapper_key,
        )
        .execute(executor)
        .await?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }
}

pub(crate) const PARTIAL_CONTACT_INSERT_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_insert;

    CREATE TRIGGER no_partial_contact_overlap_on_insert
    BEFORE INSERT ON contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM partial_contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t insert Contact: There already exists a partial contact with this user_name')
        END;
    END;";

pub(crate) const PARTIAL_CONTACT_UPDATE_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_update;

    CREATE TRIGGER no_partial_contact_overlap_on_update
    BEFORE UPDATE ON contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM partial_contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t update Contact: There already exists a partial contact with this user_name')
        END;
    END;";

impl Storable for PartialContact {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS partial_contacts (
            user_name TEXT PRIMARY KEY,
            conversation_id BLOB NOT NULL,
            friendship_package_ear_key BLOB NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id)
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let user_name = row.get(0)?;
        let conversation_id = row.get(1)?;
        let friendship_package_ear_key = row.get(2)?;

        Ok(PartialContact {
            user_name,
            conversation_id,
            friendship_package_ear_key,
        })
    }
}

impl PartialContact {
    pub(crate) fn load(
        connection: &Connection,
        user_name: &QualifiedUserName,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM partial_contacts WHERE user_name = ?")?;
        stmt.query_row([user_name], Self::from_row).optional()
    }

    pub(crate) async fn load_2(
        db: &SqlitePool,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            PartialContact,
            r#"SELECT
                user_name AS "user_name: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM partial_contacts WHERE user_name = ?"#,
            user_name
        )
        .fetch_optional(db)
        .await
    }

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM partial_contacts")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }

    pub(crate) async fn load_all_2(db: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        query_as!(
            PartialContact,
            r#"SELECT
                user_name AS "user_name: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM partial_contacts"#
        )
        .fetch_all(db)
        .await
    }

    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO partial_contacts (user_name, conversation_id, friendship_package_ear_key) VALUES (?, ?, ?)",
            params![
                self.user_name,
                self.conversation_id,
                self.friendship_package_ear_key,
            ],
        )?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn store_2(
        &self,
        db: &SqlitePool,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        query!(
            "INSERT INTO partial_contacts
                (user_name, conversation_id, friendship_package_ear_key)
                VALUES (?, ?, ?)",
            self.user_name,
            self.conversation_id,
            self.friendship_package_ear_key,
        )
        .execute(db)
        .await?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }

    fn delete(
        self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM partial_contacts WHERE user_name = ?",
            params![self.user_name],
        )?;
        notifier.remove(self.user_name.clone());
        Ok(())
    }

    pub(crate) async fn delete_2(
        self,
        executor: impl Executor<'_, Database = Sqlite>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM partial_contacts WHERE user_name = ?",
            self.user_name
        )
        .execute(executor)
        .await?;
        notifier.remove(self.user_name.clone());
        Ok(())
    }

    /// Creates a Contact from this PartialContact and the additional data. Then
    /// persists the resulting contact.
    pub(crate) fn mark_as_complete(
        self,
        transaction: &mut Transaction,
        notifier: &mut StoreNotifier,
        friendship_package: FriendshipPackage,
        client: AsClientId,
    ) -> Result<(), rusqlite::Error> {
        let savepoint = transaction.savepoint()?;

        let conversation_id = self.conversation_id;
        let user_name = self.user_name.clone();
        self.delete(&savepoint, notifier)?;
        let contact = Contact {
            user_name,
            clients: vec![client],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            client_credential_ear_key: friendship_package.client_credential_ear_key,
            signature_ear_key_wrapper_key: friendship_package.signature_ear_key_wrapper_key,
            conversation_id,
        };
        contact.store(&savepoint, notifier)?;

        savepoint.commit()?;

        Ok(())
    }

    pub(crate) async fn mark_as_complete_2(
        self,
        db: &SqlitePool,
        notifier: &mut StoreNotifier,
        friendship_package: FriendshipPackage,
        client: AsClientId,
    ) -> sqlx::Result<()> {
        let mut transaction = db.begin().await?;

        let user_name = self.user_name.clone();
        let conversation_id = self.conversation_id;

        self.delete_2(&mut *transaction, notifier).await?;
        let contact = Contact {
            user_name,
            clients: vec![client],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            client_credential_ear_key: friendship_package.client_credential_ear_key,
            signature_ear_key_wrapper_key: friendship_package.signature_ear_key_wrapper_key,
            conversation_id,
        };
        contact.store_2(&mut *transaction, notifier).await?;

        transaction.commit().await?;
        Ok(())
    }
}
