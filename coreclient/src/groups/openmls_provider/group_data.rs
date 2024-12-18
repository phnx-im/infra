// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, types::FromSql, Connection, OptionalExtension, ToSql};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper, StorableGroupIdRef};

#[derive(Debug, Clone, Copy)]
pub(super) enum GroupDataType {
    JoinGroupConfig,
    Tree,
    InterimTranscriptHash,
    Context,
    ConfirmationTag,
    GroupState,
    MessageSecrets,
    ResumptionPskStore,
    OwnLeafIndex,
    UseRatchetTreeExtension,
    GroupEpochSecrets,
}

impl ToSql for GroupDataType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            GroupDataType::JoinGroupConfig => "join_group_config".to_sql(),
            GroupDataType::Tree => "tree".to_sql(),
            GroupDataType::InterimTranscriptHash => "interim_transcript_hash".to_sql(),
            GroupDataType::Context => "context".to_sql(),
            GroupDataType::ConfirmationTag => "confirmation_tag".to_sql(),
            GroupDataType::GroupState => "group_state".to_sql(),
            GroupDataType::MessageSecrets => "message_secrets".to_sql(),
            GroupDataType::ResumptionPskStore => "resumption_psk_store".to_sql(),
            GroupDataType::OwnLeafIndex => "own_leaf_index".to_sql(),
            GroupDataType::UseRatchetTreeExtension => "use_ratchet_tree_extension".to_sql(),
            GroupDataType::GroupEpochSecrets => "group_epoch_secrets".to_sql(),
        }
    }
}

impl FromSql for GroupDataType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = String::column_result(value)?;
        match value.as_str() {
            "join_group_config" => Ok(GroupDataType::JoinGroupConfig),
            "tree" => Ok(GroupDataType::Tree),
            "interim_transcript_hash" => Ok(GroupDataType::InterimTranscriptHash),
            "context" => Ok(GroupDataType::Context),
            "confirmation_tag" => Ok(GroupDataType::ConfirmationTag),
            "group_state" => Ok(GroupDataType::GroupState),
            "message_secrets" => Ok(GroupDataType::MessageSecrets),
            "resumption_psk_store" => Ok(GroupDataType::ResumptionPskStore),
            "own_leaf_index" => Ok(GroupDataType::OwnLeafIndex),
            "use_ratchet_tree_extension" => Ok(GroupDataType::UseRatchetTreeExtension),
            "group_epoch_secrets" => Ok(GroupDataType::GroupEpochSecrets),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

pub(crate) struct StorableGroupData<GroupData: Entity<CURRENT_VERSION>>(pub GroupData);

impl<GroupData: Entity<CURRENT_VERSION>> Storable for StorableGroupData<GroupData> {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS group_data (
            group_id BLOB NOT NULL,
            data_type TEXT NOT NULL CHECK (data_type IN (
                'join_group_config', 
                'tree', 
                'interim_transcript_hash',
                'context', 
                'confirmation_tag', 
                'group_state', 
                'message_secrets', 
                'resumption_psk_store',
                'own_leaf_index',
                'use_ratchet_tree_extension',
                'group_epoch_secrets'
            )),
            group_data BLOB NOT NULL,
            PRIMARY KEY (group_id, data_type)
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(payload) = row.get(0)?;
        Ok(Self(payload))
    }
}

pub(super) struct StorableGroupDataRef<'a, GroupData: Entity<CURRENT_VERSION>>(pub &'a GroupData);

impl<GroupData: Entity<CURRENT_VERSION>> StorableGroupData<GroupData> {
    pub(super) fn load<GroupId: Key<CURRENT_VERSION>>(
        connection: &Connection,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> Result<Option<GroupData>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT group_data FROM group_data WHERE group_id = ? AND data_type = ?")?;
        stmt.query_row(params![KeyRefWrapper(group_id), data_type], Self::from_row)
            .map(|x| x.0)
            .optional()
    }
}

impl<GroupData: Entity<CURRENT_VERSION>> StorableGroupDataRef<'_, GroupData> {
    pub(super) fn store<GroupId: Key<CURRENT_VERSION>>(
        &self,
        connection: &Connection,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO group_data (group_id, data_type, group_data) VALUES (?, ?, ?)",
            params![KeyRefWrapper(group_id), data_type, EntityRefWrapper(self.0)],
        )?;
        Ok(())
    }
}

impl<GroupId: Key<CURRENT_VERSION>> StorableGroupIdRef<'_, GroupId> {
    pub(super) fn delete_group_data(
        &self,
        connection: &Connection,
        data_type: GroupDataType,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM group_data WHERE group_id = ? AND data_type = ?",
            params![KeyRefWrapper(self.0), data_type],
        )?;
        Ok(())
    }
}
