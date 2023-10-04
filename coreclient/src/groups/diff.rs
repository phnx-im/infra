// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use phnx_types::credentials::{keys::InfraCredentialSigningKey, ClientCredential};

use crate::utils::{deserialize_btreemap, serialize_hashmap};

use super::*;

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GroupDiff {
    pub(crate) leaf_signer: Option<InfraCredentialSigningKey>,
    pub(crate) signature_ear_key: Option<SignatureEarKeyWrapperKey>,
    pub(crate) credential_ear_key: Option<ClientCredentialEarKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
    pub(crate) user_auth_key: Option<UserAuthSigningKey>,
    // Changes to the client credentials. `None` denotes a deleted credential at
    // the given index, `Some` denotes an added or updated credential. The
    // vector must be sorted in ascending order of indices.
    #[serde(
        serialize_with = "serialize_hashmap",
        deserialize_with = "deserialize_btreemap"
    )]
    pub(crate) client_information: BTreeMap<usize, Option<(ClientCredential, SignatureEarKey)>>,
    pub(crate) new_number_of_leaves: usize,
}

impl GroupDiff {
    pub(crate) fn new(group: &Group) -> Self {
        Self {
            leaf_signer: None,
            signature_ear_key: None,
            credential_ear_key: None,
            group_state_ear_key: None,
            user_auth_key: None,
            client_information: BTreeMap::new(),
            new_number_of_leaves: group
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

    pub(crate) fn client_information<'a>(
        &'a self,
        index: usize,
        existing_client_information: &'a BTreeMap<usize, (ClientCredential, SignatureEarKey)>,
    ) -> Option<&'a (ClientCredential, SignatureEarKey)> {
        if let Some(Some(credential)) = self.client_information.get(&index) {
            Some(credential)
        } else {
            existing_client_information.get(&index)
        }
    }

    /// Add a client credential in the first free index, or extend the current
    /// list of credentials. This takes into account previous removes.
    pub(crate) fn add_client_information(
        &mut self,
        existing_client_information: &BTreeMap<usize, (ClientCredential, SignatureEarKey)>,
        new_client_information: (ClientCredential, SignatureEarKey),
    ) {
        for index in 0..self.new_number_of_leaves {
            if self
                .client_information(index, existing_client_information)
                .is_none()
            {
                self.client_information
                    .insert(index, Some(new_client_information));
                return;
            }
        }
        // If we're still here, we have not found a free index yet and we have
        // to extend the vector of credentials.
        self.client_information
            .insert(self.new_number_of_leaves, Some(new_client_information));
        self.new_number_of_leaves += 1;
    }

    pub(crate) fn apply_pending_removes(&mut self, staged_commit: &StagedCommit) {
        for pending_remove in staged_commit.remove_proposals() {
            self.remove_client_credential(pending_remove.remove_proposal().removed())
        }
    }
}
