// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::{GroupId, MlsGroup};
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::{
        ear::keys::{ClientCredentialEarKey, GroupStateEarKey, SignatureEarKeyWrapperKey},
        signatures::keys::UserAuthSigningKey,
    },
};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable};

use super::{diff::StagedGroupDiff, openmls_provider::PhnxOpenMlsProvider, Group};

pub(crate) struct StorableGroup {
    group_id: GroupId,
    leaf_signer: PseudonymousCredentialSigningKey,
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
        let group_id: GroupIdWrapper = row.get(0)?;
        let leaf_signer = row.get(1)?;
        let signature_ear_key_wrapper_key = row.get(2)?;
        let credential_ear_key = row.get(3)?;
        let group_state_ear_key = row.get(4)?;
        let user_auth_signing_key_option = row.get(5)?;
        let pending_diff = row.get(6)?;

        Ok(StorableGroup {
            group_id: group_id.into(),
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
        let Some(mls_group) =
            MlsGroup::load(PhnxOpenMlsProvider::new(connection).storage(), group_id)?
        else {
            println!("While loading, MlsGroup::load returned None");
            return Ok(None);
        };
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut stmt = connection.prepare("SELECT * FROM groups WHERE group_id = ?")?;
        stmt.query_row(params![group_id], StorableGroup::from_row)
            .optional()
            .map(|sg| {
                sg.map(|sg| Group {
                    group_id: sg.group_id,
                    leaf_signer: sg.leaf_signer,
                    signature_ear_key_wrapper_key: sg.signature_ear_key_wrapper_key,
                    credential_ear_key: sg.credential_ear_key,
                    group_state_ear_key: sg.group_state_ear_key,
                    user_auth_signing_key_option: sg.user_auth_signing_key_option,
                    pending_diff: sg.pending_diff,
                    mls_group,
                })
            })
    }

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
        transaction: &mut Transaction,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        let savepoint = transaction.savepoint()?;
        let provider = PhnxOpenMlsProvider::new(&savepoint);
        if let Some(mut group) = Group::load(&savepoint, group_id)? {
            group.mls_group.delete(provider.storage())?;
        };
        let group_id = GroupIdRefWrapper::from(group_id);
        savepoint.execute("DELETE FROM groups WHERE group_id = ?", params![group_id])?;
        savepoint.commit()?;
        Ok(())
    }
}
