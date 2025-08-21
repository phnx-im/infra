// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::{
    crypto::{
        ear::keys::WelcomeAttributionInfoEarKey, indexed_aead::keys::UserProfileKeyIndex,
        kdf::keys::ConnectionKey,
    },
    identifiers::{Fqdn, UserHandle, UserId},
    messages::FriendshipToken,
};
use chrono::Utc;
use sqlx::{SqliteExecutor, SqliteTransaction, query, query_as};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
    Contact, ConversationId, clients::connection_offer::FriendshipPackage, store::StoreNotifier,
};

use super::HandleContact;

struct SqlContact {
    user_uuid: Uuid,
    user_domain: Fqdn,
    conversation_id: ConversationId,
    wai_ear_key: WelcomeAttributionInfoEarKey,
    friendship_token: FriendshipToken,
    connection_key: ConnectionKey,
    user_profile_key_index: UserProfileKeyIndex,
}

impl From<SqlContact> for Contact {
    fn from(
        SqlContact {
            user_uuid,
            user_domain,
            wai_ear_key,
            friendship_token,
            conversation_id,
            connection_key,
            user_profile_key_index,
        }: SqlContact,
    ) -> Self {
        Self {
            user_id: UserId::new(user_uuid, user_domain),
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
        user_id: &UserId,
    ) -> sqlx::Result<Option<Self>> {
        let uuid = user_id.uuid();
        let domain = user_id.domain();
        query_as!(
            SqlContact,
            r#"SELECT
                user_uuid AS "user_uuid: _",
                user_domain AS "user_domain: _",
                conversation_id AS "conversation_id: _",
                wai_ear_key AS "wai_ear_key: _",
                friendship_token AS "friendship_token: _",
                connection_key AS "connection_key: _",
                user_profile_key_index AS "user_profile_key_index: _"
            FROM contacts WHERE user_uuid = ? AND user_domain = ?"#,
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
                user_uuid AS "user_uuid: _",
                user_domain AS "user_domain: _",
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

    pub(crate) async fn upsert(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let uuid = self.user_id.uuid();
        let domain = self.user_id.domain();
        query!(
            "INSERT OR REPLACE INTO contacts (
                user_uuid,
                user_domain,
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
            .add(self.user_id.clone())
            .update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn update_user_profile_key_index(
        executor: impl SqliteExecutor<'_>,
        user_id: &UserId,
        key_index: &UserProfileKeyIndex,
    ) -> sqlx::Result<()> {
        let uuid = user_id.uuid();
        let domain = user_id.domain();
        query!(
            "UPDATE contacts SET user_profile_key_index = ?
            WHERE user_uuid = ? AND user_domain = ?",
            key_index,
            uuid,
            domain,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl HandleContact {
    pub(crate) async fn upsert(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let created_at = Utc::now();
        query!(
            "INSERT OR REPLACE INTO user_handle_contacts (
                user_handle,
                conversation_id,
                friendship_package_ear_key,
                created_at
            ) VALUES (?, ?, ?, ?)",
            self.handle,
            self.conversation_id,
            self.friendship_package_ear_key,
            created_at,
        )
        .execute(executor)
        .await?;
        notifier.update(self.conversation_id);
        Ok(())
    }

    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            r#"SELECT
                user_handle AS "handle: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM user_handle_contacts
            WHERE user_handle = ?"#,
            handle,
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            r#"SELECT
                user_handle AS "handle: _",
                conversation_id AS "conversation_id: _",
                friendship_package_ear_key AS "friendship_package_ear_key: _"
            FROM user_handle_contacts"#,
        )
        .fetch_all(executor)
        .await
    }

    async fn delete(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        query!(
            "DELETE FROM user_handle_contacts WHERE user_handle = ?",
            self.handle
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Creates and persists a [`Contact`] from this [`HandleContact`] and the additional data
    pub(crate) async fn mark_as_complete(
        self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        user_id: UserId,
        friendship_package: FriendshipPackage,
        user_profile_key_index: UserProfileKeyIndex,
    ) -> anyhow::Result<Contact> {
        let contact = Contact {
            user_id,
            conversation_id: self.conversation_id,
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            connection_key: friendship_package.connection_key,
            user_profile_key_index,
        };

        self.delete(txn.as_mut()).await?;
        contact.upsert(txn.as_mut(), notifier).await?;

        Ok(contact)
    }
}

#[cfg(test)]
mod tests {
    use aircommon::{
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
        let user_id = UserId::random("localhost".parse().unwrap());
        let user_profile_key = UserProfileKey::random(&user_id).unwrap();
        let contact = Contact {
            user_id,
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            friendship_token: FriendshipToken::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            conversation_id,
            user_profile_key_index: user_profile_key.index().clone(),
        };
        (contact, user_profile_key)
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
        contact.upsert(&pool, &mut store_notifier).await?;

        let loaded = Contact::load(&pool, &contact.user_id).await?.unwrap();
        assert_eq!(loaded, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn handle_contact_upsert_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();
        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let handle = UserHandle::new("ellie_".to_owned()).unwrap();
        let handle_contact = HandleContact {
            handle: handle.clone(),
            conversation_id: conversation.id(),
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        };

        handle_contact.upsert(&pool, &mut store_notifier).await?;

        let loaded = HandleContact::load(&pool, &handle).await?.unwrap();
        assert_eq!(loaded, handle_contact);

        Ok(())
    }

    #[sqlx::test]
    async fn handle_contact_mark_as_complete(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();
        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let handle = UserHandle::new("ellie_".to_owned()).unwrap();
        let handle_contact = HandleContact {
            handle: handle.clone(),
            conversation_id: conversation.id(),
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        };

        let user_id = UserId::random("localhost".parse().unwrap());
        let user_profile_key = UserProfileKey::random(&user_id)?;
        user_profile_key.store(&pool).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: FriendshipToken::random().unwrap(),
            connection_key: ConnectionKey::random().unwrap(),
            wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
            user_profile_base_secret: user_profile_key.base_secret().clone(),
        };

        let mut txn = pool.begin().await?;

        let contact = handle_contact
            .mark_as_complete(
                &mut txn,
                &mut store_notifier,
                user_id,
                friendship_package,
                user_profile_key.index().clone(),
            )
            .await?;

        txn.commit().await?;

        let loaded_handle_contact = HandleContact::load(&pool, &handle).await?;
        assert!(loaded_handle_contact.is_none());

        let loaded_contact = Contact::load(&pool, &contact.user_id).await?.unwrap();
        assert_eq!(loaded_contact, contact);

        Ok(())
    }

    #[sqlx::test]
    async fn handle_contact_delete(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();
        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let handle = UserHandle::new("ellie_".to_owned()).unwrap();
        let handle_contact = HandleContact {
            handle: handle.clone(),
            conversation_id: conversation.id(),
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        };

        handle_contact.upsert(&pool, &mut store_notifier).await?;

        let mut txn = pool.begin().await?;
        handle_contact.delete(txn.as_mut()).await?;
        txn.commit().await?;

        let loaded = HandleContact::load(&pool, &handle).await?;
        assert!(loaded.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn handle_contact_upsert_idempotent(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();
        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let handle = UserHandle::new("ellie_".to_owned()).unwrap();
        let handle_contact = HandleContact {
            handle: handle.clone(),
            conversation_id: conversation.id(),
            friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
        };

        handle_contact.upsert(&pool, &mut store_notifier).await?;
        handle_contact.upsert(&pool, &mut store_notifier).await?; // Upsert again

        let loaded = HandleContact::load(&pool, &handle).await?.unwrap();
        assert_eq!(loaded, handle_contact);

        Ok(())
    }
}
