// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::borrow::Cow;

use openmls::group::GroupId;
use phnxtypes::identifiers::QualifiedUserNameError;
use rusqlite::types::FromSqlError;
use sqlx::{
    encode::IsNull, error::BoxDynError, sqlite::SqliteValueRef, Database, Decode, Encode, Sqlite,
    Type,
};
use tracing::error;
use uuid::Uuid;

use crate::{utils::persistence::GroupIdWrapper, ConversationMessageId};

use super::{ConversationId, ConversationStatus, ConversationType, InactiveConversation};

impl<DB> Type<DB> for ConversationId
where
    DB: Database,
    Uuid: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <Uuid as Type<DB>>::type_info()
    }
}

impl<'q, DB> Encode<'q, DB> for ConversationId
where
    DB: Database,
    Uuid: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Uuid as Encode<DB>>::encode_by_ref(&self.uuid, buf)
    }
}

impl<'r, DB> Decode<'r, DB> for ConversationId
where
    DB: Database,
    Uuid: Decode<'r, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <Uuid as Decode<DB>>::decode(value)?;
        Ok(Self::from(value))
    }
}

impl<DB> Type<DB> for ConversationMessageId
where
    DB: Database,
    Uuid: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <Uuid as Type<DB>>::type_info()
    }
}

impl<'q, DB> Encode<'q, DB> for ConversationMessageId
where
    DB: Database,
    Uuid: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Uuid as Encode<DB>>::encode(self.uuid(), buf)
    }
}

impl<'r, DB> Decode<'r, DB> for ConversationMessageId
where
    DB: Database,
    Uuid: Decode<'r, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <Uuid as Decode<DB>>::decode(value)?;
        Ok(Self::new(value))
    }
}

impl ConversationStatus {
    pub(super) fn db_value(&self) -> Cow<'static, str> {
        match self {
            Self::Active => "active".into(),
            Self::Inactive(inactive_conversation) => {
                // TODO: use itertools to avoid allocation of Vec
                let user_names = inactive_conversation
                    .past_members()
                    .iter()
                    .map(|user_name| user_name.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("inactive:{user_names}").into()
            }
        }
    }

    pub(super) fn from_db_value(
        value: &str,
    ) -> Result<ConversationStatus, ConversationStatusFromDbError> {
        if value.starts_with("active") {
            return Ok(Self::Active);
        }
        let Some(user_names) = value.strip_prefix("inactive:") else {
            return Err(ConversationStatusFromDbError::InvalidType);
        };
        let user_names = user_names
            .split(',')
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()
            .inspect_err(|error| {
                error!(%error, "Failed to parse user names from database");
            })?;
        Ok(Self::Inactive(InactiveConversation::new(user_names)))
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ConversationStatusFromDbError {
    #[error("Invalid type")]
    InvalidType,
    #[error(transparent)]
    QualifiedUserName(#[from] QualifiedUserNameError),
}

impl From<ConversationStatusFromDbError> for FromSqlError {
    fn from(e: ConversationStatusFromDbError) -> Self {
        match e {
            ConversationStatusFromDbError::InvalidType => Self::InvalidType,
            ConversationStatusFromDbError::QualifiedUserName(e) => Self::Other(Box::new(e)),
        }
    }
}

impl Type<Sqlite> for ConversationStatus {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Cow<str> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ConversationStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }

    fn encode(
        self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ConversationStatus {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Sqlite>>::decode(value)?;
        Ok(Self::from_db_value(value)?)
    }
}

impl ConversationType {
    pub(super) fn db_value(&self) -> Cow<'static, str> {
        match self {
            Self::UnconfirmedConnection(user_name) => {
                format!("unconfirmed_connection:{user_name}").into()
            }
            Self::Connection(user_name) => format!("connection:{user_name}").into(),
            Self::Group => "group".into(),
        }
    }

    pub(super) fn from_db_value(
        value: &str,
    ) -> Result<ConversationType, ConversationTypeFromDbError> {
        if value.starts_with("group") {
            return Ok(Self::Group);
        }
        let Some((conversation_type, user_name)) = value.split_once(':') else {
            return Err(ConversationTypeFromDbError::InvalidType);
        };
        match conversation_type {
            "unconfirmed_connection" => Ok(Self::UnconfirmedConnection(
                user_name.parse().inspect_err(|error| {
                    error!(%error, "Failed to parse user name from database");
                })?,
            )),
            "connection" => Ok(Self::Connection(user_name.parse().inspect_err(
                |error| {
                    error!(%error, "Failed to parse user name from database");
                },
            )?)),
            _ => Err(ConversationTypeFromDbError::InvalidType),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ConversationTypeFromDbError {
    #[error("Invalid type")]
    InvalidType,
    #[error(transparent)]
    QualifiedUserName(#[from] QualifiedUserNameError),
}

impl From<ConversationTypeFromDbError> for FromSqlError {
    fn from(e: ConversationTypeFromDbError) -> Self {
        match e {
            ConversationTypeFromDbError::InvalidType => Self::InvalidType,
            ConversationTypeFromDbError::QualifiedUserName(e) => Self::Other(Box::new(e)),
        }
    }
}

impl Type<Sqlite> for ConversationType {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Cow<str> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ConversationType {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }

    fn encode(
        self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ConversationType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Sqlite>>::decode(value)?;
        Ok(Self::from_db_value(value)?)
    }
}

impl Type<Sqlite> for GroupIdWrapper {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for GroupIdWrapper {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&[u8] as Decode<Sqlite>>::decode(value)?;
        Ok(GroupIdWrapper(GroupId::from_slice(value)))
    }
}
