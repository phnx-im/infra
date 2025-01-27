// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    credentials::keys::{CredentialCreationError, PseudonymousCredentialSigningKey},
    crypto::ear::keys::IdentityLinkKey,
};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::*;

impl Storable for LeafKeys {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS leaf_keys (
            verifying_key BLOB PRIMARY KEY,
            leaf_signing_key BLOB NOT NULL,
            identity_link_key BLOB NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let verifying_key_bytes: Vec<u8> = row.get(0)?;
        let verifying_key = SignaturePublicKey::from(verifying_key_bytes);
        Ok(Self {
            verifying_key,
            leaf_signing_key: row.get(1)?,
            identity_link_key: row.get(2)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LeafKeys {
    verifying_key: SignaturePublicKey,
    leaf_signing_key: PseudonymousCredentialSigningKey,
    identity_link_key: IdentityLinkKey,
}

impl LeafKeys {
    pub(crate) fn generate(
        signing_key: &ClientSigningKey,
        connection_key: &ConnectionKey,
    ) -> Result<Self, CredentialCreationError> {
        let (leaf_signing_key, identity_link_key) =
            PseudonymousCredentialSigningKey::generate(signing_key, connection_key)?;
        let keys = Self {
            verifying_key: leaf_signing_key.credential().verifying_key().clone(),
            leaf_signing_key,
            identity_link_key,
        };
        Ok(keys)
    }

    pub(crate) fn credential(&self) -> Result<CredentialWithKey, tls_codec::Error> {
        let credential = CredentialWithKey {
            credential: self.leaf_signing_key.credential().try_into()?,
            signature_key: self.verifying_key.clone(),
        };
        Ok(credential)
    }

    pub(crate) fn identity_link_key(&self) -> &IdentityLinkKey {
        &self.identity_link_key
    }

    pub(crate) fn into_leaf_signer(self) -> PseudonymousCredentialSigningKey {
        self.leaf_signing_key
    }

    pub(crate) fn into_parts(self) -> (PseudonymousCredentialSigningKey, IdentityLinkKey) {
        (self.leaf_signing_key, self.identity_link_key)
    }
}

impl LeafKeys {
    pub(crate) fn load(
        connection: &Connection,
        verifying_key: &SignaturePublicKey,
    ) -> Result<Option<LeafKeys>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT verifying_key, leaf_signing_key, identity_link_key FROM leaf_keys WHERE verifying_key = ?",
        )?;
        stmt.query_row(params![verifying_key.as_slice()], Self::from_row)
            .optional()
    }

    pub(crate) fn delete(
        connection: &Connection,
        verifying_key: &SignaturePublicKey,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = connection.prepare("DELETE FROM leaf_keys WHERE verifying_key = ?")?;
        stmt.execute(params![verifying_key.as_slice()])?;
        Ok(())
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO leaf_keys (verifying_key, leaf_signing_key, identity_link_key) VALUES (?, ?, ?)",
            params![
                self.verifying_key.as_slice(),
                self.leaf_signing_key,
                self.identity_link_key
            ],
        )?;
        Ok(())
    }
}
