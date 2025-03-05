// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, credentials::keys::PseudonymousCredentialSigningKey};
use rusqlite::{types::FromSql, ToSql};
use sqlx::{encode::IsNull, error::BoxDynError, prelude::Type, Database, Decode, Encode, Sqlite};

use super::*;

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
pub(crate) struct GroupDiff {
    pub(crate) leaf_signer: Option<PseudonymousCredentialSigningKey>,
    pub(crate) identity_link_key: Option<IdentityLinkWrapperKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
}

impl GroupDiff {
    pub(crate) fn new() -> Self {
        Self {
            leaf_signer: None,
            identity_link_key: None,
            group_state_ear_key: None,
        }
    }

    pub(crate) fn stage(self) -> StagedGroupDiff {
        StagedGroupDiff {
            leaf_signer: self.leaf_signer,
            identity_link_key: self.identity_link_key,
            group_state_ear_key: self.group_state_ear_key,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StagedGroupDiff {
    pub(crate) leaf_signer: Option<PseudonymousCredentialSigningKey>,
    pub(crate) identity_link_key: Option<IdentityLinkWrapperKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
}

impl ToSql for StagedGroupDiff {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let bytes = PhnxCodec::to_vec(self)?;

        Ok(rusqlite::types::ToSqlOutput::from(bytes))
    }
}

impl FromSql for StagedGroupDiff {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let staged_diff = PhnxCodec::from_slice(value.as_blob()?)?;
        Ok(staged_diff)
    }
}

impl Type<Sqlite> for StagedGroupDiff {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for StagedGroupDiff {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let bytes = PhnxCodec::to_vec(self)?;
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for StagedGroupDiff {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
        let value = PhnxCodec::from_slice(bytes)?;
        Ok(value)
    }
}
