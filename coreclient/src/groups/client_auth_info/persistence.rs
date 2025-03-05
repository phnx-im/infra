// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::{group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    codec::persist::BlobPersist,
    credentials::CredentialFingerprint,
    crypto::ear::keys::{IdentityLinkKey, IdentityLinkKeySecret},
    identifiers::{AsClientId, QualifiedUserName},
};
use sqlx::{query, query_as, query_scalar, Row, SqliteExecutor};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable};

use super::{GroupMembership, StorableClientCredential};

impl Storable for StorableClientCredential {
    const CREATE_TABLE_STATEMENT: &str = "CREATE TABLE IF NOT EXISTS client_credentials (
                fingerprint BLOB PRIMARY KEY,
                client_id TEXT NOT NULL,
                client_credential BLOB NOT NULL
            );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let client_credential = row.get(0)?;
        Ok(Self::new(client_credential))
    }
}

impl StorableClientCredential {
    /// Load the [`StorableClientCredential`] with the given
    /// [`CredentialFingerprint`] from the database.
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        credential_fingerprint: &CredentialFingerprint,
    ) -> sqlx::Result<Option<Self>> {
        query_scalar!(
            r#"SELECT
                client_credential AS "client_credential: _"
            FROM client_credentials
            WHERE fingerprint = ?"#,
            credential_fingerprint
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(StorableClientCredential::new))
    }

    pub(crate) async fn load_by_client_id(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        query_scalar!(
            r#"SELECT
                client_credential AS "client_credential: _"
            FROM client_credentials WHERE client_id = ?"#,
            client_id
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(StorableClientCredential::new))
    }

    /// Stores the client credential in the database if it does not already exist.
    pub(crate) async fn store(&self, connection: &mut sqlx::SqliteConnection) -> sqlx::Result<()> {
        let fingerprint = self.fingerprint();
        let identity = self.client_credential.identity();
        let client_credential = self.client_credential.persist();
        query!(
            "INSERT OR IGNORE INTO client_credentials (fingerprint, client_id, client_credential) VALUES (?, ?, ?)",
            fingerprint,
            identity,
            client_credential ,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

impl Storable for GroupMembership {
    // TODO(#349): Reinstate the foreign key constraint as soon as we have migrated
    // the mls group table to the new style of storage.
    // FOREIGN KEY (group_id) REFERENCES mlsgroup(primary_key),
    const CREATE_TABLE_STATEMENT: &str = "CREATE TABLE IF NOT EXISTS group_membership (
                client_credential_fingerprint BLOB NOT NULL,
                group_id BLOB NOT NULL,
                client_uuid BLOB NOT NULL,
                user_name TEXT NOT NULL,
                leaf_index INTEGER NOT NULL,
                identity_link_key BLOB NOT NULL,
                status TEXT DEFAULT 'staged_update' NOT NULL CHECK (status IN ('staged_update', 'staged_removal', 'staged_add', 'merged')),
                FOREIGN KEY (client_credential_fingerprint) REFERENCES client_credentials(fingerprint),
                PRIMARY KEY (group_id, leaf_index, status)
            );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let client_credential_fingerprint = row.get(0)?;
        let group_id: GroupIdWrapper = row.get(1)?;
        let client_uuid = row.get(2)?;
        let user_name = row.get(3)?;
        let leaf_index: i64 = row.get(4)?;
        let identity_link_key: IdentityLinkKeySecret = row.get(5)?;
        let client_id = AsClientId::new(user_name, client_uuid);
        Ok(Self {
            client_id,
            group_id: group_id.into(),
            leaf_index: LeafNodeIndex::new(leaf_index as u32),
            identity_link_key: IdentityLinkKey::from(identity_link_key),
            client_credential_fingerprint,
        })
    }
}

struct SqlGroupMembership {
    client_credential_fingerprint: CredentialFingerprint,
    group_id: GroupIdWrapper,
    client_uuid: Uuid,
    user_name: QualifiedUserName,
    leaf_index: u32,
    identity_link_key: IdentityLinkKeySecret,
}

