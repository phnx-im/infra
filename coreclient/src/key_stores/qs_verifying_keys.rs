// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnx_types::{crypto::signatures::keys::QsVerifyingKey, identifiers::Fqdn};

use super::*;

pub(crate) struct QsVerifyingKeyStore<'a> {
    db_connection: &'a Connection,
    api_clients: ApiClients,
}

impl<'a> QsVerifyingKeyStore<'a> {
    pub(crate) fn new(db_connection: &'a Connection, api_clients: ApiClients) -> Self {
        Self {
            db_connection,
            api_clients,
        }
    }

    pub(crate) async fn get(&self, domain: &Fqdn) -> Result<PersistableQsVerifyingKey> {
        if let Some(verifying_key) =
            PersistableQsVerifyingKey::load_one(self.db_connection, Some(domain), None)?
        {
            Ok(verifying_key)
        } else {
            let verifying_key_response = self.api_clients.get(domain)?.qs_verifying_key().await?;
            let verifying_key = PersistableQsVerifyingKey::from_connection_and_payload(
                self.db_connection,
                QualifiedQsVerifyingKey {
                    qs_verifying_key: verifying_key_response.verifying_key,
                    domain: domain.clone(),
                },
            );
            verifying_key.persist()?;
            Ok(verifying_key)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QualifiedQsVerifyingKey {
    qs_verifying_key: QsVerifyingKey,
    domain: Fqdn,
}

pub(crate) struct PersistableQsVerifyingKey<'a> {
    connection: &'a Connection,
    payload: QualifiedQsVerifyingKey,
}

impl Deref for PersistableQsVerifyingKey<'_> {
    type Target = QsVerifyingKey;

    fn deref(&self) -> &Self::Target {
        &self.payload.qs_verifying_key
    }
}

impl<'a> Persistable<'a> for PersistableQsVerifyingKey<'a> {
    type Key = Fqdn;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::QsVerifyingKey;

    fn key(&self) -> &Self::Key {
        &self.payload.domain
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.domain
    }

    type Payload = QualifiedQsVerifyingKey;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}
