// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

pub(crate) struct LeafKeyStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for LeafKeyStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> LeafKeyStore<'a> {
    pub(crate) fn get(
        &self,
        verifying_key: &SignaturePublicKey,
    ) -> Result<Option<PersistableLeafKeys>, PersistenceError> {
        let verifying_key_str = hex::encode(verifying_key.as_slice());
        PersistableLeafKeys::load_one(self.db_connection, Some(&verifying_key_str), None)
    }

    pub(crate) fn generate(&self, signing_key: &ClientSigningKey) -> Result<PersistableLeafKeys> {
        let signature_ear_key = SignatureEarKey::random()?;
        let leaf_signing_key = InfraCredentialSigningKey::generate(signing_key, &signature_ear_key);
        let keys = PersistableLeafKeys::from_connection_and_payload(
            self.db_connection,
            (leaf_signing_key, signature_ear_key),
        );
        keys.persist()?;
        Ok(keys)
    }

    pub(crate) fn delete(
        &self,
        verifying_key: &SignaturePublicKey,
    ) -> Result<(), PersistenceError> {
        let verifying_key_str = hex::encode(verifying_key.as_slice());
        PersistableLeafKeys::purge_key(self.db_connection, &verifying_key_str)
    }
}

pub(crate) struct PersistableLeafKeys<'a> {
    connection: &'a Connection,
    verifying_key_str: String,
    payload: (InfraCredentialSigningKey, SignatureEarKey),
}

impl PersistableLeafKeys<'_> {
    pub(crate) fn leaf_signing_key(&self) -> &InfraCredentialSigningKey {
        &self.payload.0
    }

    pub(crate) fn signature_ear_key(&self) -> &SignatureEarKey {
        &self.payload.1
    }
}

impl<'a> Persistable<'a> for PersistableLeafKeys<'a> {
    type Key = String;

    type SecondaryKey = String;

    const DATA_TYPE: DataType = DataType::LeafKeys;

    fn key(&self) -> &Self::Key {
        &self.verifying_key_str
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.verifying_key_str
    }

    type Payload = (InfraCredentialSigningKey, SignatureEarKey);

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        let verifying_key_str = hex::encode(payload.0.credential().verifying_key().as_slice());
        Self {
            connection: conn,
            verifying_key_str,
            payload,
        }
    }
}
