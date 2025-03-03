// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, types::FromSql, Connection, OptionalExtension, ToSql};
use sqlx::{
    encode::IsNull, error::BoxDynError, sqlite::SqliteTypeInfo, Database, Decode, Encode, Sqlite,
};

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

impl GroupDataType {
    fn to_str(self) -> &'static str {
        match self {
            GroupDataType::JoinGroupConfig => "join_group_config",
            GroupDataType::Tree => "tree",
            GroupDataType::InterimTranscriptHash => "interim_transcript_hash",
            GroupDataType::Context => "context",
            GroupDataType::ConfirmationTag => "confirmation_tag",
            GroupDataType::GroupState => "group_state",
            GroupDataType::MessageSecrets => "message_secrets",
            GroupDataType::ResumptionPskStore => "resumption_psk_store",
            GroupDataType::OwnLeafIndex => "own_leaf_index",
            GroupDataType::UseRatchetTreeExtension => "use_ratchet_tree_extension",
            GroupDataType::GroupEpochSecrets => "group_epoch_secrets",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_group_config" => Some(GroupDataType::JoinGroupConfig),
            "tree" => Some(GroupDataType::Tree),
            "interim_transcript_hash" => Some(GroupDataType::InterimTranscriptHash),
            "context" => Some(GroupDataType::Context),
            "confirmation_tag" => Some(GroupDataType::ConfirmationTag),
            "group_state" => Some(GroupDataType::GroupState),
            "message_secrets" => Some(GroupDataType::MessageSecrets),
            "resumption_psk_store" => Some(GroupDataType::ResumptionPskStore),
            "own_leaf_index" => Some(GroupDataType::OwnLeafIndex),
            "use_ratchet_tree_extension" => Some(GroupDataType::UseRatchetTreeExtension),
            "group_epoch_secrets" => Some(GroupDataType::GroupEpochSecrets),
            _ => None,
        }
    }
}

impl ToSql for GroupDataType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.to_str().to_sql()
    }
}

impl FromSql for GroupDataType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = String::column_result(value)?;
        Self::from_str(&value).ok_or(rusqlite::types::FromSqlError::InvalidType)
    }
}

impl sqlx::Type<Sqlite> for GroupDataType {
    fn type_info() -> SqliteTypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for GroupDataType {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.to_str(), buf)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid group data type: {value}")]
struct InvalidGroupDataTypeError {
    value: String,
}

impl<'r> Decode<'r, Sqlite> for GroupDataType {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value: &str = Decode::<Sqlite>::decode(value)?;
        Self::from_str(value).ok_or_else(|| {
            InvalidGroupDataTypeError {
                value: value.to_string(),
            }
            .into()
        })
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
