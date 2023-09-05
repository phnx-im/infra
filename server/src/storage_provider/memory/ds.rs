// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Mutex,
};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use mls_assist::openmls::prelude::GroupId;
use phnxbackend::{
    crypto::EncryptedDsGroupState,
    ds::{group_state::TimeStamp, DsStorageProvider, LoadState},
    qs::Fqdn,
};
use uuid::Uuid;

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

    async fn create_group_state(
        &self,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<GroupId, MemoryDsStorageError> {
        // Generate a new group ID.
        let group_id = GroupId::from_slice(Uuid::new_v4().as_bytes());

        let mut encrypted_group_state = encrypted_group_state;
        encrypted_group_state.last_used = Utc::now();

        if let Ok(mut groups) = self.groups.try_lock() {
            match groups.insert(group_id.clone(), StorageState::Taken(encrypted_group_state)) {
                Some(_) => Err(MemoryDsStorageError::GroupAlreadyExists),
                None => Ok(group_id),
            }
        } else {
            Err(MemoryDsStorageError::MemoryStoreError)
        }
    }

    async fn load_group_state(&self, group_id: &GroupId) -> LoadState {
        match self.groups.try_lock() {
            Ok(groups) => match groups.get(group_id) {
                Some(StorageState::Taken(encrypted_group_state)) => {
                    if Utc::now().signed_duration_since(encrypted_group_state.last_used)
                        < Duration::days(90)
                    {
                        LoadState::Success(encrypted_group_state.clone())
                    } else {
                        LoadState::Expired
                    }
                }
                Some(StorageState::Reserved(timestamp)) => LoadState::Reserved(timestamp.clone()),
                None => LoadState::NotFound,
            },
            Err(_) => LoadState::NotFound,
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
