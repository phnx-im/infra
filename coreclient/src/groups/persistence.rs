// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::{GroupId, MlsGroup};
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    credentials::keys::InfraCredentialSigningKey,
    crypto::{
        ear::keys::{ClientCredentialEarKey, GroupStateEarKey, SignatureEarKeyWrapperKey},
        signatures::keys::UserAuthSigningKey,
    },
};
use rusqlite::{params, OptionalExtension};

use crate::{
    groups::openmls_provider::PhnxOpenMlsProvider,
    utils::persistence::{GroupIdRefWrapper, Storable},
};

use super::{diff::StagedGroupDiff, Group};

// A helper struct to store a group in the database. The `MlsGroup` part of the
// group is loaded separately via the OpenMLS storage provider.
pub(crate) struct StorableGroup {
    leaf_signer: InfraCredentialSigningKey,
    signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    credential_ear_key: ClientCredentialEarKey,
    group_state_ear_key: GroupStateEarKey,
    user_auth_signing_key_option: Option<UserAuthSigningKey>,
    pending_diff: Option<StagedGroupDiff>,
}

impl Storable for StorableGroup {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS groups (
            group_id BLOB PRIMARY KEY,
            leaf_signer BLOB NOT NULL,
            signature_ear_key_wrapper_key BLOB NOT NULL,
            credential_ear_key BLOB NOT NULL,
            group_state_ear_key BLOB NOT NULL,
            user_auth_signing_key_option BLOB,
            pending_diff BLOB
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let leaf_signer = row.get(1)?;
        let signature_ear_key_wrapper_key = row.get(2)?;
        let credential_ear_key = row.get(3)?;
        let group_state_ear_key = row.get(4)?;
        let user_auth_signing_key_option = row.get(5)?;
        let pending_diff = row.get(6)?;

        Ok(StorableGroup {
            leaf_signer,
            signature_ear_key_wrapper_key,
            credential_ear_key,
            group_state_ear_key,
            user_auth_signing_key_option,
            pending_diff,
        })
    }
}

impl Group {
    /// This does not store the `MlsGroup` part of the group. This is done by
    /// the OpenMLS storage provider as part of the group creation process. As a
    /// consequence, group creation and the call to this `store` function should
    /// happen atomically (as part of a transaction) to ensure that the data
    /// stays consistent.
    pub(crate) fn store(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        connection.execute(
            "INSERT INTO groups (group_id, leaf_signer, signature_ear_key_wrapper_key, credential_ear_key, group_state_ear_key, user_auth_signing_key_option, pending_diff) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                group_id,
                self.leaf_signer,
                self.signature_ear_key_wrapper_key,
                self.credential_ear_key,
                self.group_state_ear_key,
                self.user_auth_signing_key_option,
                self.pending_diff,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn load(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let group_id_wrapper = GroupIdRefWrapper::from(group_id);
        let mut stmt = connection.prepare("SELECT * FROM groups WHERE group_id = ?")?;
        let provider = PhnxOpenMlsProvider::new(connection);
        let mls_group_option = MlsGroup::load(provider.storage(), group_id)?;

        let storable_group_option = stmt
            .query_row(params![group_id_wrapper], StorableGroup::from_row)
            .optional()?;
        let (Some(storable_group), Some(mls_group)) = (storable_group_option, mls_group_option)
        else {
            return Ok(None);
        };
        let group = Group {
            group_id: group_id.clone(),
            leaf_signer: storable_group.leaf_signer,
            signature_ear_key_wrapper_key: storable_group.signature_ear_key_wrapper_key,
            credential_ear_key: storable_group.credential_ear_key,
            group_state_ear_key: storable_group.group_state_ear_key,
            user_auth_signing_key_option: storable_group.user_auth_signing_key_option,
            mls_group,
            pending_diff: storable_group.pending_diff,
        };
        Ok(Some(group))
    }

    /// This does not store the `MlsGroup` part of the group. This is done by
    /// the OpenMLS storage provider as part of a group operation. As a
    /// consequence, the group operation and a call to this `store` function
    /// should happen atomically (as part of a transaction) to ensure that the
    /// data stays consistent.
    pub(crate) fn store_update(
        &self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        connection.execute(
            "UPDATE groups SET leaf_signer = ?, signature_ear_key_wrapper_key = ?, credential_ear_key = ?, group_state_ear_key = ?, user_auth_signing_key_option = ?, pending_diff = ? WHERE group_id = ?",
            params![
                self.leaf_signer,
                self.signature_ear_key_wrapper_key,
                self.credential_ear_key,
                self.group_state_ear_key,
                self.user_auth_signing_key_option,
                self.pending_diff,
                group_id,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn delete_from_db(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(group_id);
        connection.execute("DELETE FROM groups WHERE group_id = ?", params![group_id])?;
        Ok(())
    }
}
