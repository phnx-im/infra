// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::{
        ear::keys::{FriendshipPackageEarKey, WelcomeAttributionInfoEarKey},
        indexed_aead::keys::UserProfileKeyIndex,
        kdf::keys::ConnectionKey,
    },
    identifiers::{Fqdn, UserId},
    messages::FriendshipToken,
};
use sqlx::{SqliteExecutor, SqliteTransaction, query, query_as};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
    Contact, ConversationId, PartialContact, clients::connection_establishment::FriendshipPackage,
    store::StoreNotifier,
};

struct SqlContact {
    as_client_uuid: Uuid,
    as_domain: Fqdn,
    conversation_id: ConversationId,
    wai_ear_key: WelcomeAttributionInfoEarKey,
    friendship_token: FriendshipToken,
    connection_key: ConnectionKey,
    user_profile_key_index: UserProfileKeyIndex,
}

impl From<SqlContact> for Contact {
    fn from(
        SqlContact {
            as_client_uuid,
            as_domain,
            wai_ear_key,
            friendship_token,
            conversation_id,
            connection_key,
            user_profile_key_index,
        }: SqlContact,
    ) -> Self {
        Self {
            client_id: UserId::new(as_client_uuid, as_domain),
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
        client_id: &UserId,
    ) -> sqlx::Result<Option<Self>> {
        let uuid = client_id.uuid();
        let domain = client_id.domain();
        query_as!(
            SqlContact,
            r#"SELECT
                as_client_uuid AS "as_client_uuid: _",
                as_domain AS "as_domain: _",
                conversation_id AS "conversation_id: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                connection_key AS "connection_key: _",
                user_profile_key_index AS "user_profile_key_index: _"
            FROM contacts WHERE as_client_uuid = ? AND as_domain = ?"#,
            uuid,
            domain
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(From::from))
    }

    pub(crate) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            SqlContact,
            r#"SELECT
                as_client_uuid AS "as_client_uuid: _",
                as_domain AS "as_domain: _",
                conversation_id AS "conversation_id: _",
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
        let uuid = self.client_id.uuid();
        let domain = self.client_id.domain();
        query!(
            "INSERT INTO contacts (
                as_client_uuid,
                as_domain,
                conversation_id,
                wai_ear_key,
                friendship_token,
                connection_key,
                user_profile_key_index
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            uuid,
            domain,
            self.conversation_id,
            self.wai_ear_key,
            self.friendship_token,
            self.connection_key,
            self.user_profile_key_index,
        )
        .execute(executor)
        .await?;
        notifier
            .add(self.client_id.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn update_user_profile_key_index(
        executor: impl SqliteExecutor<'_>,
        client_id: &UserId,
        key_index: &UserProfileKeyIndex,
    ) -> sqlx::Result<()> {
        let uuid = client_id.uuid();
        let domain = client_id.domain();
        query!(
            "UPDATE contacts SET user_profile_key_index = ?
            WHERE as_client_uuid = ? AND as_domain = ?",
            key_index,
            uuid,
            domain,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

struct SqlPartialContact {
    as_client_uuid: Uuid,
    as_domain: Fqdn,
    conversation_id: ConversationId,
    friendship_package_ear_key: FriendshipPackageEarKey,
}

impl From<SqlPartialContact> for PartialContact {
    fn from(
        SqlPartialContact {
            as_client_uuid,
            as_domain,
            conversation_id,
            friendship_package_ear_key,
        }: SqlPartialContact,
    ) -> Self {
        Self {
            client_id: UserId::new(as_client_uuid, as_domain),
            conversation_id,
            friendship_package_ear_key,
        }
    }
}

impl PartialContact {
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        client: &UserId,
    ) -> sqlx::Result<Option<Self>> {
        let uuid = client.uuid();
        let domain = client.domain();
        query_as!(
            SqlPartialContact,
            r#"SELECT
                as_client_uuid AS "as_client_uuid: _",
                as_domain AS "as_domain: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM partial_contacts
            WHERE as_client_uuid = ? AND as_domain = ?"#,
            uuid,
            domain,
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(From::from))
    }

    pub(crate) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        let contacts = query_as!(
            SqlPartialContact,
            r#"SELECT
                as_client_uuid AS "as_client_uuid: _",
                as_domain AS "as_domain: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM partial_contacts"#
        )
        .fetch_all(executor)
        .await?;
        Ok(contacts.into_iter().map(From::from).collect())
    }

    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let domain = self.client_id.domain();
        let uuid = self.client_id.uuid();
        query!(
            "INSERT INTO partial_contacts
                (as_client_uuid, as_domain, conversation_id, friendship_package_ear_key)
                VALUES (?, ?, ?, ?)",
            uuid,
            domain,
            self.conversation_id,
            self.friendship_package_ear_key,
        )
        .execute(executor)
        .await?;
        notifier
            .add(self.client_id.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn delete(
        self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let uuid = self.client_id.uuid();
        let domain = self.client_id.domain();
        query!(
            "DELETE FROM partial_contacts
            WHERE as_client_uuid = ? AND as_domain = ?",
            uuid,
            domain,
        )
        .execute(executor)
        .await?;
        notifier.remove(self.client_id.clone());
        Ok(())
    }

    /// Creates a Contact from this PartialContact and the additional data. Then
    /// persists the resulting contact.
    pub(crate) async fn mark_as_complete(
        self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        friendship_package: FriendshipPackage,
        user_profile_key_index: UserProfileKeyIndex,
    ) -> anyhow::Result<Contact> {
        let contact = Contact {
            client_id: self.client_id.clone(),
            conversation_id: self.conversation_id,
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            connection_key: friendship_package.connection_key,
            user_profile_key_index,
        };

        self.delete(txn.as_mut(), notifier).await?;
        contact.store(txn.as_mut(), notifier).await?;

        Ok(contact)
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::{
        crypto::{
            ear::keys::{FriendshipPackageEarKey, WelcomeAttributionInfoEarKey},
            indexed_aead::keys::UserProfileKey,
            kdf::keys::ConnectionKey,
        },
        messages::FriendshipToken,
    };
    use sqlx::SqlitePool;

    use crate::{
        ConversationId, conversations::persistence::tests::test_conversation,
        key_stores::indexed_keys::StorableIndexedKey,
    };

    use super::*;

    fn test_contact(conversation_id: ConversationId) -> (Contact, UserProfileKey) {
        let client_id = UserId::random("localhost".parse().unwrap());
        let user_profile_key = UserProfileKey::random(&client_id).unwrap();
        let contact = Contact {
            client_id,
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            friendship_token: FriendshipToken::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            conversation_id,
            user_profile_key_index: user_profile_key.index().clone(),
        };
        (contact, user_profile_key)
    }

    fn test_partial_contact(conversation_id: ConversationId) -> PartialContact {
        let client_id = UserId::random("localhost".parse().unwrap());
        PartialContact {
            client_id,
            conversation_id,
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        }
    }

    #[sqlx::test]
    async fn contact_store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let (contact, user_profile_key) = test_contact(conversation.id());
        user_profile_key.store(&pool).await?;
        contact.store(&pool, &mut store_notifier).await?;

        let loaded = Contact::load(&pool, &contact.client_id).await?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn partial_contact_store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let contact = test_partial_contact(conversation.id());
        contact.store(&pool, &mut store_notifier).await?;

        let loaded = PartialContact::load(&pool, &contact.client_id)
            .await?
            .unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn partial_contact_store_load_all(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let partial = test_partial_contact(conversation.id());
        let client_id = partial.client_id.clone();

        let user_profile_key = UserProfileKey::random(&partial.client_id)?;
        user_profile_key.store(&pool).await?;

        partial.store(&pool, &mut store_notifier).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: FriendshipToken::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            user_profile_base_secret: user_profile_key.base_secret().clone(),
        };

        let mut txn = pool.begin().await?;

        let contact = partial
            .mark_as_complete(
                &mut txn,
                &mut store_notifier,
                friendship_package,
                user_profile_key.index().clone(),
            )
            .await?;

        txn.commit().await?;

        let loaded = PartialContact::load(&pool, &client_id).await?;
        assert!(loaded.is_none());

        let loaded = Contact::load(&pool, &client_id).await?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }
}
