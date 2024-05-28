// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt::Display, ops::Deref};

use anyhow::{anyhow, Result};
use openmls::{credentials::Credential, group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    credentials::{
        infra_credentials::{InfraCredential, InfraCredentialPlaintext, InfraCredentialTbs},
        ClientCredential, CredentialFingerprint, EncryptedClientCredential,
    },
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, EncryptedSignatureEarKey, SignatureEarKey,
                SignatureEarKeySecret, SignatureEarKeyWrapperKey,
            },
            EarDecryptable,
        },
        signatures::signable::Verifiable,
    },
    identifiers::{AsClientId, UserName},
};
use rusqlite::{params, params_from_iter, types::FromSql, Connection, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};

use crate::{
    key_stores::as_credentials::AsCredentialStore,
    utils::persistence::{Storable, Triggerable},
};

use super::decrypt_and_verify_client_credential;

/// Helper struct that allows us to use GroupId as sqlite input.
struct GroupIdRefWrapper<'a>(&'a GroupId);

impl<'a> From<&'a GroupId> for GroupIdRefWrapper<'a> {
    fn from(group_id: &'a GroupId) -> Self {
        Self(group_id)
    }
}

impl<'a> ToSql for GroupIdRefWrapper<'a> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.as_slice().to_sql()
    }
}

impl Display for GroupIdRefWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.0.as_slice()))
    }
}

struct GroupIdWrapper(GroupId);

impl From<GroupIdWrapper> for GroupId {
    fn from(group_id: GroupIdWrapper) -> Self {
        group_id.0
    }
}

