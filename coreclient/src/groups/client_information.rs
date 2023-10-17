// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::de::DeserializeOwned;

use crate::utils::{deserialize_btreemap, serialize_hashmap};

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ClientInformation<T: Serialize + DeserializeOwned> {
    #[serde(
        serialize_with = "serialize_hashmap",
        deserialize_with = "deserialize_btreemap"
    )]
    client_information: BTreeMap<usize, T>,
}

impl<T: Serialize + DeserializeOwned> ClientInformation<T> {
    pub(super) fn new(initial_information: T) -> Self {
        Self {
            client_information: [(0, initial_information)].into(),
        }
    }

    pub(super) fn get(&self, index: usize) -> Option<&T> {
        self.client_information.get(&index)
    }

    pub(super) fn merge_diff(&mut self, diff: StagedClientInformationDiff<T>) {
        for (index, client_information_option) in diff.client_information {
            if let Some(client_information) = client_information_option {
                let collision_entry = self.client_information.insert(index, client_information);
                debug_assert!(collision_entry.is_none());
            } else {
                let collision_entry = self.client_information.remove(&index);
                debug_assert!(collision_entry.is_some());
            }
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&usize, &T)> {
        self.client_information.iter()
    }
}

impl ClientInformation<ClientAuthInfo> {
    pub(super) async fn decrypt_and_verify(
        ear_key: &ClientCredentialEarKey,
        wrapper_key: &SignatureEarKeyWrapperKey,
        as_credential_store: &AsCredentialStore<'_>,
        encrypted_client_information: impl IntoIterator<
            Item = Option<(EncryptedClientCredential, EncryptedSignatureEarKey)>,
        >,
    ) -> Result<Self> {
        let mut client_information = BTreeMap::new();
        for (index, client_info_option) in encrypted_client_information.into_iter().enumerate() {
            if let Some((ecc, esek)) = client_info_option {
                let client_auth_info = ClientAuthInfo::decrypt_and_verify(
                    ear_key,
                    wrapper_key,
                    as_credential_store,
                    (ecc, esek),
                )
                .await?;
                client_information.insert(index, client_auth_info);
            }
        }
        Ok(Self { client_information })
    }

    pub(super) fn get_user_name(&self, index: usize) -> Result<UserName> {
        let user_name = self
            .get(index)
            .ok_or(anyhow!("Can't get user name for index {:?}", index))?
            .client_credential()
            .identity()
            .user_name();
        Ok(user_name)
    }
}

pub(super) struct ClientInformationDiff<'a, T: Serialize + DeserializeOwned> {
    original_client_information: &'a ClientInformation<T>,
    client_information: BTreeMap<usize, Option<T>>,
    new_number_of_leaves: usize,
}

impl<'a, T: Serialize + DeserializeOwned> ClientInformationDiff<'a, T> {
    pub(crate) fn new(client_information: &'a ClientInformation<T>) -> Self {
        Self {
            original_client_information: client_information,
            client_information: BTreeMap::new(),
            new_number_of_leaves: client_information
                .client_information
                .last_key_value()
                .map(|(index, _)| index + 1)
                .unwrap_or(0),
        }
    }

    /// This overrides any previous changes to the client credentials.
    pub(crate) fn remove_client_credential(&mut self, removed_index: LeafNodeIndex) {
        self.client_information.insert(removed_index.usize(), None);
    }

    pub(crate) fn get(&'a self, index: usize) -> Option<&'a T> {
        if let Some(Some(credential)) = self.client_information.get(&index) {
            Some(credential)
        } else {
            self.original_client_information.get(index)
        }
    }

    /// Add a client credential in the first free index, or extend the current
    /// list of credentials. This takes into account previous removes.
    pub(crate) fn add_client_information(&mut self, new_client_information: Vec<T>) {
        let mut free_leaves = vec![];
        for index in 0..self.new_number_of_leaves {
            if self.get(index).is_none() {
                free_leaves.push(index);
                //self.client_information
                //    .insert(index, Some(new_client_information));
                //return;
            }
            // Terminate early if we have enough free leaves
            if free_leaves.len() == new_client_information.len() {
                break;
            }
        }
        // If we do not yet have enough free leaves, we extend the tree to the right.
        while free_leaves.len() != new_client_information.len() {
            free_leaves.push(self.new_number_of_leaves);
            self.new_number_of_leaves += 1;
        }
        //Finally, we populate the free leaves.
        for (free_leaf, new_info) in free_leaves.iter().zip(new_client_information.into_iter()) {
            self.client_information.insert(*free_leaf, Some(new_info));
        }
    }

    pub(crate) fn update_client_information(&mut self, index: usize, new_client_information: T) {
        self.client_information
            .insert(index, Some(new_client_information));
    }

    pub(crate) fn apply_pending_removes(&mut self, staged_commit: &StagedCommit) {
        for pending_remove in staged_commit.remove_proposals() {
            self.remove_client_credential(pending_remove.remove_proposal().removed())
        }
    }

    pub(crate) fn stage(self) -> StagedClientInformationDiff<T> {
        StagedClientInformationDiff {
            client_information: self.client_information,
            new_number_of_leaves: self.new_number_of_leaves,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct StagedClientInformationDiff<T: Serialize + DeserializeOwned> {
    #[serde(
        serialize_with = "serialize_hashmap",
        deserialize_with = "deserialize_btreemap"
    )]
    client_information: BTreeMap<usize, Option<T>>,
    new_number_of_leaves: usize,
}

impl<T: Serialize + DeserializeOwned> StagedClientInformationDiff<T> {
    pub(super) fn get(&self, index: usize) -> Option<&Option<T>> {
        self.client_information.get(&index)
    }
}
