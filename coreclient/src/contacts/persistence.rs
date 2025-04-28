// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::{
        ear::keys::WelcomeAttributionInfoEarKey, indexed_aead::keys::UserProfileKeyIndex,
        kdf::keys::ConnectionKey,
    },
    identifiers::{AsClientId, QualifiedUserName},
    messages::FriendshipToken,
};
use sqlx::{
    Database, Decode, Sqlite, SqliteExecutor, SqlitePool, error::BoxDynError, prelude::Type, query,
    query_as,
};
use tokio_stream::StreamExt;

use crate::{
    Contact, ConversationId, PartialContact, clients::connection_establishment::FriendshipPackage,
    store::StoreNotifier,
};

/// Comma-separated list of [`AsClientId`]'s
struct SqlAsClientIds(Vec<AsClientId>);

impl Type<Sqlite> for SqlAsClientIds {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for SqlAsClientIds {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let clients_str: &str = Decode::<Sqlite>::decode(value)?;
        let clients = clients_str
            .split(',')
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(clients))
    }
}

struct SqlContact {
    user_name: QualifiedUserName,
    conversation_id: ConversationId,
    clients: SqlAsClientIds,
    wai_ear_key: WelcomeAttributionInfoEarKey,
    friendship_token: FriendshipToken,
    connection_key: ConnectionKey,
    user_profile_key_index: UserProfileKeyIndex,
}

impl From<SqlContact> for Contact {
    fn from(
        SqlContact {
            user_name,
            clients: SqlAsClientIds(clients),
            wai_ear_key,
            friendship_token,
            conversation_id,
            connection_key,
            user_profile_key_index,
        }: SqlContact,
    ) -> Self {
        Self {
            user_name,
            clients,
            wai_ear_key,
            friendship_token,
            connection_key,
            conversation_id,
            user_profile_key_index,
        }
    }
}

impl Contact {
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlContact,
            r#"SELECT
                user_name AS "user_name: _",
                conversation_id AS "conversation_id: _",
                clients AS "clients: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                connection_key AS "connection_key: _",
                user_profile_key_index AS "user_profile_key_index: _"
            FROM contacts WHERE user_name = ?"#,
            user_name
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(From::from))
    }

    pub(crate) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            SqlContact,
            r#"SELECT
                user_name AS "user_name: _",
                conversation_id AS "conversation_id: _",
                clients AS "clients: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                connection_key AS "connection_key: _",
                user_profile_key_index AS "user_profile_key_index: _"
            FROM contacts"#
        )
        .fetch(executor)
        .map(|res| res.map(From::from))
        .collect()
        .await
    }

    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
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
                connection_key, user_profile_key_index)
                VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.user_name,
            self.conversation_id,
            clients_str,
            self.wai_ear_key,
            self.friendship_token,
            self.connection_key,
            self.user_profile_key_index,
        )
        .execute(executor)
        .await?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }
}

impl PartialContact {
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
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
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            PartialContact,
            r#"SELECT
                user_name AS "user_name: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM partial_contacts"#
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
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
        .execute(executor)
        .await?;
        notifier
            .add(self.user_name.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn delete(
        self,
        executor: impl SqliteExecutor<'_>,
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
    pub(crate) async fn mark_as_complete(
        self,
        pool: &SqlitePool,
        notifier: &mut StoreNotifier,
        friendship_package: FriendshipPackage,
        client: AsClientId,
        user_profile_key_index: UserProfileKeyIndex,
    ) -> anyhow::Result<Contact> {
        let user_name = self.user_name.clone();
        let conversation_id = self.conversation_id;

        let contact = Contact {
            user_name,
            conversation_id,
            clients: vec![client],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            connection_key: friendship_package.connection_key,
            user_profile_key_index,
        };

        let mut transaction = pool.begin().await?;

        self.delete(&mut *transaction, notifier).await?;
        contact.store(&mut *transaction, notifier).await?;

        transaction.commit().await?;
        Ok(contact)
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::{
        crypto::{
            ear::keys::{FriendshipPackageEarKey, KeyPackageEarKey, WelcomeAttributionInfoEarKey},
            indexed_aead::keys::UserProfileKey,
            kdf::keys::ConnectionKey,
        },
        messages::FriendshipToken,
    };
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::{
        ConversationId, conversations::persistence::tests::test_conversation,
        key_stores::indexed_keys::StorableIndexedKey,
    };

    use super::*;

    fn test_contact(conversation_id: ConversationId) -> (Contact, UserProfileKey) {
        let user_id = Uuid::new_v4();
        let user_name: QualifiedUserName = format!("{user_id}@localhost").parse().unwrap();
        let user_profile_key = UserProfileKey::random(&user_name).unwrap();
        let contact = Contact {
            user_name: user_name.clone(),
            clients: vec![AsClientId::new(user_name, user_id)],
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            friendship_token: FriendshipToken::random().unwrap(),
            key_package_ear_key: KeyPackageEarKey::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            conversation_id,
            user_profile_key_index: user_profile_key.index().clone(),
        };
        (contact, user_profile_key)
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

    #[sqlx::test]
    async fn contact_store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let (contact, user_profile_key) = test_contact(conversation.id());
        user_profile_key.store(&pool).await?;
        contact.store(&pool, &mut store_notifier).await?;

        let loaded = Contact::load(&pool, &contact.user_name).await?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn partial_contact_store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let contact = test_partial_contact(conversation.id());
        contact.store(&pool, &mut store_notifier).await?;

        let loaded = PartialContact::load(&pool, &contact.user_name)
            .await?
            .unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn partial_contact_store_load_all(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let alice = test_partial_contact(conversation.id());
        let bob = test_partial_contact(conversation.id());

        alice.store(&pool, &mut store_notifier).await?;
        bob.store(&pool, &mut store_notifier).await?;

        let loaded = PartialContact::load_all(&pool).await?;
        assert_eq!(loaded, [alice, bob]);

        Ok(())
    }

    #[sqlx::test]
    async fn partial_contact_mark_as_complete(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let partial = test_partial_contact(conversation.id());
        let user_name = partial.user_name.clone();

        let user_profile_key = UserProfileKey::random(&user_name).unwrap();
        user_profile_key.store(&pool).await?;

        partial.store(&pool, &mut store_notifier).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: FriendshipToken::random().unwrap(),
            key_package_ear_key: KeyPackageEarKey::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            user_profile_base_secret: user_profile_key.base_secret().clone(),
        };
        let contact = partial
            .mark_as_complete(
                &pool,
                &mut store_notifier,
                friendship_package,
                AsClientId::new(user_name.clone(), Uuid::new_v4()),
                user_profile_key.index().clone(),
            )
            .await?;

        let loaded = PartialContact::load(&pool, &user_name).await?;
        assert!(loaded.is_none());

        let loaded = Contact::load(&pool, &user_name).await?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }
}
