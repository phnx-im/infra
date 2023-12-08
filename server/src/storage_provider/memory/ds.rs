// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Mutex,
};

use async_trait::async_trait;
use mls_assist::{group, openmls::prelude::GroupId};
use phnxbackend::ds::{
    group_state::EncryptedDsGroupState, DsGenericStorageProvider, GenericLoadState,
};
use phnxtypes::{identifiers::Fqdn, time::TimeStamp};
use sqlx::types::Uuid;

#[derive(Debug)]
pub enum MemoryDsStorageError {
    GroupAlreadyExists,
    MemoryStoreError,
}

impl Display for MemoryDsStorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemoryDsStorageError")
    }
}

enum StorageState {
    Reserved(TimeStamp),
    Taken(Vec<u8>),
}

/// A storage provider for the DS using PostgreSQL.
pub struct MemoryDsStorage {
    groups: Mutex<HashMap<Uuid, StorageState>>,
    own_domain: Fqdn,
}

impl MemoryDsStorage {
    pub fn new(own_domain: Fqdn) -> Self {
        Self {
            groups: Mutex::new(HashMap::new()),
            own_domain,
        }
    }
}

#[async_trait]
impl<GroupCiphertext, GroupCiphertextId>
    DsGenericStorageProvider<GroupCiphertext, GroupCiphertextId> for MemoryDsStorage
where
    GroupCiphertext: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Sync + Send,
    GroupCiphertextId: AsRef<[u8; 16]> + std::fmt::Debug + Sync + Send,
{
    type StorageError = MemoryDsStorageError;

    async fn load_group_state(
        &self,
        group_id: &GroupCiphertextId,
    ) -> Result<GenericLoadState<GroupCiphertext>, Self::StorageError> {
        let group_id = Uuid::from_bytes_ref(group_id.as_ref());
        match self.groups.try_lock() {
            Ok(groups) => match groups.get(group_id) {
                Some(StorageState::Taken(encrypted_group_state)) => {
                    let group_ciphertext = serde_json::from_slice(encrypted_group_state)
                        .map_err(|_| MemoryDsStorageError::MemoryStoreError)?;
                    Ok(GenericLoadState::Success(group_ciphertext))
                }
                Some(StorageState::Reserved(timestamp)) => {
                    Ok(GenericLoadState::Reserved(timestamp.clone()))
                }
                None => Ok(GenericLoadState::NotFound),
            },
            Err(_) => Err(MemoryDsStorageError::MemoryStoreError),
        }
    }

    async fn save_group_state(
        &self,
        group_id: &GroupCiphertextId,
        encrypted_group_state: GroupCiphertext,
    ) -> Result<(), MemoryDsStorageError>
    where
        GroupCiphertext: 'async_trait,
    {
        let ciphertext_bytes = serde_json::to_vec(&encrypted_group_state)
            .map_err(|_| MemoryDsStorageError::MemoryStoreError)?;
        let group_id = Uuid::from_bytes_ref(group_id.as_ref());
        if let Ok(mut groups) = self.groups.try_lock() {
            groups.insert(group_id.clone(), StorageState::Taken(ciphertext_bytes));
            Ok(())
        } else {
            Err(MemoryDsStorageError::MemoryStoreError)
        }
    }

    async fn reserve_group_id(
        &self,
        group_id: &GroupCiphertextId,
    ) -> Result<bool, Self::StorageError> {
        let group_id = Uuid::from_bytes_ref(group_id.as_ref());
        if let Ok(mut groups) = self.groups.try_lock() {
            match groups.insert(group_id.clone(), StorageState::Reserved(TimeStamp::now())) {
                Some(_) => Ok(false),
                None => Ok(true),
            }
        } else {
            Err(MemoryDsStorageError::MemoryStoreError)
        }
    }

    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
    }
}
