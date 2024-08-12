// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::{GroupId, MlsGroup};
use rusqlite::{
    params,
    types::{FromSql, ToSqlOutput},
    OptionalExtension, ToSql,
};

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable};

use super::Group;

struct MlsGroupWrapper {
    mls_group: MlsGroup,
}

impl FromSql for MlsGroupWrapper {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        println!("Deserializing MlsGroup");
        let mls_group = phnxtypes::codec::from_slice(value.as_blob()?).map_err(|e| {
            log::error!("Failed to deserialize MlsGroup: {:?}", e);
            rusqlite::types::FromSqlError::Other(Box::new(e))
        })?;
        println!("Successfully deserialized MlsGroup");
        Ok(MlsGroupWrapper { mls_group })
    }
}

struct MlsGroupRefWrapper<'a> {
    mls_group: &'a MlsGroup,
}

impl<'a> ToSql for MlsGroupRefWrapper<'a> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let bytes = phnxtypes::codec::to_vec(self.mls_group).map_err(|e| {
            log::error!("Failed to serialize MlsGroup: {:?}", e);
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        println!("Successfully serialized MlsGroup of length {}", bytes.len());
        println!("Deserializing MlsGroup to test");
        let _group = phnxtypes::codec::from_slice::<MlsGroup>(&bytes).unwrap();

        Ok(ToSqlOutput::from(bytes))
    }
}

impl Storable for Group {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS groups (
            group_id BLOB PRIMARY KEY,
            leaf_signer BLOB NOT NULL,
            signature_ear_key_wrapper_key BLOB NOT NULL,
            credential_ear_key BLOB NOT NULL,
            group_state_ear_key BLOB NOT NULL,
            user_auth_signing_key_option BLOB,
            mls_group BLOB NOT NULL,
            pending_diff BLOB
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let group_id: GroupIdWrapper = row.get(0)?;
        let leaf_signer = row.get(1)?;
        let signature_ear_key_wrapper_key = row.get(2)?;
        let credential_ear_key = row.get(3)?;
        let group_state_ear_key = row.get(4)?;
        let user_auth_signing_key_option = row.get(5)?;
        let mls_group: MlsGroupWrapper = row.get(6)?;
        let pending_diff = row.get(7)?;

        Ok(Group {
            group_id: group_id.into(),
            leaf_signer,
            signature_ear_key_wrapper_key,
            credential_ear_key,
            group_state_ear_key,
            user_auth_signing_key_option,
            mls_group: mls_group.mls_group,
            pending_diff,
        })
    }
}

impl Group {
    pub(crate) fn store(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        let mls_group = MlsGroupRefWrapper {
            mls_group: &self.mls_group,
        };
        connection.execute(
            "INSERT INTO groups (group_id, leaf_signer, signature_ear_key_wrapper_key, credential_ear_key, group_state_ear_key, user_auth_signing_key_option, mls_group, pending_diff) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                group_id,
                self.leaf_signer,
                self.signature_ear_key_wrapper_key,
                self.credential_ear_key,
                self.group_state_ear_key,
                self.user_auth_signing_key_option,
                mls_group,
                self.pending_diff,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn load(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut stmt = connection.prepare("SELECT * FROM groups WHERE group_id = ?")?;
        stmt.query_row(params![group_id], Self::from_row).optional()
    }

    pub(crate) fn store_update(
        &self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        let mls_group = MlsGroupRefWrapper {
            mls_group: &self.mls_group,
        };
        connection.execute(
            "UPDATE groups SET leaf_signer = ?, signature_ear_key_wrapper_key = ?, credential_ear_key = ?, group_state_ear_key = ?, user_auth_signing_key_option = ?, mls_group = ?, pending_diff = ? WHERE group_id = ?",
            params![
                self.leaf_signer,
                self.signature_ear_key_wrapper_key,
                self.credential_ear_key,
                self.group_state_ear_key,
                self.user_auth_signing_key_option,
                mls_group,
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
