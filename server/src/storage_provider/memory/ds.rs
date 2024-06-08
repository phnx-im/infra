// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Mutex,
};

use async_trait::async_trait;
use mls_assist::openmls::prelude::GroupId;
use phnxbackend::ds::{group_state::EncryptedDsGroupState, DsStorageProvider, LoadState};
use phnxtypes::{identifiers::Fqdn, time::TimeStamp};

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
    Taken(EncryptedDsGroupState),
}

/// A storage provider for the DS using PostgreSQL.
pub struct MemoryDsStorage {
    groups: Mutex<HashMap<GroupId, StorageState>>,
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
impl DsStorageProvider for MemoryDsStorage {
    type StorageError = MemoryDsStorageError;

    async fn load_group_state(&self, group_id: &GroupId) -> Result<LoadState, Self::StorageError> {
        match self.groups.try_lock() {
            Ok(groups) => match groups.get(group_id) {
                Some(StorageState::Taken(encrypted_group_state)) => {
                    if encrypted_group_state.last_used.has_expired(90) {
                        Ok(LoadState::Expired)
                    } else {
                        Ok(LoadState::Success(encrypted_group_state.clone()))
                    }
                }
                Some(StorageState::Reserved(timestamp)) => {
                    Ok(LoadState::Reserved(*timestamp))
                }
                None => Ok(LoadState::NotFound),
            },
            Err(_) => Err(MemoryDsStorageError::MemoryStoreError),
        }
    }

    async fn save_group_state(
        &self,
        group_id: &GroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<(), MemoryDsStorageError> {
        if let Ok(mut groups) = self.groups.try_lock() {
            groups.insert(group_id.clone(), StorageState::Taken(encrypted_group_state));
            Ok(())
        } else {
            Err(MemoryDsStorageError::MemoryStoreError)
        }
    }

    async fn reserve_group_id(&self, group_id: &GroupId) -> Result<bool, Self::StorageError> {
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
