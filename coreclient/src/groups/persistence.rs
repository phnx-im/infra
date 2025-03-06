// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::{GroupId, MlsGroup};
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::ear::keys::{GroupStateEarKey, IdentityLinkWrapperKey},
};
use sqlx::{query, query_as, SqliteExecutor};

use crate::utils::persistence::{GroupIdRefWrapper, GroupIdWrapper};

use super::{diff::StagedGroupDiff, openmls_provider::PhnxOpenMlsProvider, Group};

struct SqlGroup {
    group_id: GroupIdWrapper,
    leaf_signer: PseudonymousCredentialSigningKey,
    identity_link_wrapper_key: IdentityLinkWrapperKey,
    group_state_ear_key: GroupStateEarKey,
    pending_diff: Option<StagedGroupDiff>,
}

impl SqlGroup {
    fn into_group(self, mls_group: MlsGroup) -> Group {
        let Self {
            group_id: GroupIdWrapper(group_id),
            leaf_signer,
            identity_link_wrapper_key,
            group_state_ear_key,
            pending_diff,
        } = self;
        Group {
            group_id,
            leaf_signer,
            identity_link_wrapper_key,
            group_state_ear_key,
            mls_group,
            pending_diff,
        }
    }
}

impl Group {
    pub(crate) async fn store(&self, connection: &mut sqlx::SqliteConnection) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        query!(
            "INSERT INTO groups (
                group_id,
                leaf_signer,
                identity_link_wrapper_key,
                group_state_ear_key,
                pending_diff
            )
            VALUES (?, ?, ?, ?, ?)",
            group_id,
            self.leaf_signer,
            self.identity_link_wrapper_key,
            self.group_state_ear_key,
            self.pending_diff,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
    ) -> sqlx::Result<Option<Self>> {
        let Some(mls_group) = MlsGroup::load(
            PhnxOpenMlsProvider::new(&mut *connection).storage(),
            group_id,
        )?
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
                pending_diff AS "pending_diff: _"
            FROM groups WHERE group_id = ?"#,
            group_id
        )
        .fetch_optional(connection)
        .await
        .map(|res| res.map(|group| SqlGroup::into_group(group, mls_group)))
    }

    pub(crate) async fn store_update(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        query!(
            "UPDATE groups SET
                leaf_signer = ?,
                identity_link_wrapper_key = ?,
                group_state_ear_key = ?,
                pending_diff = ?
            WHERE group_id = ?",
            self.leaf_signer,
            self.identity_link_wrapper_key,
            self.group_state_ear_key,
            self.pending_diff,
            group_id,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_from_db(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
    ) -> sqlx::Result<()> {
        use sqlx::Connection;
        let mut transaction = connection.begin().await?;

        if let Some(mut group) = Group::load(&mut transaction, group_id).await? {
            let provider = PhnxOpenMlsProvider::new(&mut transaction);
            group.mls_group.delete(provider.storage())?;
        };
        let group_id = GroupIdRefWrapper::from(group_id);
        query!("DELETE FROM groups WHERE group_id = ?", group_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(())
    }
}
