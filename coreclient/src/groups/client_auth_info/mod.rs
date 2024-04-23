// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::Result;
use openmls::{credentials::Credential, group::GroupId, prelude::LeafNodeIndex};
use phnxtypes::{
    credentials::{
        infra_credentials::{InfraCredential, InfraCredentialPlaintext, InfraCredentialTbs},
        ClientCredential, EncryptedClientCredential,
    },
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, EncryptedSignatureEarKey, SignatureEarKey,
                SignatureEarKeyWrapperKey,
            },
            EarDecryptable,
        },
        signatures::signable::Verifiable,
    },
    identifiers::AsClientId,
};
use rusqlite::{params, types::FromSqlError, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    key_stores::as_credentials::AsCredentialStore,
    utils::persistence::{Storable, Triggerable},
};

use super::decrypt_and_verify_client_credential;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StorableClientCredential {
    client_credential: ClientCredential,
}

impl From<ClientCredential> for StorableClientCredential {
    fn from(client_credential: ClientCredential) -> Self {
        Self { client_credential }
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

    pub(super) async fn decrypt_and_verify_all(
        ear_key: &ClientCredentialEarKey,
        as_credential_store: &AsCredentialStore<'_>,
        encrypted_client_credentials: impl IntoIterator<Item = EncryptedClientCredential>,
    ) -> Result<Vec<Self>> {
        let mut client_credentials = Vec::new();
        for ctxt in encrypted_client_credentials.into_iter() {
            let client_credential =
                Self::decrypt_and_verify(ear_key, as_credential_store, ctxt).await?;
            client_credentials.push(client_credential);
        }
        Ok(client_credentials)
    }

    pub(super) async fn decrypt_and_verify(
        ear_key: &ClientCredentialEarKey,
        as_credential_store: &AsCredentialStore<'_>,
        ecc: EncryptedClientCredential,
    ) -> Result<Self> {
        let client_credential =
            decrypt_and_verify_client_credential(as_credential_store, ear_key, &ecc).await?;
        Ok(Self { client_credential })
    }
}

impl StorableClientCredential {
    /// Load the [`ClientAuthInfo`] for the given client_id from the database.
    pub(super) fn load(connection: &Connection, client_id: &AsClientId) -> Result<Option<Self>> {
        let mut stmt = connection
            .prepare("SELECT client_credential FROM client_auth_infos WHERE client_id = ?")?;
        let client_credential_option = stmt
            .query_row(params![client_id.to_string()], |row| {
                let client_credential_bytes: Vec<u8> = row.get(0)?;
                let client_credential: ClientCredential =
                    serde_json::from_slice(&client_credential_bytes)
                        .map_err(|e| FromSqlError::Other(e.into()))?;
                Ok(Self::new(client_credential))
            })
            .optional()?;

        Ok(client_credential_option)
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<()> {
        let client_credential_bytes = serde_json::to_vec(&self.client_credential)?;
        let fingerprint = self.hash().as_bytes();
        connection.execute(
            "INSERT INTO client_credentials (fingerprint, client_id, client_credential) VALUES (?, ?, ?)",
            params![
                fingerprint,
                self.client_credential.identity().to_string(),
                client_credential_bytes
            ],
        )?;
        Ok(())
    }
}

impl Storable for StorableClientCredential {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS client_credentials (
                fingerprint BLOB PRIMARY KEY,
                client_id TEXT NOT NULL,
                client_credential BLOB NOT NULL,
            )";
}

pub(crate) struct GroupMembership {
    client_id: AsClientId,
    group_id: GroupId,
    signature_ear_key: SignatureEarKey,
    leaf_index: LeafNodeIndex,
}

impl GroupMembership {
    pub(super) fn new(
        client_id: AsClientId,
        group_id: GroupId,
        leaf_index: LeafNodeIndex,
        signature_ear_key: SignatureEarKey,
    ) -> Self {
        Self {
            client_id,
            group_id,
            leaf_index,
            signature_ear_key,
        }
    }

