// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::{Result, anyhow};
use openmls::{credentials::Credential, group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    credentials::{ClientCredential, CredentialFingerprint, VerifiableClientCredential},
    identifiers::UserId,
};
use sqlx::{SqliteConnection, SqliteExecutor};

use crate::{clients::api_clients::ApiClients, key_stores::as_credentials::AsCredentials};

pub(crate) mod persistence;

#[derive(Debug, Clone)]
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

    pub(crate) async fn verify(
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
        verifiable_client_credential: VerifiableClientCredential,
    ) -> Result<Self> {
        let client_credential = AsCredentials::verify_client_credential(
            connection,
            api_clients,
            verifiable_client_credential,
        )
        .await?;
        Ok(Self { client_credential })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GroupMembership {
    user_id: UserId,
    client_credential_fingerprint: CredentialFingerprint,
    group_id: GroupId,
    leaf_index: LeafNodeIndex,
}

impl GroupMembership {
    pub(super) fn new(
        user_id: UserId,
        group_id: GroupId,
        leaf_index: LeafNodeIndex,
        client_credential_fingerprint: CredentialFingerprint,
    ) -> Self {
        Self {
            user_id,
            client_credential_fingerprint,
            group_id,
            leaf_index,
        }
    }

    // Computes free indices based on existing leaf indices and staged removals.
    // Not that staged additions are not considered.
    pub(super) async fn free_indices(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> Result<impl Iterator<Item = LeafNodeIndex> + 'static> {
        let leaf_indices = Self::member_indices(executor, group_id).await?;
        let highest_index = leaf_indices
            .last()
            .cloned()
            .unwrap_or(LeafNodeIndex::new(0));
        let free_indices = (0..highest_index.u32())
            .filter(move |index| !leaf_indices.contains(&LeafNodeIndex::new(*index)))
            .chain(highest_index.u32() + 1..)
            .map(LeafNodeIndex::new);
        Ok(free_indices)
    }

    /// Set the group member's leaf index. This can be required for resync
    /// operations.
    pub(super) fn set_leaf_index(&mut self, leaf_index: LeafNodeIndex) {
        self.leaf_index = leaf_index;
    }

    pub(crate) fn user_id(&self) -> &UserId {
        &self.user_id
    }
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

    /// Verify the given credentials
    pub(super) async fn verify_credentials(
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
        group_id: &GroupId,
        client_credentials: impl IntoIterator<Item = (LeafNodeIndex, Credential)>,
    ) -> Result<Vec<Self>> {
        let mut client_auth_infos = Vec::new();
        for (leaf_index, credential) in client_credentials {
            let client_auth_info =
                Self::verify_credential(connection, api_clients, group_id, leaf_index, credential)
                    .await?;
            client_auth_infos.push(client_auth_info);
        }
        Ok(client_auth_infos)
    }

    /// Verify the given credential
    pub(super) async fn verify_credential(
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
        credential: Credential,
    ) -> Result<Self> {
        // Verify the leaf credential
        let client_credential =
            StorableClientCredential::verify(connection, api_clients, credential.try_into()?)
                .await?;
        let group_membership = GroupMembership::new(
            client_credential.identity().clone(),
            group_id.clone(),
            leaf_index,
            client_credential.fingerprint(),
        );
        let client_auth_info = ClientAuthInfo {
            client_credential,
            group_membership,
        };
        Ok(client_auth_info)
    }

    pub(super) async fn stage_update(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> sqlx::Result<()> {
        self.client_credential.store(&mut *connection).await?;
        self.group_membership.stage_update(&mut *connection).await?;
        Ok(())
    }

    pub(super) async fn stage_add(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> sqlx::Result<()> {
        self.client_credential.store(&mut *connection).await?;
        self.group_membership.stage_add(&mut *connection).await?;
        Ok(())
    }

    pub(crate) async fn store(&self, connection: &mut sqlx::SqliteConnection) -> Result<()> {
        self.client_credential.store(&mut *connection).await?;
        self.group_membership.store(&mut *connection).await?;
        Ok(())
    }

    pub(super) async fn load(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) =
            GroupMembership::load(&mut *connection, group_id, leaf_index).await?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            &mut *connection,
            &group_membership.client_credential_fingerprint,
        )
        .await?
        .ok_or_else(|| {
            anyhow!("Found a matching Groupmembership, but no matching ClientCredential")
        })?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) async fn load_staged(
        connection: &mut sqlx::SqliteConnection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) =
            GroupMembership::load_staged(&mut *connection, group_id, leaf_index).await?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            connection,
            &group_membership.client_credential_fingerprint,
        )
        .await?
        .ok_or_else(|| {
            anyhow!("Found a matching Groupmembership, but no matching ClientCredential")
        })?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) fn client_credential(&self) -> &StorableClientCredential {
        &self.client_credential
    }

    pub(super) fn into_client_credential(self) -> ClientCredential {
        self.client_credential.into()
    }

    pub(super) fn group_membership(&self) -> &GroupMembership {
        &self.group_membership
    }

    pub(super) fn group_membership_mut(&mut self) -> &mut GroupMembership {
        &mut self.group_membership
    }
}
