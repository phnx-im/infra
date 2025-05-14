// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::ensure;
use openmls::group::{GroupId, MlsGroup};
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    codec::{BlobDecoded, BlobEncoded},
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::ear::keys::{GroupStateEarKey, IdentityLinkWrapperKey},
};
use sqlx::{SqliteExecutor, SqliteTransaction, query, query_as};

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper};

use super::{Group, diff::StagedGroupDiff, openmls_provider::PhnxOpenMlsProvider};

struct SqlGroup {
    group_id: GroupIdWrapper,
    leaf_signer: PseudonymousCredentialSigningKey,
    identity_link_wrapper_key: IdentityLinkWrapperKey,
    group_state_ear_key: GroupStateEarKey,
    pending_diff: Option<BlobDecoded<StagedGroupDiff>>,
    room_state: Vec<u8>,
}

impl SqlGroup {
    fn into_group(self, mls_group: MlsGroup) -> Group {
        let Self {
            group_id: GroupIdWrapper(group_id),
            leaf_signer,
            identity_link_wrapper_key,
            group_state_ear_key,
            pending_diff,
            room_state,
        } = self;
        Group {
            group_id,
            leaf_signer,
            identity_link_wrapper_key,
            group_state_ear_key,
            mls_group,
            pending_diff: pending_diff.map(|BlobDecoded(diff)| diff),
            room_state: serde_json::from_slice(&room_state).unwrap(),
        }
    }
}

impl Group {
    pub(crate) async fn store(&self, txn: &mut SqliteTransaction<'_>) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        let room_state = serde_json::to_vec(&self.room_state).unwrap();
        let pending_diff = self.pending_diff.as_ref().map(BlobEncoded);

        query!(
            "INSERT INTO groups (
                group_id,
                leaf_signer,
                identity_link_wrapper_key,
                group_state_ear_key,
                pending_diff,
                room_state
            )
            VALUES (?, ?, ?, ?, ?, ?)",
            group_id,
            self.leaf_signer,
            self.identity_link_wrapper_key,
            self.group_state_ear_key,
            pending_diff,
            room_state,
        )
        .execute(&mut **txn)
        .await?;
        Ok(())
    }

    pub async fn load_clean(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
    ) -> anyhow::Result<Option<Self>> {
        let Some(group) = Group::load(connection, group_id).await? else {
            return Ok(None);
        };

        ensure!(
            group.mls_group.pending_commit().is_none(),
            "Room already had a staging commit"
        );

        Ok(Some(group))
    }

    pub(crate) async fn load(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
    ) -> sqlx::Result<Option<Self>> {
        let Some(mls_group) =
            MlsGroup::load(PhnxOpenMlsProvider::new(connection).storage(), group_id)?
        else {
            return Ok(None);
        };
        let group_id = GroupIdRefWrapper::from(group_id);
        query_as!(
            SqlGroup,
            r#"SELECT
                group_id AS "group_id: _",
                leaf_signer AS "leaf_signer: _",
                identity_link_wrapper_key AS "identity_link_wrapper_key: _",
                group_state_ear_key AS "group_state_ear_key: _",
                pending_diff AS "pending_diff: _",
                room_state AS "room_state: _"
            FROM groups WHERE group_id = ?"#,
            group_id
        )
        .fetch_optional(connection)
        .await
        .map(|res| res.map(|group| SqlGroup::into_group(group, mls_group)))
    }

    pub(crate) async fn store_update(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        let pending_diff = self.pending_diff.as_ref().map(BlobEncoded);
        let room_state = serde_json::to_vec(&self.room_state).unwrap();
        query!(
            "UPDATE groups SET
                leaf_signer = ?,
                identity_link_wrapper_key = ?,
                group_state_ear_key = ?,
                pending_diff = ?,
                room_state = ?
            WHERE group_id = ?",
            self.leaf_signer,
            self.identity_link_wrapper_key,
            self.group_state_ear_key,
            pending_diff,
            room_state,
            group_id,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_from_db(
        txn: &mut sqlx::SqliteTransaction<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<()> {
        if let Some(mut group) = Group::load(&mut *txn, group_id).await? {
            let provider = PhnxOpenMlsProvider::new(&mut *txn);
            group.mls_group.delete(provider.storage())?;
        };
        let group_id = GroupIdRefWrapper::from(group_id);
        query!("DELETE FROM groups WHERE group_id = ?", group_id)
            .execute(&mut **txn)
            .await?;
        Ok(())
    }

    pub(crate) async fn load_all_group_ids(
        connection: &mut sqlx::SqliteConnection,
    ) -> sqlx::Result<Vec<GroupId>> {
        struct SqlGroupId {
            group_id: GroupIdWrapper,
        }
        let group_ids = query_as!(
            SqlGroupId,
            r#"SELECT group_id AS "group_id: _" FROM groups"#,
        )
        .fetch_all(connection)
        .await?;

        Ok(group_ids
            .into_iter()
            .map(
                |SqlGroupId {
                     group_id: GroupIdWrapper(group_id),
                 }| group_id,
            )
            .collect())
    }
}
