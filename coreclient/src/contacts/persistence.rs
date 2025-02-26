// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::{AsClientId, QualifiedUserName};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    Contact, PartialContact, clients::connection_establishment::FriendshipPackage,
    store::StoreNotifier, utils::persistence::Storable,
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
            key_package_ear_key BLOB NOT NULL,
            connection_key BLOB NOT NULL,
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
        let key_package_ear_key = row.get(5)?;
        let connection_key = row.get(6)?;

        Ok(Contact {
            user_name,
            clients,
            wai_ear_key,
            friendship_token,
            key_package_ear_key,
            connection_key,
            conversation_id,
        })
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

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM contacts")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
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
            "INSERT INTO contacts (
                user_name,
                conversation_id,
                clients,
                wai_ear_key,
                friendship_token,
                key_package_ear_key,
                connection_key)
            VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                self.user_name,
                self.conversation_id,
                clients_str,
                self.wai_ear_key,
                self.friendship_token,
                self.key_package_ear_key,
                self.connection_key,
            ],
        )?;
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
        connection
            .prepare("SELECT * FROM partial_contacts WHERE user_name = ?")?
            .query_row([user_name], Self::from_row)
            .optional()
    }

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Self>, rusqlite::Error> {
        connection
            .prepare("SELECT * FROM partial_contacts")?
            .query_map([], Self::from_row)?
            .collect()
    }

    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO partial_contacts (
                user_name,
                conversation_id,
                friendship_package_ear_key
            ) VALUES (?, ?, ?)",
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

    fn delete(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        user_name: &QualifiedUserName,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM partial_contacts WHERE user_name = ?",
            params![user_name],
        )?;
        notifier.remove(user_name.clone());
        Ok(())
    }

    /// Creates a Contact from this PartialContact and the additional data. Then
    /// persists the resulting contact.
    pub(crate) fn mark_as_complete(
        self,
        connection: &mut Connection,
        notifier: &mut StoreNotifier,
        friendship_package: FriendshipPackage,
        client: AsClientId,
    ) -> Result<Contact, rusqlite::Error> {
        let transaction = connection.transaction()?;

        let conversation_id = self.conversation_id;
        Self::delete(&transaction, notifier, &self.user_name)?;
        let contact = Contact {
            user_name: self.user_name,
            clients: vec![client],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            key_package_ear_key: friendship_package.key_package_ear_key,
            connection_key: friendship_package.connection_key,
            conversation_id,
        };
        contact.store(&transaction, notifier)?;

        transaction.commit()?;

        Ok(contact)
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::{
        crypto::{
            ear::keys::{FriendshipPackageEarKey, KeyPackageEarKey, WelcomeAttributionInfoEarKey},
            kdf::keys::ConnectionKey,
        },
        messages::FriendshipToken,
    };
    use uuid::Uuid;

    use crate::{
        Conversation, ConversationId, UserProfile,
        conversations::persistence::tests::test_conversation,
    };

    use super::*;

    fn test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                &[
                    Conversation::CREATE_TABLE_STATEMENT,
                    Contact::CREATE_TABLE_STATEMENT,
                    PartialContact::CREATE_TABLE_STATEMENT,
                ]
                .join("\n"),
            )
            .unwrap();

        connection
    }

    fn test_contact(conversation_id: ConversationId) -> Contact {
        let user_id = Uuid::new_v4();
        let user_name: QualifiedUserName = format!("{user_id}@localhost").parse().unwrap();
        Contact {
            user_name: user_name.clone(),
            clients: vec![AsClientId::new(user_name, user_id)],
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            friendship_token: FriendshipToken::random().unwrap(),
            key_package_ear_key: KeyPackageEarKey::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            conversation_id,
        }
    }

    fn test_partial_contact(conversation_id: ConversationId) -> PartialContact {
        let user_id = Uuid::new_v4();
        let user_name: QualifiedUserName = format!("{user_id}@localhost").parse().unwrap();
        PartialContact {
            user_name,
            conversation_id,
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        }
    }

    #[test]
    fn contact_store_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let contact = test_contact(conversation.id());
        contact.store(&connection, &mut store_notifier)?;

        let loaded = Contact::load(&connection, &contact.user_name)?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[test]
    fn partial_contact_store_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let contact = test_partial_contact(conversation.id());
        contact.store(&connection, &mut store_notifier)?;

        let loaded = PartialContact::load(&connection, &contact.user_name)?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[test]
    fn partial_contact_store_load_all() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let alice = test_partial_contact(conversation.id());
        let bob = test_partial_contact(conversation.id());

        alice.store(&connection, &mut store_notifier)?;
        bob.store(&connection, &mut store_notifier)?;

        let loaded = PartialContact::load_all(&connection)?;
        assert_eq!(loaded, [alice, bob]);

        Ok(())
    }

    #[test]
    fn partial_contact_mark_as_complete() -> anyhow::Result<()> {
        let mut connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let partial = test_partial_contact(conversation.id());
        let user_name = partial.user_name.clone();

        partial.store(&connection, &mut store_notifier)?;

        let friendship_package = FriendshipPackage {
            friendship_token: FriendshipToken::random().unwrap(),
            key_package_ear_key: KeyPackageEarKey::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            user_profile: UserProfile::new(user_name.clone(), None, None),
        };
        let contact = partial.mark_as_complete(
            &mut connection,
            &mut store_notifier,
            friendship_package,
            AsClientId::new(user_name.clone(), Uuid::new_v4()),
        )?;

        let loaded = PartialContact::load(&connection, &user_name)?;
        assert!(loaded.is_none());

        let loaded = Contact::load(&connection, &user_name)?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }
}
