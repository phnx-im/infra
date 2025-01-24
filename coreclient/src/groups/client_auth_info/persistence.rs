// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::{group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    credentials::CredentialFingerprint,
    crypto::ear::keys::{IdentityLinkKey, IdentityLinkKeySecret},
    identifiers::{AsClientId, QualifiedUserName},
};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, ToSql};

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable};

use super::{GroupMembership, StorableClientCredential};

impl Storable for StorableClientCredential {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS client_credentials (
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
    pub(in crate::groups) fn load(
        connection: &Connection,
        credential_fingerprint: &CredentialFingerprint,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT client_credential FROM client_credentials WHERE fingerprint = ?")?;
        let client_credential_option = stmt
            .query_row(params![credential_fingerprint], Self::from_row)
            .optional()?;

        Ok(client_credential_option)
    }

    pub(crate) fn load_by_client_id(
        connection: &Connection,
        client_id: &AsClientId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT client_credential FROM client_credentials WHERE client_id = ?")?;
        let client_credential_option = stmt
            .query_row(params![client_id], Self::from_row)
            .optional()?;

        Ok(client_credential_option)
    }

    /// Stores the client credential in the database if it does not already exist.
    pub(crate) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        let fingerprint = self.fingerprint();
        connection.execute(
            "INSERT OR IGNORE INTO client_credentials (fingerprint, client_id, client_credential) VALUES (?, ?, ?)",
            params![
                fingerprint,
                self.client_credential.identity(),
                self.client_credential,
            ],
        )?;
        Ok(())
    }
}

