// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::{anyhow, Result};
use openmls::{credentials::Credential, group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    credentials::{
        pseudonymous_credentials::PseudonymousCredential, ClientCredential, CredentialFingerprint,
        VerifiableClientCredential,
    },
    crypto::ear::{
        keys::{EncryptedIdentityLinkKey, IdentityLinkKey, IdentityLinkWrapperKey},
        EarDecryptable,
    },
    identifiers::AsClientId,
};
use rusqlite::Connection;

use crate::{
    clients::api_clients::ApiClients, key_stores::as_credentials::AsCredentials,
    utils::persistence::SqliteConnection,
};

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
        connection: SqliteConnection,
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

#[derive(Debug)]
pub(crate) struct GroupMembership {
    client_id: AsClientId,
    client_credential_fingerprint: CredentialFingerprint,
    group_id: GroupId,
    identity_link_key: IdentityLinkKey,
    leaf_index: LeafNodeIndex,
}

impl GroupMembership {
    pub(super) fn new(
        client_id: AsClientId,
        group_id: GroupId,
        leaf_index: LeafNodeIndex,
        identity_link_key: IdentityLinkKey,
        client_credential_fingerprint: CredentialFingerprint,
    ) -> Self {
        Self {
            client_id,
            client_credential_fingerprint,
            group_id,
            leaf_index,
            identity_link_key,
        }
    }

    // Computes free indices based on existing leaf indices and staged removals.
    // Not that staged additions are not considered.
    pub(super) fn free_indices(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<impl Iterator<Item = LeafNodeIndex>> {
        let leaf_indices = Self::member_indices(connection, group_id)?;
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

    /// Get the identity link key.
    pub(super) fn identity_link_key(&self) -> &IdentityLinkKey {
        &self.identity_link_key
    }

    /// Set the group member's leaf index. This can be required for resync
    /// operations.
    pub(super) fn set_leaf_index(&mut self, leaf_index: LeafNodeIndex) {
        self.leaf_index = leaf_index;
    }

    pub(crate) fn client_id(&self) -> &AsClientId {
        &self.client_id
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

    /// Decrypt and verify the given encrypted client auth info. This includes
    /// verification of the signature over the [`PseudonymousCredential`], as
    /// well as the signature over the [`ClientCredential`].
    pub(super) async fn decrypt_and_verify_all(
        connection: SqliteConnection,
        api_clients: &ApiClients,
        group_id: &GroupId,
        wrapper_key: &IdentityLinkWrapperKey,
        encrypted_client_information: impl Iterator<
            Item = ((LeafNodeIndex, Credential), EncryptedIdentityLinkKey),
        >,
    ) -> Result<Vec<Self>> {
        let mut client_information = Vec::new();
        for ((leaf_index, credential), encrypted_identity_link_key) in encrypted_client_information
        {
            let client_auth_info = Self::decrypt_and_verify(
                connection.clone(),
                api_clients,
                group_id,
                wrapper_key,
                encrypted_identity_link_key,
                leaf_index,
                credential,
            )
            .await?;
            client_information.push(client_auth_info);
        }
        Ok(client_information)
    }

    /// Decrypt and verify the given credential.
    pub(super) async fn decrypt_credential_and_verify(
        connection: SqliteConnection,
        api_clients: &ApiClients,
        group_id: &GroupId,
        identity_link_key: IdentityLinkKey,
        leaf_index: LeafNodeIndex,
        credential: Credential,
    ) -> Result<Self> {
        let pseudonymous_credential = PseudonymousCredential::try_from(credential)?;
        // Verify the leaf credential
        let credential_plaintext =
            pseudonymous_credential.decrypt_and_verify(&identity_link_key)?;
        let client_credential = StorableClientCredential::verify(
            connection,
            api_clients,
            credential_plaintext.client_credential,
        )
        .await?;
        let group_membership = GroupMembership::new(
            client_credential.identity(),
            group_id.clone(),
            leaf_index,
            identity_link_key,
            client_credential.fingerprint(),
        );
        let client_auth_info = ClientAuthInfo {
            client_credential,
            group_membership,
        };
        Ok(client_auth_info)
    }

    /// Decrypt and verify the given identity link key and credential.
    pub(super) async fn decrypt_and_verify(
        connection: SqliteConnection,
        api_clients: &ApiClients,
        group_id: &GroupId,
        wrapper_key: &IdentityLinkWrapperKey,
        encrypted_identity_link_key: EncryptedIdentityLinkKey,
        leaf_index: LeafNodeIndex,
        credential: Credential,
    ) -> Result<Self> {
        let identity_link_key =
            IdentityLinkKey::decrypt(wrapper_key, &encrypted_identity_link_key)?;
        Self::decrypt_credential_and_verify(
            connection,
            api_clients,
            group_id,
            identity_link_key,
            leaf_index,
            credential,
        )
        .await
    }

    pub(super) fn stage_update(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.stage_update(connection)?;
        Ok(())
    }

    pub(super) fn stage_add(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.stage_add(connection)?;
        Ok(())
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.store(connection)?;
        Ok(())
    }

    pub(super) fn load(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) = GroupMembership::load(connection, group_id, leaf_index)?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            connection,
            &group_membership.client_credential_fingerprint,
        )?
        .ok_or(anyhow!(
            "Found a matching Groupmembership, but no matching ClientCredential"
        ))?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) fn load_staged(
        connection: &Connection,
        group_id: &GroupId,
        leaf_index: LeafNodeIndex,
    ) -> Result<Option<Self>> {
        let Some(group_membership) =
            GroupMembership::load_staged(connection, group_id, leaf_index)?
        else {
            return Ok(None);
        };
        let client_credential = StorableClientCredential::load(
            connection,
            &group_membership.client_credential_fingerprint,
        )?
        .ok_or(anyhow!(
            "Found a matching Groupmembership, but no matching ClientCredential"
        ))?;
        Ok(Some(Self::new(client_credential, group_membership)))
    }

    pub(super) fn client_credential(&self) -> &StorableClientCredential {
        &self.client_credential
    }

    pub(super) fn group_membership_mut(&mut self) -> &mut GroupMembership {
        &mut self.group_membership
    }
}