impl From<SqlGroupMembership> for GroupMembership {
    fn from(
        SqlGroupMembership {
            client_credential_fingerprint,
            group_id: GroupIdWrapper(group_id),
            client_uuid,
            user_name,
            leaf_index,
            identity_link_key,
        }: SqlGroupMembership,
    ) -> Self {
        Self {
            client_id: AsClientId::new(user_name, client_uuid),
            group_id,
            leaf_index: LeafNodeIndex::new(leaf_index),
            identity_link_key: IdentityLinkKey::from(identity_link_key),
            client_credential_fingerprint,
        }
    }
}

pub(crate) const GROUP_MEMBERSHIP_TRIGGER: &str =
    "CREATE TRIGGER IF NOT EXISTS delete_orphaned_data
        AFTER DELETE ON group_membership
        FOR EACH ROW
        BEGIN
            -- Delete client credentials if they are not our own and not used in any group.
            DELETE FROM client_credentials
            WHERE fingerprint = OLD.client_credential_fingerprint AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_credential_fingerprint = OLD.client_credential_fingerprint
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_client_uuid = OLD.client_uuid
            );

            -- Delete user profiles of users that are not in any group and that are not our own.
            DELETE FROM users
            WHERE user_name = OLD.user_name AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE user_name = OLD.user_name
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_user_name = OLD.user_name
            );
        END;";

impl GroupMembership {
    /// Merge all staged group memberships for the given group id.
    pub(in crate::groups) async fn merge_for_group(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
    ) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(group_id);

        // Delete all 'staged_removal' rows.
        query!(
            "DELETE FROM group_membership WHERE group_id = ? AND status = 'staged_removal'",
            group_id,
        )
        .execute(&mut *connection)
        .await?;

        // Move modified information from 'staged_update' rows to their
        // 'merged' counterparts (i.e. rows with the same group_id and
        // client_id).
        query!(
            "UPDATE group_membership AS merged
            SET client_credential_fingerprint = staged.client_credential_fingerprint,
                leaf_index = staged.leaf_index,
                identity_link_key = staged.identity_link_key
            FROM group_membership AS staged
            WHERE merged.group_id = staged.group_id
              AND merged.client_uuid = staged.client_uuid
              AND merged.user_name = staged.user_name
              AND merged.status = 'merged'
              AND staged.status = 'staged_update'"
        )
        .execute(&mut *connection)
        .await?;

        // Delete all (previously merged) 'staged_update' rows.
        query!(
            "DELETE FROM group_membership WHERE group_id = ? AND status = 'staged_update'",
            group_id,
        )
        .execute(&mut *connection)
        .await?;

        // Mark all 'staged_add' rows as 'merged'.
        query!(
            "UPDATE group_membership SET status = 'merged'
            WHERE group_id = ? AND status = 'staged_add'",
            group_id,
        )
        .execute(connection)
        .await?;