impl GroupMembership {
    /// Merge all staged group memberships for the given group id.
    pub(in crate::groups) fn merge_for_group(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(group_id);
        // Delete all 'staged_removal' rows.
        connection.execute(
            "DELETE FROM group_membership WHERE group_id = ? AND status = 'staged_removal'",
            params![group_id],
        )?;

        // Move modified information from 'staged_update' rows to their
        // 'merged' counterparts (i.e. rows with the same group_id and
        // client_id).
        connection.execute(
            "
        UPDATE group_membership AS merged
        SET client_credential_fingerprint = staged.client_credential_fingerprint,
            leaf_index = staged.leaf_index,
            signature_ear_key = staged.signature_ear_key,
            status = 'merged'
        FROM group_membership AS staged
        WHERE merged.group_id = staged.group_id
          AND merged.client_uuid = staged.client_uuid
          AND merged.user_name = staged.user_name
          AND staged.status = 'staged_update'",
            [],
        )?;

        // Delete all (previously merged) 'staged_update' rows.
        connection.execute(
            "DELETE FROM group_membership WHERE group_id = ? AND status = 'staged_update'",
            params![group_id],
        )?;

        // Mark all 'staged_add' rows as 'merged'.
        connection.execute(
            "UPDATE group_membership SET status = 'merged' WHERE group_id = ? AND status = 'staged_add'",
            params![group_id],
        )?;

        Ok(())
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR IGNORE INTO group_membership (client_uuid, user_name, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, ?, 'merged')",
            params![
                self.client_id.client_id(),
                self.client_id.user_name(),
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    pub(super) fn stage_update(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO group_membership (client_uuid, user_name, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, ?, 'staged_update')",
            params![
                self.client_id.client_id(),
                self.client_id.user_name(),
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    pub(super) fn stage_add(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO group_membership (client_uuid, user_name, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, ?, 'staged_add')",
            params![
                self.client_id.client_id(),
                self.client_id.user_name(),
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    pub(in crate::groups) fn stage_removal(
        connection: &Connection,
        group_id: &GroupId,
        removed_index: LeafNodeIndex,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE group_membership SET status = 'staged_removal' WHERE group_id = ? AND leaf_index = ?",
            params![GroupIdRefWrapper::from(group_id), removed_index.u32()],
        )?;
        Ok(())
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads merged group memberships. Use `load_staged` to load
    /// an staged group membership.
    pub(in crate::groups) fn load(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>, rusqlite::Error> {
        Self::load_internal(connection, group_id, leaf_index, true)
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads staged group memberships. Use `load` to load
    /// a merged group membership.
    pub(super) fn load_staged(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>, rusqlite::Error> {
        Self::load_internal(connection, group_id, leaf_index, false)
    }

    fn load_internal(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
        merged: bool,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut query_string = "SELECT client_credential_fingerprint, group_id, client_uuid, user_name, leaf_index, signature_ear_key FROM group_membership WHERE group_id = ? AND leaf_index = ?".to_owned();
        if merged {
            query_string += " AND status = 'merged'";
        } else {
            query_string += " AND status LIKE 'staged_%'";
        };
        let mut stmt = connection.prepare(&query_string)?;
        let group_membership = stmt
            .query_row(
                params![GroupIdRefWrapper::from(group_id), leaf_index.u32()],
                Self::from_row,
            )
            .optional()?;
        Ok(group_membership)
    }

    /// Returns a vector of all leaf indices occupied by (merged) group members
    /// that were not staged for removal.
    pub(super) fn member_indices(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<Vec<LeafNodeIndex>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND NOT (status = 'staged_removal' OR status = 'staged_add' OR status = 'staged_update')",
        )?;
        let indices = stmt
            .query_map(params![group_id.as_slice()], |row| {
                let leaf_index: i64 = row.get(0)?;
                Ok(LeafNodeIndex::new(leaf_index as u32))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(indices)
    }

    pub(in crate::groups) fn client_indices(
        connection: &Connection,
        group_id: &GroupId,
        client_ids: &[AsClientId],
    ) -> Result<Vec<LeafNodeIndex>, rusqlite::Error> {
        let client_infos = client_ids
            .iter()
            .map(|client_id| (client_id.client_id(), client_id.user_name()))
            .collect::<Vec<_>>();
        let placeholders = client_infos
            .iter()
            .map(|_| "(?, ?)")
            .collect::<Vec<_>>()
            .join(",");
        let query_string = format!(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND (client_uuid, user_name) IN ({})",
            placeholders
        );
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut params: Vec<&dyn ToSql> = vec![&group_id];
        params.extend(client_infos.iter().flat_map(|(client_uuid, user_name)| {
            [client_uuid as &dyn ToSql, user_name as &dyn ToSql]
        }));
        let mut stmt = connection.prepare(&query_string)?;
        let rows = stmt
            .query_map(params_from_iter(params), |row| {
                let leaf_index_raw: i64 = row.get(0)?;
                let leaf_index = LeafNodeIndex::new(leaf_index_raw as u32);
                Ok(leaf_index)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub(in crate::groups) fn user_client_ids(
        connection: &Connection,
        group_id: &GroupId,
        user_name: &QualifiedUserName,
    ) -> Result<Vec<AsClientId>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT client_uuid FROM group_membership WHERE group_id = ? AND user_name = ?",
        )?;
        let indices = stmt
            .query_map(
                params![GroupIdRefWrapper::from(group_id), user_name],
                |row| {
                    let client_uuid = row.get(0)?;
                    let client_id = AsClientId::new(user_name.clone(), client_uuid);
                    Ok(client_id)
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(indices)
    }

    /// Returns the leaf indices of the clients owned by the given user.
    pub(in crate::groups) fn user_client_indices(
        connection: &Connection,
        group_id: &GroupId,
        user_name: QualifiedUserName,
    ) -> Result<Vec<LeafNodeIndex>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND user_name = ?",
        )?;
        let indices = stmt
            .query_map(
                params![GroupIdRefWrapper::from(group_id), user_name],
                |row| {
                    let leaf_index: i64 = row.get(0)?;
                    Ok(LeafNodeIndex::new(leaf_index as u32))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(indices)
    }

    pub(in crate::groups) fn group_members(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<Vec<AsClientId>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT client_uuid, user_name FROM group_membership WHERE group_id = ?")?;
        let group_members = stmt
            .query_map(params![GroupIdRefWrapper::from(group_id)], |row| {
                let client_uuid = row.get(0)?;
                let user_name = row.get(1)?;
                let client_id = AsClientId::new(user_name, client_uuid);
                Ok(client_id)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(group_members)
    }

    fn sql_group_id(&self) -> GroupIdRefWrapper<'_> {
        GroupIdRefWrapper::from(&self.group_id)
    }
}

impl Storable for GroupMembership {
    // TODO: Reinstate the foreign key constraint as soon as we have migrated
    // the mls group table to the new style of storage.
    // FOREIGN KEY (group_id) REFERENCES mlsgroup(primary_key),
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS group_membership (
                client_credential_fingerprint BLOB NOT NULL,
                group_id BLOB NOT NULL,
                client_uuid BLOB NOT NULL,
                user_name TEXT NOT NULL,
                leaf_index INTEGER NOT NULL,
                signature_ear_key BLOB NOT NULL,
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
        let signature_ear_key: IdentityLinkKeySecret = row.get(5)?;
        let client_id = AsClientId::new(user_name, client_uuid);
        Ok(Self {
            client_id,
            group_id: group_id.into(),
            leaf_index: LeafNodeIndex::new(leaf_index as u32),
            signature_ear_key: IdentityLinkKey::from(signature_ear_key),
            client_credential_fingerprint,
        })
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