impl FromSql for GroupIdWrapper {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let group_id = GroupId::from_slice(value.as_blob()?);
        Ok(GroupIdWrapper(group_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StorableClientCredential {
    client_credential: ClientCredential,
}

impl From<ClientCredential> for StorableClientCredential {
    fn from(client_credential: ClientCredential) -> Self {
        Self { client_credential }
    }
}

impl From<StorableClientCredential> for ClientCredential {
    fn from(storable_client_credential: StorableClientCredential) -> Self {
        storable_client_credential.client_credential
    }
}

impl Deref for StorableClientCredential {
    type Target = ClientCredential;

    fn deref(&self) -> &Self::Target {
        &self.client_credential
    }
}

impl StorableClientCredential {
    pub(crate) fn new(client_credential: ClientCredential) -> Self {
        Self { client_credential }
    }

    //pub(super) async fn decrypt_and_verify_all(
    //    ear_key: &ClientCredentialEarKey,
    //    as_credential_store: &AsCredentialStore<'_>,
    //    encrypted_client_credentials: impl IntoIterator<Item = EncryptedClientCredential>,
    //) -> Result<Vec<Self>> {
    //    let mut client_credentials = Vec::new();
    //    for ctxt in encrypted_client_credentials.into_iter() {
    //        let client_credential =
    //            Self::decrypt_and_verify(ear_key, as_credential_store, ctxt).await?;
    //        client_credentials.push(client_credential);
    //    }
    //    Ok(client_credentials)
    //}

    pub(super) async fn decrypt_and_verify(
        ear_key: &ClientCredentialEarKey,
        as_credential_store: &AsCredentialStore<'_>,
        ecc: EncryptedClientCredential,
    ) -> Result<Self> {
        let client_credential =
            decrypt_and_verify_client_credential(as_credential_store, ear_key, &ecc).await?;
        Ok(Self { client_credential })
    }
}

impl StorableClientCredential {
    /// Load the [`ClientAuthInfo`] for the given client_id from the database.
    pub(super) fn load(
        connection: &Connection,
        credential_fingerprint: &CredentialFingerprint,
    ) -> Result<Option<Self>> {
        let mut stmt = connection
            .prepare("SELECT client_credential FROM client_credentials WHERE fingerprint = ?")?;
        let client_credential_option = stmt
            .query_row(params![credential_fingerprint], |row| {
                let client_credential: ClientCredential = row.get(0)?;
                Ok(Self::new(client_credential))
            })
            .optional()?;

        Ok(client_credential_option)
    }

    /// Stores the client credential in the database if it does not already exist.
    pub(crate) fn store(&self, connection: &Connection) -> Result<()> {
        let client_credential_bytes = serde_json::to_vec(&self.client_credential)?;
        let fingerprint = self.fingerprint();
        connection.execute(
            "INSERT OR IGNORE INTO client_credentials (fingerprint, client_id, client_credential) VALUES (?, ?, ?)",
            params![
                fingerprint,
                self.client_credential.identity(),
                client_credential_bytes
            ],
        )?;
        Ok(())
    }
}

impl Storable for StorableClientCredential {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS client_credentials (
                fingerprint BLOB PRIMARY KEY,
                client_id TEXT NOT NULL,
                client_credential BLOB NOT NULL
            )";
}

pub(crate) struct GroupMembership {
    client_id: AsClientId,
    client_credential_fingerprint: CredentialFingerprint,
    group_id: GroupId,
    signature_ear_key: SignatureEarKey,
    leaf_index: LeafNodeIndex,
}

impl GroupMembership {
    pub(super) fn new(
        client_id: AsClientId,
        group_id: GroupId,
        leaf_index: LeafNodeIndex,
        signature_ear_key: SignatureEarKey,
        client_credential_fingerprint: CredentialFingerprint,
    ) -> Self {
        Self {
            client_id,
            client_credential_fingerprint,
            group_id,
            leaf_index,
            signature_ear_key,
        }
    }

    /// Merge all staged group memberships for the given group id. Returns an
    /// error if there are no staged changes.
    pub(super) fn merge_for_group(connection: &Connection, group_id: &GroupId) -> Result<()> {
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
          AND merged.client_id = staged.client_id
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

    pub(super) fn store(&self, connection: &Connection) -> Result<()> {
        connection.execute(
            "INSERT OR IGNORE INTO group_membership (client_id, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, 'merged')",
            params![
                self.client_id,
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    pub(super) fn stage_update(&self, connection: &Connection) -> Result<()> {
        connection.execute(
            "INSERT INTO group_membership (client_id, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, 'staged_update')",
            params![
                self.client_id,
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    pub(super) fn stage_add(&self, connection: &Connection) -> Result<()> {
        connection.execute(
            "INSERT INTO group_membership (client_id, group_id, leaf_index, signature_ear_key, client_credential_fingerprint, status) VALUES (?, ?, ?, ?, ?, 'staged_add')",
            params![
                self.client_id,
                self.sql_group_id(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref(),
                self.client_credential_fingerprint,
            ],
        )?;
        Ok(())
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads merged group memberships. Use `load_staged` to load
    /// an staged group membership.
    pub(super) fn load(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        Self::load_internal(connection, group_id, leaf_index, true)
    }

    /// Load the [`GroupMembership`] for the given group_id and leaf_index from the
    /// database. Only loads staged group memberships. Use `load` to load
    /// a merged group membership.
    fn load_staged(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        Self::load_internal(connection, group_id, leaf_index, false)
    }

    fn load_internal(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
        merged: bool,
    ) -> Result<Option<Self>> {
        let mut stmt = if merged {
            connection.prepare("SELECT group_id, client_id, leaf_index, signature_ear_key, client_credential_fingerprint FROM group_membership WHERE group_id = ? AND leaf_index = ? AND status = 'merged'")?
        } else {
            connection.prepare("SELECT group_id, client_id, leaf_index, signature_ear_key, client_credential_fingerprint FROM group_membership WHERE group_id = ? AND leaf_index = ? AND status LIKE 'staged_%'")?
        };
        let group_membership = stmt
            .query_row(
                params![GroupIdRefWrapper::from(group_id), leaf_index.u32()],
                |row| {
                    let group_id: GroupIdWrapper = row.get(0)?;
                    let client_id: AsClientId = row.get(1)?;
                    let leaf_index: i64 = row.get(2)?;
                    let signature_ear_key: SignatureEarKeySecret = row.get(3)?;
                    let client_credential_fingerprint = row.get(4)?;
                    Ok(Self {
                        client_id: client_id,
                        group_id: group_id.into(),
                        leaf_index: LeafNodeIndex::new(leaf_index as u32),
                        signature_ear_key: SignatureEarKey::from(signature_ear_key),
                        client_credential_fingerprint,
                    })
                },
            )
            .optional()?;
        Ok(group_membership)
    }

    // Computes free indices based on existing leaf indices and staged removals.
    // Not that staged additions are not considered.
    pub(super) fn free_indices(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<impl Iterator<Item = LeafNodeIndex>> {
        let mut stmt = connection.prepare(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND NOT (status = 'staged_removal' OR status = 'staged_add' OR status = 'staged_update')",
        )?;
        let leaf_indices = stmt
            .query_map(params![group_id.as_slice()], |row| {
                let leaf_index: i64 = row.get(0)?;
                Ok(leaf_index as u32)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let highest_index = leaf_indices.last().cloned().unwrap_or(0);
        let free_indices = (0..highest_index)
            .filter(move |index| !leaf_indices.contains(&index))
            .chain(highest_index + 1..)
            .map(|e| LeafNodeIndex::new(e));
        Ok(free_indices)
    }

    pub(super) fn client_indices(
        connection: &Connection,
        group_id: &GroupId,
        client_ids: &[AsClientId],
    ) -> Result<Vec<LeafNodeIndex>> {
        let placeholders = client_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query_string = format!(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND client_id IN ({})",
            placeholders
        );
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut params: Vec<&dyn ToSql> = vec![&group_id];
        params.extend(client_ids.iter().map(|client_id| client_id as &dyn ToSql));
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

    pub(super) fn user_client_ids(
        connection: &Connection,
        group_id: &GroupId,
        user_name: &UserName,
    ) -> Result<Vec<AsClientId>, rusqlite::Error> {
        let query_string = format!(
            "SELECT client_id FROM group_membership WHERE group_id = ? AND client_id LIKE '%.{}'",
            user_name
        );
        let mut stmt = connection.prepare(&query_string)?;
        let indices = stmt
            .query_map([GroupIdRefWrapper::from(group_id)], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(indices)
    }

    /// Returns the leaf indices of the clients owned by the given user.
    pub(super) fn user_client_indices(
        connection: &Connection,
        group_id: &GroupId,
        user_name: UserName,
    ) -> Result<Vec<LeafNodeIndex>, rusqlite::Error> {
        let query_string = format!(
            "SELECT leaf_index FROM group_membership WHERE group_id = ? AND client_id LIKE %.{}",
            user_name
        );
        let mut stmt = connection.prepare(&query_string)?;
        let indices = stmt
            .query_map([GroupIdRefWrapper::from(group_id)], |row| {
                let leaf_index: i64 = row.get(0)?;
                Ok(LeafNodeIndex::new(leaf_index as u32))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(indices)
    }

    pub(super) fn group_members(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<Vec<AsClientId>> {
        let mut stmt =
            connection.prepare("SELECT client_id FROM group_membership WHERE group_id = ?")?;
        let group_members = stmt
            .query_map(params![GroupIdRefWrapper::from(group_id)], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(group_members)
    }

    pub(super) fn stage_removal(
        connection: &Connection,
        group_id: &GroupId,
        removed_index: LeafNodeIndex,
    ) -> Result<()> {
        connection.execute(
            "UPDATE group_membership SET status = 'staged_removal' WHERE group_id = ? AND leaf_index = ?",
            params![GroupIdRefWrapper::from(group_id), removed_index.u32()],
        )?;
        Ok(())
    }

    pub(crate) fn client_credential_fingerprint(&self) -> &CredentialFingerprint {
        &self.client_credential_fingerprint
    }

    /// Set the signature ear key.
    pub(crate) fn set_signature_ear_key(&mut self, signature_ear_key: SignatureEarKey) {
        self.signature_ear_key = signature_ear_key;
    }

    /// Set the group member's leaf index. This can be required for resync
    /// operations.
    pub(crate) fn set_leaf_index(&mut self, leaf_index: LeafNodeIndex) {
        self.leaf_index = leaf_index;
    }

    fn sql_group_id(&self) -> GroupIdRefWrapper<'_> {
        GroupIdRefWrapper::from(&self.group_id)
    }

    pub(crate) fn client_id(&self) -> &AsClientId {
        &self.client_id
    }
}

impl Storable for GroupMembership {
    // TODO: Reinstate the foreign key constraint as soon as we have migrated
    // the mls group table to the new style of storage.
    // FOREIGN KEY (group_id) REFERENCES mlsgroup(primary_key),
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS group_membership (
                client_credential_fingerprint BLOB NOT NULL,
                group_id BLOB NOT NULL,
                client_id TEXT NOT NULL,
                leaf_index INTEGER NOT NULL,
                signature_ear_key BLOB NOT NULL,
                status TEXT DEFAULT 'staged_update' NOT NULL CHECK (status IN ('staged_update', 'staged_removal', 'staged_add', 'merged')),
                FOREIGN KEY (client_credential_fingerprint) REFERENCES client_credentials(fingerprint),
                PRIMARY KEY (group_id, leaf_index, status)
            )";
}

impl Triggerable for GroupMembership {
    const CREATE_TRIGGER_STATEMENT: &'static str = "CREATE TRIGGER IF NOT EXISTS delete_orphaned_client_credentials AFTER DELETE ON group_membership
        BEGIN
            DELETE FROM client_credentials
            WHERE fingerprint = OLD.client_credential_fingerprint AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_credential_fingerprint = OLD.client_credential_fingerprint
            );
        END";
}

pub(super) struct ClientAuthInfo {
    client_credential: StorableClientCredential,
    group_membership: GroupMembership,
}

impl ClientAuthInfo {
    pub(super) fn new(
        client_credential: impl Into<StorableClientCredential>,
        group_membership: GroupMembership,
    ) -> Self {
        Self {
            client_credential: client_credential.into(),
            group_membership,
        }
    }

    /// Decrypt and verify the given encrypted client auth info. The encrypted
    /// client auth info needs to be given s.t. the index of the client in the
    /// group corresponds to the index in the iterator.
    pub(super) async fn decrypt_and_verify_all(
        group_id: &GroupId,
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        encrypted_client_information: impl Iterator<
            Item = (
                LeafNodeIndex,
                (EncryptedClientCredential, EncryptedSignatureEarKey),
            ),
        >,
    ) -> Result<Vec<Self>> {
        let mut client_information = Vec::new();
        for (leaf_index, encrypted_client_info) in encrypted_client_information {
            let client_auth_info = Self::decrypt_and_verify(
                group_id,
                ear_key,
                wrapper_key,
                as_credential_store,
                encrypted_client_info,
                leaf_index,
            )
            .await?;
            client_information.push(client_auth_info);
        }
        Ok(client_information)
    }

    /// Decrypt and verify the given encrypted client auth info.
    pub(super) async fn decrypt_and_verify(
        group_id: &GroupId,
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        (ecc, esek): (EncryptedClientCredential, EncryptedSignatureEarKey),
        leaf_index: LeafNodeIndex,
    ) -> Result<Self> {
        let client_credential =
            StorableClientCredential::decrypt_and_verify(ear_key, as_credential_store, ecc).await?;
        let signature_ear_key = SignatureEarKey::decrypt(wrapper_key, &esek)?;
        let group_membership = GroupMembership::new(
            client_credential.identity(),
            group_id.clone(),
            leaf_index,
            signature_ear_key,
            client_credential.fingerprint(),
        );
        let client_auth_info = ClientAuthInfo {
            client_credential,
            group_membership,
        };
        Ok(client_auth_info)
    }

    pub(super) fn verify_infra_credential(&self, credential: &Credential) -> Result<()> {
        let infra_credential = InfraCredential::try_from(credential.clone())?;

        // Verify the leaf credential
        let credential_plaintext = InfraCredentialPlaintext::decrypt(
            &infra_credential,
            &self.group_membership.signature_ear_key,
        )?;
        credential_plaintext
            .verify::<InfraCredentialTbs>(self.client_credential().verifying_key())?;
        Ok(())
    }

    pub(super) fn stage_update(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.stage_update(connection)?;
        Ok(())
    }

    pub(super) fn stage_add(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.stage_add(connection)?;
        Ok(())
    }

    pub(super) fn store(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.store(connection)?;
        Ok(())
    }

    pub(super) fn load(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) = GroupMembership::load(connection, group_id, leaf_index)?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            connection,
            &group_membership.client_credential_fingerprint,
        )?
        .ok_or(anyhow!(
            "Found a matching Groupmembership, but no matching ClientCredential"
        ))?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) fn load_staged(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) =
            GroupMembership::load_staged(connection, group_id, leaf_index)?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            connection,
            &group_membership.client_credential_fingerprint,
        )?
        .ok_or(anyhow!(
            "Found a matching Groupmembership, but no matching ClientCredential"
        ))?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) fn client_credential(&self) -> &StorableClientCredential {
        &self.client_credential
    }

    pub(super) fn group_membership_mut(&mut self) -> &mut GroupMembership {
        &mut self.group_membership
    }
}
