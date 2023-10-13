// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::credentials::keys::InfraCredentialSigningKey;

use super::{
    client_information::{ClientInformationDiff, StagedClientInformationDiff},
    *,
};

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
pub(crate) struct GroupDiff<'a> {
    pub(crate) leaf_signer: Option<InfraCredentialSigningKey>,
    pub(crate) signature_ear_key: Option<SignatureEarKeyWrapperKey>,
    pub(crate) credential_ear_key: Option<ClientCredentialEarKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
    pub(crate) user_auth_key: Option<UserAuthSigningKey>,
    // Changes to the client credentials. `None` denotes a deleted credential at
    // the given index, `Some` denotes an added or updated credential. The
    // vector must be sorted in ascending order of indices.
    client_information: ClientInformationDiff<'a, ClientAuthInfo>,
}

impl<'a> GroupDiff<'a> {
    pub(crate) fn new(group: &'a Group) -> Self {
        Self {
            leaf_signer: None,
            signature_ear_key: None,
            credential_ear_key: None,
            group_state_ear_key: None,
            user_auth_key: None,
            client_information: ClientInformationDiff::new(&group.client_information),
        }
    }

    /// This overrides any previous changes to the client credentials.
    pub(crate) fn remove_client_credential(&mut self, removed_index: LeafNodeIndex) {
        self.client_information
            .remove_client_credential(removed_index)
    }

    pub(crate) fn get(&'a self, index: usize) -> Option<&'a ClientAuthInfo> {
        self.client_information.get(index)
    }

    pub(crate) fn update_client_information(
        &mut self,
        index: usize,
        new_client_information: ClientAuthInfo,
    ) {
        self.client_information
            .update_client_information(index, new_client_information)
    }

    /// Add a client credential in the first free index, or extend the current
    /// list of credentials. This takes into account previous removes.
    pub(crate) fn add_client_information(&mut self, new_client_information: Vec<ClientAuthInfo>) {
        self.client_information
            .add_client_information(new_client_information)
    }

    pub(crate) fn apply_pending_removes(&mut self, staged_commit: &StagedCommit) {
        self.client_information.apply_pending_removes(staged_commit)
    }

    pub(crate) fn stage(self) -> StagedGroupDiff {
        StagedGroupDiff {
            leaf_signer: self.leaf_signer,
            signature_ear_key: self.signature_ear_key,
            credential_ear_key: self.credential_ear_key,
            group_state_ear_key: self.group_state_ear_key,
            user_auth_key: self.user_auth_key,
            client_information: self.client_information.stage(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StagedGroupDiff {
    pub(crate) leaf_signer: Option<InfraCredentialSigningKey>,
    pub(crate) signature_ear_key: Option<SignatureEarKeyWrapperKey>,
    pub(crate) credential_ear_key: Option<ClientCredentialEarKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
    pub(crate) user_auth_key: Option<UserAuthSigningKey>,
    // Changes to the client credentials. `None` denotes a deleted credential at
    // the given index, `Some` denotes an added or updated credential. The
    // vector must be sorted in ascending order of indices.
    pub(super) client_information: StagedClientInformationDiff<ClientAuthInfo>,
}
