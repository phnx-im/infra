// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
pub(crate) struct ClientAuthInfo {
    client_credential: ClientCredential,
    signature_ear_key: SignatureEarKey,
}

impl ClientAuthInfo {
    /// Create a new [`ClientAuthInfo`] with the given client credential and signature ear key.
    pub(crate) fn new(
        client_credential: ClientCredential,
        signature_ear_key: SignatureEarKey,
    ) -> Self {
        Self {
            client_credential,
            signature_ear_key,
        }
    }

    pub(super) async fn decrypt_and_verify_all(
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        encrypted_client_information: impl IntoIterator<
            Item = (EncryptedClientCredential, EncryptedSignatureEarKey),
        >,
    ) -> Result<Vec<Self>> {
        let mut client_auth_infos = Vec::new();
        for ctxt in encrypted_client_information.into_iter() {
            let client_info =
                Self::decrypt_and_verify(ear_key, wrapper_key, as_credential_store, ctxt).await?;
            client_auth_infos.push(client_info);
        }
        Ok(client_auth_infos)
    }

    pub(super) async fn decrypt_and_verify(
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        (ecc, esek): (EncryptedClientCredential, EncryptedSignatureEarKey),
    ) -> Result<Self> {
        let client_credential =
            decrypt_and_verify_client_credential(as_credential_store, ear_key, &ecc).await?;
        let signature_ear_key = SignatureEarKey::decrypt(wrapper_key, &esek)?;
        Ok(Self {
            client_credential,
            signature_ear_key,
        })
    }

    pub(super) fn verify_infra_credential(&self, credential: &Credential) -> Result<()> {
        let infra_credential = InfraCredential::try_from(credential.clone())?;

        // Verify the leaf credential
        let credential_plaintext =
            InfraCredentialPlaintext::decrypt(&infra_credential, self.signature_ear_key())?;
        credential_plaintext
            .verify::<InfraCredentialTbs>(self.client_credential().verifying_key())?;
        Ok(())
    }

    pub(super) fn client_credential(&self) -> &ClientCredential {
        &self.client_credential
    }

    fn signature_ear_key(&self) -> &SignatureEarKey {
        &self.signature_ear_key
    }
}

impl ClientAuthInfo {
    /// Load the [`ClientAuthInfo`] for the given client_id from the database.
    pub(super) fn load(connection: &Connection, client_id: &AsClientId) -> Result<Option<Self>> {
        let mut stmt = connection.prepare("SELECT client_credential, signature_ear_key FROM client_auth_infos WHERE client_id = ?")?;
        let client_auth_info = stmt
            .query_row(params![client_id.to_string()], |row| {
                let client_credential_bytes: Vec<u8> = row.get(0)?;
                let client_credential: ClientCredential =
                    serde_json::from_slice(&client_credential_bytes)
                        .map_err(|e| FromSqlError::Other(e.into()))?;
                let signature_ear_key_bytes: Vec<u8> = row.get(1)?;
                let signature_ear_key: SignatureEarKey =
                    serde_json::from_slice(&signature_ear_key_bytes)
                        .map_err(|e| FromSqlError::Other(e.into()))?;
                Ok(Self::new(client_credential, signature_ear_key))
            })
            .optional()?;

        Ok(client_auth_info)
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<()> {
        let client_credential_bytes = serde_json::to_vec(&self.client_credential)?;
        let signature_ear_key_bytes = serde_json::to_vec(&self.signature_ear_key)?;
        connection.execute(
            "INSERT INTO client_auth_infos (client_id, client_credential, signature_ear_key) VALUES (?, ?, ?)",
            params![self.client_credential.identity().to_string(), client_credential_bytes, signature_ear_key_bytes],
        )?;
        Ok(())
    }
}

impl Storable for ClientAuthInfo {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS client_auth_infos (
                rowid INTEGER PRIMARY KEY,
                client_id TEXT NOT NULL,
                client_credential BLOB NOT NULL,
                signature_ear_key BLOB NOT NULL,
            )";
}

pub(crate) struct GroupMembership {
    client_id: AsClientId,
    group_id: GroupId,
    leaf_index: LeafNodeIndex,
}

impl Storable for GroupMembership {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS group_membership (
                client_auth_info_rowid INTEGER NOT NULL,
                group_id TEXT NOT NULL,
                client_id TEXT NOT NULL,
                leaf_index INTEGER NOT NULL,
                FOREIGN KEY (client_auth_info_rowid) REFERENCES client_auth_infos(rowid),
                FOREIGN KEY (group_id) REFERENCES mlsgroup(primary_key),
                PRIMARY KEY (client_id, group_id)
            )";
}

impl Triggerable for GroupMembership {
    const CREATE_TRIGGER_STATEMENT: &'static str = "CREATE TRIGGER IF NOT EXISTS delete_orphaned_client_auth_infos AFTER DELETE ON group_membership
        BEGIN
            DELETE FROM client_auth_infos
            WHERE rowid = OLD.rowid AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_auth_info_rowid = OLD.rowid
            );
        END";
}
