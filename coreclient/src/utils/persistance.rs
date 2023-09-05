// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::auth_service::AsClientId;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tls_codec::Serialize as TlsSerializeTrait;
use turbosql::{execute, select, Turbosql};

#[derive(Turbosql)]
struct TurboData {
    rowid: Option<i64>,
    data_type: Option<Vec<u8>>,
    client_id: Option<Vec<u8>>,
    key: Option<Vec<u8>>,
    secondary_key: Option<Vec<u8>>,
    value: Option<Vec<u8>>,
}

impl TurboData {
    fn from_persistable<T: Persistable>(value: &T) -> Result<Self, turbosql::Error> {
        let client_id_bytes = value.own_client_id_bytes();
        let key_bytes = serde_json::to_vec(&value.key())?;
        let secondary_key_bytes = serde_json::to_vec(&value.secondary_key())?;
        let data_type_bytes = serde_json::to_vec(&T::DATA_TYPE)?;
        let value_bytes = serde_json::to_vec(&value)?;
        Ok(Self {
            rowid: value.rowid(),
            data_type: Some(data_type_bytes),
            client_id: Some(client_id_bytes),
            key: Some(key_bytes),
            value: Some(value_bytes),
            secondary_key: Some(secondary_key_bytes),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum DataType {
    Contact,
    Conversation,
    Group,
    Message,
    AsCredential,
}

pub(crate) trait Persistable: Serialize + DeserializeOwned {
    type Key: Serialize + std::fmt::Debug;
    type SecondaryKey: Serialize + std::fmt::Debug;

    const DATA_TYPE: DataType;

    fn own_client_id_bytes(&self) -> Vec<u8>;

    fn rowid(&self) -> Option<i64>;

    fn key(&self) -> &Self::Key;

    fn secondary_key(&self) -> &Self::SecondaryKey;

    fn load(own_client_id: &AsClientId, key: &Self::Key) -> Result<Self, turbosql::Error> {
        let client_id_bytes = own_client_id.tls_serialize_detached().unwrap();
        let key_bytes = serde_json::to_vec(key)?;
        let data_type_bytes = serde_json::to_vec(&Self::DATA_TYPE)?;
        let turbo_data = select!(TurboData "WHERE client_id = " client_id_bytes " AND key = " key_bytes " AND data_type = " data_type_bytes)?;
        let value = turbo_data
            .value
            .ok_or(turbosql::Error::OtherError("Could not load value from DB."))?;
        Ok(serde_json::from_slice(&value)?)
    }

    fn load_secondary(
        own_client_id: &AsClientId,
        secondary_key: &Self::SecondaryKey,
    ) -> Result<Self, turbosql::Error> {
        let client_id_bytes = own_client_id.tls_serialize_detached().unwrap();
        let key_bytes = serde_json::to_vec(secondary_key)?;
        let data_type_bytes = serde_json::to_vec(&Self::DATA_TYPE)?;
        let turbo_data = select!(TurboData "WHERE client_id = " client_id_bytes " AND secondary_key = " key_bytes " AND data_type = " data_type_bytes)?;
        let value = turbo_data
            .value
            .ok_or(turbosql::Error::OtherError("Could not load value from DB."))?;
        Ok(serde_json::from_slice(&value)?)
    }

    fn load_multiple_secondary(
        own_client_id: &AsClientId,
        secondary_key: &Self::SecondaryKey,
    ) -> Result<Vec<Self>, turbosql::Error> {
        let client_id_bytes = own_client_id.tls_serialize_detached().unwrap();
        let data_type_bytes = serde_json::to_vec(&Self::DATA_TYPE)?;
        let key_bytes = serde_json::to_vec(secondary_key)?;
        let values = select!(Vec<TurboData> "WHERE client_id = " client_id_bytes " AND secondary_key = " key_bytes " AND data_type = " data_type_bytes)?;
        let mapped_values = values
            .into_iter()
            .map(|turbo_data| {
                let value = turbo_data
                    .value
                    .ok_or(turbosql::Error::OtherError("Could not load value from DB."))?;
                Ok(serde_json::from_slice(&value)?)
            })
            .collect::<Result<Vec<_>, turbosql::Error>>()?;
        Ok(mapped_values)
    }

    fn load_all(own_client_id: &AsClientId) -> Result<Vec<Self>, turbosql::Error> {
        let client_id_bytes = own_client_id.tls_serialize_detached().unwrap();
        let data_type_bytes = serde_json::to_vec(&Self::DATA_TYPE)?;
        let values = select!(Vec<TurboData> "WHERE client_id = " client_id_bytes " AND data_type = " data_type_bytes)?;
        let mapped_values = values
            .into_iter()
            .map(|turbo_data| {
                let value = turbo_data
                    .value
                    .ok_or(turbosql::Error::OtherError("Could not load value from DB."))?;
                Ok(serde_json::from_slice(&value)?)
            })
            .collect::<Result<Vec<_>, turbosql::Error>>()?;
        Ok(mapped_values)
    }

    fn persist(&self) -> Result<(), turbosql::Error> {
        let turbo_data = TurboData::from_persistable(self)?;
        if self.rowid().is_some() {
            turbo_data.update()?;
        } else {
            // We can unwrap these, as they are both set in the constructor.
            let client_id_bytes = turbo_data.client_id.as_ref().unwrap();
            let key_bytes = turbo_data.key.as_ref().unwrap();
            let data_type_bytes = serde_json::to_vec(&Self::DATA_TYPE)?;
            // Check if a contact with this ID already exists.
            if let Ok(old_data) = select!(TurboData "WHERE client_id = " client_id_bytes " AND key = " key_bytes " AND data_type = " data_type_bytes)
            {
                // If it exists, delete it from the DB. (We could probably just
                // read out the rowid of the existing contact and set it for the
                // new contact, but this does the trick.)
                execute!("DELETE FROM turbodata WHERE rowid = " old_data.rowid.unwrap())?;
            }
            // Insert the new data into the DB.
            turbo_data.insert()?;
        }
        Ok(())
    }

    fn _purge(&self) -> Result<(), turbosql::Error> {
        let turbo_data = TurboData::from_persistable(self)?;
        let rowid = turbo_data.rowid.ok_or(turbosql::Error::OtherError(
            "Cannot purge data without rowid.",
        ))?;
        execute!("DELETE FROM turbodata WHERE rowid = " rowid)?;
        Ok(())
    }
}
