// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{crypto::signatures::keys::QsVerifyingKey, identifiers::Fqdn};

use crate::utils::persistence::PersistableStruct;

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

impl Deref for QualifiedQsVerifyingKey {
    type Target = QsVerifyingKey;

    fn deref(&self) -> &Self::Target {
        &self.qs_verifying_key
    }
}

pub(crate) type PersistableQsVerifyingKey<'a> = PersistableStruct<'a, QualifiedQsVerifyingKey>;

impl Persistable for QualifiedQsVerifyingKey {
    type Key = Fqdn;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::QsVerifyingKey;

    fn key(&self) -> &Self::Key {
        &self.domain
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.domain
    }
}
