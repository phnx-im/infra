// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
#[derive(Debug)]
pub(crate) struct GroupDiff {
    pub(crate) leaf_signer: Option<InfraCredentialSigningKey>,
    pub(crate) signature_ear_key: Option<SignatureEarKey>,
    pub(crate) credential_ear_key: Option<ClientCredentialEarKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
    pub(crate) user_auth_key: Option<UserAuthSigningKey>,
    // Changes to the client credentials. `None` denotes a deleted credential at
    // the given index, `Some` denotes an added or updated credential. The
    // vector must be sorted in ascending order of indices.
    pub(crate) client_credentials: HashMap<usize, Option<ClientCredential>>,
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
            client_credentials: HashMap::new(),
            new_number_of_leaves: group.client_credentials.len(),
        }
    }

    /// This overrides any previous changes to the client credentials.
    pub(crate) fn remove_client_credential(&mut self, removed_index: LeafNodeIndex) {
        self.client_credentials.insert(removed_index.usize(), None);
    }

    pub(crate) fn credential<'a>(
        &'a self,
        index: usize,
        existing_client_credentials: &'a [Option<ClientCredential>],
    ) -> Option<&'a ClientCredential> {
        if let Some(Some(credential)) = self.client_credentials.get(&index) {
            Some(credential)
        } else {
            existing_client_credentials
                .get(index)
                .and_then(|c| c.as_ref())
        }
    }

    /// Add a client credential in the first free index, or extend the current
    /// list of credentials. This takes into account previous removes.
    pub(crate) fn add_client_credentials(
        &mut self,
        existing_client_credentials: &[Option<ClientCredential>],
        new_client_credential: ClientCredential,
    ) {
        for index in 0..self.new_number_of_leaves {
            if self
                .credential(index, existing_client_credentials)
                .is_none()
            {
                self.client_credentials
                    .insert(index, Some(new_client_credential));
                return;
            }
        }
        // If we're still here, we have not found a free index yet and we have
        // to extend the vector of credentials.
        self.client_credentials
            .insert(self.new_number_of_leaves, Some(new_client_credential));
        self.new_number_of_leaves += 1;
    }
}