        Ok(())
    }

    pub(crate) async fn store(&self, connection: &mut sqlx::SqliteConnection) -> sqlx::Result<()> {
        let client_id = self.client_id.client_id();
        let user_name = self.client_id.user_name();
        let sql_group_id = self.sql_group_id();
        let leaf_index = self.leaf_index.u32();
        let identity_link_key = self.identity_link_key.as_ref();
        query!(
            "INSERT OR IGNORE INTO group_membership (
                client_uuid,
                user_name,
                group_id,
                leaf_index,
                identity_link_key,
                client_credential_fingerprint,
                status
            ) VALUES (?, ?, ?, ?, ?, ?, 'merged')",
            client_id,
            user_name,
            sql_group_id,
            leaf_index,
            identity_link_key,
            self.client_credential_fingerprint,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(super) async fn stage_update(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> sqlx::Result<()> {
        let client_id = self.client_id.client_id();
        let user_name = self.client_id.user_name();
        let sql_group_id = self.sql_group_id();
        let leaf_index = self.leaf_index.u32();
        let identity_link_key = self.identity_link_key.as_ref();
        query!(
            "INSERT INTO group_membership (
                client_uuid,
                user_name,
                group_id,
                leaf_index,
                identity_link_key,
                client_credential_fingerprint,
                status)
            VALUES (?, ?, ?, ?, ?, ?, 'staged_update')",
            client_id,
            user_name,
            sql_group_id,
            leaf_index,
            identity_link_key,
            self.client_credential_fingerprint,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(super) async fn stage_add(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> sqlx::Result<()> {
        let client_id = self.client_id.client_id();
        let user_name = self.client_id.user_name();
        let sql_group_id = self.sql_group_id();
        let leaf_index = self.leaf_index.u32();
        let identity_link_key = self.identity_link_key.as_ref();
        query!(
            "INSERT INTO group_membership (client_uuid,
                user_name,
                group_id,
                leaf_index,
                identity_link_key,
                client_credential_fingerprint,
                status)
            VALUES (?, ?, ?, ?, ?, ?, 'staged_add')",
            client_id,
            user_name,
            sql_group_id,
            leaf_index,
            identity_link_key,
            self.client_credential_fingerprint,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(in crate::groups) async fn stage_removal(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        removed_index: LeafNodeIndex,
    ) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(group_id);
        let removed_index = removed_index.u32();
        query!(
            "UPDATE group_membership SET status = 'staged_removal'
            WHERE group_id = ? AND leaf_index = ?",
            group_id,
            removed_index,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads merged group memberships. Use `load_staged` to load
    /// an staged group membership.
    pub(in crate::groups) async fn load(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> sqlx::Result<Option<Self>> {
        Self::load_internal(connection, group_id, leaf_index, true).await
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads staged group memberships. Use `load` to load
    /// a merged group membership.
    pub(super) async fn load_staged(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> sqlx::Result<Option<Self>> {
        Self::load_internal(connection, group_id, leaf_index, false).await
    }

    async fn load_internal(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
        merged: bool,
    ) -> sqlx::Result<Option<Self>> {
        let group_id = GroupIdRefWrapper::from(group_id);
        let leaf_index = leaf_index.u32();
        let sql_group_membership = if merged {
            query_as!(
                SqlGroupMembership,
                r#"SELECT
                    client_credential_fingerprint AS "client_credential_fingerprint: _",
                    group_id AS "group_id: _",
                    client_uuid AS "client_uuid: _",
                    user_name AS "user_name: _",
                    leaf_index AS "leaf_index: _",
                    identity_link_key AS "identity_link_key: _"
                FROM group_membership
                WHERE group_id = ? AND leaf_index = ?
                AND status = 'merged'"#,
                group_id,
                leaf_index,
            )
            .fetch_optional(connection)
            .await
        } else {
            query_as!(
                SqlGroupMembership,
                r#"SELECT
                    client_credential_fingerprint AS "client_credential_fingerprint: _",
                    group_id AS "group_id: _",
                    client_uuid AS "client_uuid: _",
                    user_name AS "user_name: _",
                    leaf_index AS "leaf_index: _",
                    identity_link_key AS "identity_link_key: _"
                FROM group_membership
                WHERE group_id = ? AND leaf_index = ?
                AND status LIKE 'staged_%'"#,
                group_id,
                leaf_index,
            )
            .fetch_optional(connection)
            .await
        };

        sql_group_membership.map(|res| res.map(From::from))
    }

    /// Returns a vector of all leaf indices occupied by (merged) group members
    /// that were not staged for removal.
    pub(super) async fn member_indices(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Vec<LeafNodeIndex>> {
        let group_id = GroupIdRefWrapper::from(group_id);
        query_scalar!(
            r#"SELECT
                leaf_index AS "leaf_index: _"
            FROM group_membership WHERE group_id = ?
            AND NOT (status = 'staged_removal'
                OR status = 'staged_add'
                OR status = 'staged_update')"#,
            group_id
        )
        .fetch(executor)
        .map(|res| res.map(LeafNodeIndex::new))
        .collect()
        .await
    }

    pub(in crate::groups) async fn client_indices(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        client_ids: &[AsClientId],
    ) -> sqlx::Result<Vec<LeafNodeIndex>> {
        let placeholders = client_ids
            .iter()
            .map(|_| "(?, ?)")
            .collect::<Vec<_>>()
            .join(",");
        let query_string = format!(
            "SELECT leaf_index FROM group_membership
            WHERE group_id = ? AND (client_uuid, user_name) IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_string).bind(GroupIdRefWrapper::from(group_id));
        for client_id in client_ids {
            query = query
                .bind(client_id.client_id())
                .bind(client_id.user_name());
        }

        query
            .fetch(connection)
            .map(|row| {
                let leaf_index: u32 = row?.try_get(0)?;
                Ok(LeafNodeIndex::new(leaf_index))
            })
            .collect()
            .await
    }

    pub(in crate::groups) async fn user_client_ids(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Vec<AsClientId>> {
        let group_id = GroupIdRefWrapper::from(group_id);
        query_scalar!(
            r#"SELECT client_uuid AS "client_uuid: _"
            FROM group_membership WHERE group_id = ? AND user_name = ?"#,
            group_id,
            user_name
        )
        .fetch(executor)
        .map(|res| res.map(|client_uuid| AsClientId::new(user_name.clone(), client_uuid)))
        .collect()
        .await
    }

    pub(in crate::groups) async fn group_members(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Vec<AsClientId>> {
        struct SqlGroupMember {
            client_uuid: Uuid,
            user_name: QualifiedUserName,
        }

        let group_id = GroupIdRefWrapper::from(group_id);
        query_as!(
            SqlGroupMember,
            r#"SELECT
                client_uuid AS "client_uuid: _",
                user_name AS "user_name: _"
            FROM group_membership WHERE group_id = ?"#,
            group_id
        )
        .fetch(executor)
        .map(|res| {
            let SqlGroupMember {
                client_uuid,
                user_name,
            } = res?;
            Ok(AsClientId::new(user_name, client_uuid))
        })
        .collect()
        .await
    }

    fn sql_group_id(&self) -> GroupIdRefWrapper<'_> {
        GroupIdRefWrapper::from(&self.group_id)
    }
}

#[cfg(test)]
mod tests {
    use openmls::prelude::SignatureScheme;
    use phnxtypes::{
        credentials::{ClientCredential, ClientCredentialCsr, ClientCredentialPayload},
        crypto::{
            secrets::Secret,
            signatures::signable::{Signature, SignedStruct},
        },
    };
    use rand::Rng;
    use tls_codec::Serialize;
    use uuid::Uuid;

    use super::*;

    fn test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                &[
                    StorableClientCredential::CREATE_TABLE_STATEMENT,
                    GroupMembership::CREATE_TABLE_STATEMENT,
                ]
                .join("\n"),
            )
            .unwrap();
        connection
    }

    /// Returns test credential with a fixed identity but random payload.
    fn test_client_credential(client_id: Uuid) -> StorableClientCredential {
        let client_id =
            AsClientId::new(format!("{client_id}@localhost").parse().unwrap(), client_id);
        let (client_credential_csr, _) =
            ClientCredentialCsr::new(client_id, SignatureScheme::ED25519).unwrap();
        let fingerprint = CredentialFingerprint::new_for_test(b"fingerprint".to_vec());
        let client_credential = ClientCredential::from_payload(
            ClientCredentialPayload::new(client_credential_csr, None, fingerprint),
            Signature::new_for_test(b"signature".to_vec()),
        );
        StorableClientCredential { client_credential }
    }

    /// Returns test group membership with a given parameters, fixed group id and random data.
    fn test_group_membership(
        credential: &ClientCredential,
        index: LeafNodeIndex,
    ) -> GroupMembership {
        let group_id = GroupId::from_slice(&[0; 32]);
        let secret: [u8; 32] = rand::thread_rng().gen();

        GroupMembership::new(
            credential.identity(),
            group_id,
            index,
            IdentityLinkKey::from(Secret::from(secret)),
            credential.fingerprint(),
        )
    }

    #[sqlx::test]
    fn client_credential_store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let credential = test_client_credential(Uuid::new_v4());

        credential.store(&pool)?;
        let loaded = StorableClientCredential::load_by_client_id(&pool, &credential.identity())?
            .expect("missing credential");
        assert_eq!(
            loaded.client_credential.tls_serialize_detached(),
            credential.client_credential.tls_serialize_detached()
        );

        Ok(())
    }

    #[test]
    fn client_credential_store_load_by_id() -> anyhow::Result<()> {
        let connection = test_connection();
        let credential = test_client_credential(Uuid::new_v4());

        credential.store(&connection)?;
        let loaded =
            StorableClientCredential::load_by_client_id(&connection, &credential.identity())?
                .expect("missing credential");
        assert_eq!(
            loaded.client_credential.tls_serialize_detached(),
            credential.client_credential.tls_serialize_detached()
        );

        Ok(())
    }

    #[test]
    fn group_membership_merge_for_group() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential_add = test_client_credential(Uuid::new_v4());
        credential_add.store(&connection)?;
        let index_add = LeafNodeIndex::new(0);
        let membership_add = test_group_membership(&credential_add, index_add);
        membership_add.stage_add(&connection)?;

        let credential_update = test_client_credential(Uuid::new_v4());
        credential_update.store(&connection)?;
        let index_update = LeafNodeIndex::new(1);
        let membership_update = test_group_membership(&credential_update, index_update);
        membership_update.store(&connection)?;
        membership_update.stage_update(&connection)?;

        let credential_remove = test_client_credential(Uuid::new_v4());
        credential_remove.store(&connection)?;
        let index_remove = LeafNodeIndex::new(2);
        let membership_remove = test_group_membership(&credential_remove, index_remove);
        membership_remove.store(&connection)?;
        GroupMembership::stage_removal(&connection, &membership_remove.group_id, index_remove)?;

        let credential = test_client_credential(Uuid::new_v4());
        credential.store(&connection)?;
        let index = LeafNodeIndex::new(3);
        let membership = test_group_membership(&credential, index);
        membership.store(&connection)?;

        GroupMembership::merge_for_group(&connection, &membership.group_id)?;

        let group_id = &membership.group_id;

        let loaded =
            GroupMembership::load(&connection, group_id, index_add)?.expect("missing membership");
        assert_eq!(loaded, membership_add);
        let loaded = GroupMembership::load(&connection, group_id, index_update)?
            .expect("missing membership");
        assert_eq!(loaded, membership_update);
        let loaded = GroupMembership::load(&connection, group_id, index_remove)?;
        assert_eq!(loaded, None);
        let loaded =
            GroupMembership::load(&connection, group_id, index)?.expect("missing membership");
        assert_eq!(loaded, membership);

        Ok(())
    }

    #[test]
    fn store_idempotent() -> anyhow::Result<()> {
        let connection = test_connection();
        let id = Uuid::new_v4();
        let credential_1 = test_client_credential(id);
        let credential_2 = test_client_credential(id);

        // precondition
        assert_eq!(credential_1.identity(), credential_2.identity());
        assert_ne!(
            credential_1.tls_serialize_detached(),
            credential_2.tls_serialize_detached()
        );

        credential_1.store(&connection)?;
        credential_2.store(&connection)?;

        let loaded =
            StorableClientCredential::load_by_client_id(&connection, &credential_1.identity())?
                .expect("missing credential");
        assert_eq!(
            loaded.client_credential.tls_serialize_detached(),
            credential_1.client_credential.tls_serialize_detached()
        );

        Ok(())
    }

    #[test]
    fn group_membership_store_load() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential = test_client_credential(Uuid::new_v4());
        credential.store(&connection)?;

        let index = LeafNodeIndex::new(0);
        let membership = test_group_membership(&credential, index);

        membership.store(&connection)?;
        let loaded = GroupMembership::load(&connection, &membership.group_id, index)?
            .expect("missing membership");
        assert_eq!(loaded, membership);
        let loaded = GroupMembership::load_staged(&connection, &membership.group_id, index)?;
        assert_eq!(loaded, None);

        Ok(())
    }

    #[test]
    fn group_membership_store_load_staged() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential = test_client_credential(Uuid::new_v4());
        credential.store(&connection)?;

        let index = LeafNodeIndex::new(0);
        let membership = test_group_membership(&credential, index);

        membership.stage_add(&connection)?;
        let loaded = GroupMembership::load(&connection, &membership.group_id, index)?;
        assert_eq!(loaded, None);
        let loaded = GroupMembership::load_staged(&connection, &membership.group_id, index)?
            .expect("missing membership");
        assert_eq!(loaded, membership);

        Ok(())
    }

    #[test]
    fn group_membership_member_indices() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential = test_client_credential(Uuid::new_v4());
        credential.store(&connection)?;

        let index = LeafNodeIndex::new(0);
        let membership = test_group_membership(&credential, index);

        membership.store(&connection)?;
        let indices = GroupMembership::member_indices(&connection, &membership.group_id)?;
        assert_eq!(indices, [index]);

        Ok(())
    }

    #[test]
    fn group_membership_client_indices() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential_a = test_client_credential(Uuid::new_v4());
        credential_a.store(&connection)?;

        let credential_b = test_client_credential(Uuid::new_v4());
        credential_b.store(&connection)?;

        let index_a = LeafNodeIndex::new(0);
        let membership_a = test_group_membership(&credential_a, index_a);

        let index_b = LeafNodeIndex::new(1);
        let membership_b = test_group_membership(&credential_b, index_b);

        membership_a.store(&connection)?;
        membership_b.store(&connection)?;

        let indices = GroupMembership::client_indices(
            &connection,
            &membership_a.group_id,
            &[credential_a.identity()],
        )?;
        assert_eq!(indices, vec![index_a]);

        let indices = GroupMembership::client_indices(
            &connection,
            &membership_b.group_id,
            &[credential_b.identity()],
        )?;
        assert_eq!(indices, [index_b]);

        Ok(())
    }

    #[test]
    fn group_membership_user_client_ids() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential = test_client_credential(Uuid::new_v4());
        credential.store(&connection)?;

        let index = LeafNodeIndex::new(0);
        let membership = test_group_membership(&credential, index);

        membership.store(&connection)?;
        let client_ids = GroupMembership::user_client_ids(
            &connection,
            &membership.group_id,
            &membership.client_id.user_name(),
        )?;
        assert_eq!(client_ids, [credential.identity()]);

        Ok(())
    }

    #[test]
    fn group_membership_group_members() -> anyhow::Result<()> {
        let connection = test_connection();

        let credential_a = test_client_credential(Uuid::new_v4());
        credential_a.store(&connection)?;

        let credential_b = test_client_credential(Uuid::new_v4());
        credential_b.store(&connection)?;

        let index_a = LeafNodeIndex::new(0);
        let membership_a = test_group_membership(&credential_a, index_a);

        let index_b = LeafNodeIndex::new(1);
        let membership_b = test_group_membership(&credential_b, index_b);

        membership_a.store(&connection)?;
        membership_b.store(&connection)?;

        let members = GroupMembership::group_members(&connection, &membership_a.group_id)?;
        assert_eq!(members, [credential_a.identity(), credential_b.identity()]);

        Ok(())
    }
}