    pub(super) fn store(&self, connection: &Connection) -> Result<()> {
        connection.execute(
            "INSERT INTO group_membership (client_id, group_id, leaf_index, signature_ear_key) VALUES (?, ?, ?, ?)",
            params![
                self.client_id.to_string(),
                self.group_id.to_string(),
                self.leaf_index.usize() as i64,
                self.signature_ear_key.as_ref()
            ],
        )?;
        Ok(())
    }

    pub(super) fn load(connection: &Connection, client_id: &AsClientId) -> Result<Vec<Self>> {
        let mut stmt = connection.prepare("SELECT group_id, leaf_index, signature_ear_key FROM group_membership WHERE client_id = ?")?;
        let group_memberships = stmt.query_map(params![client_id.to_string()], |row| {
            let group_id: String = row.get(0)?;
            let leaf_index: i64 = row.get(1)?;
            let signature_ear_key: Vec<u8> = row.get(2)?;
            Ok(Self {
                client_id: client_id.clone(),
                group_id: group_id.parse()?,
                leaf_index: LeafNodeIndex::from(leaf_index as usize),
                signature_ear_key: SignatureEarKey::from(signature_ear_key),
            })
        })?;
        let mut group_memberships = group_memberships.collect::<Result<Vec<Self>, _>>()?;
        group_memberships.sort_by_key(|gm| gm.group_id.clone());
        Ok(group_memberships)
    }
}

impl Storable for GroupMembership {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS group_membership (
                client_credential_fingerprint BLOB NOT NULL,
                group_id TEXT NOT NULL,
                client_id TEXT NOT NULL,
                leaf_index INTEGER NOT NULL,
                signature_ear_key BLOB NOT NULL,
                FOREIGN KEY (client_credential_fingerprint) REFERENCES client_credentials(fingerprint),
                FOREIGN KEY (group_id) REFERENCES mlsgroup(primary_key),
                PRIMARY KEY (client_id, group_id)
            )";
}

impl Triggerable for GroupMembership {
    const CREATE_TRIGGER_STATEMENT: &'static str = "CREATE TRIGGER IF NOT EXISTS delete_orphaned_client_credentials AFTER DELETE ON group_membership
        BEGIN
            DELETE FROM client_credentials
            WHERE fingerprint = OLD.fingerprint AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_credential_fingerprint = OLD.fingerprint
            );
        END";
}

pub(super) struct ClientAuthInfo {
    client_credential: StorableClientCredential,
    group_membership: GroupMembership,
}

impl ClientAuthInfo {
    pub(super) async fn decrypt_and_verify(
        group_id: &GroupId,
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        encrypted_client_information: impl IntoIterator<
            Item = Option<(EncryptedClientCredential, EncryptedSignatureEarKey)>,
        >,
    ) -> Result<Vec<Self>> {
        let mut client_information = Vec::new();
        for (index, client_info_option) in encrypted_client_information.into_iter().enumerate() {
            if let Some((ecc, esek)) = client_info_option {
                let client_credential =
                    StorableClientCredential::decrypt_and_verify(ear_key, as_credential_store, ecc)
                        .await?;
                let signature_ear_key = SignatureEarKey::decrypt(wrapper_key, &esek)?;
                let group_membership = GroupMembership::new(
                    client_credential.identity(),
                    group_id.clone(),
                    LeafNodeIndex::new(index.try_into()?),
                    signature_ear_key,
                );
                let client_auth_info = ClientAuthInfo {
                    client_credential,
                    group_membership,
                };
                client_information.push(client_auth_info);
            }
        }
        Ok(client_information)
    }

    pub(super) fn verify_infra_credential(&self, credential: &Credential) -> Result<()> {
        let infra_credential = InfraCredential::try_from(credential.clone())?;

        // Verify the leaf credential
        let credential_plaintext = InfraCredentialPlaintext::decrypt(
            &infra_credential,
            &self.group_membership.signature_ear_key,
        )?;
        credential_plaintext
            .verify::<InfraCredentialTbs>(self.client_credential().verifying_key())?;
        Ok(())
    }

    pub(super) fn store(&self, connection: &Connection) -> Result<()> {
        self.client_credential.store(connection)?;
        self.group_membership.store(connection)?;
        Ok(())
    }
}
