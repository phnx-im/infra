// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::credentials::keys::InfraCredentialSigningKey;

use super::*;

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
pub(crate) struct GroupDiff {
    pub(crate) leaf_signer: Option<InfraCredentialSigningKey>,
    pub(crate) signature_ear_key: Option<SignatureEarKeyWrapperKey>,
    pub(crate) credential_ear_key: Option<ClientCredentialEarKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
    pub(crate) user_auth_key: Option<UserAuthSigningKey>,
}

impl GroupDiff {
    pub(crate) fn new() -> Self {
        Self {
            leaf_signer: None,
            signature_ear_key: None,
            credential_ear_key: None,
            group_state_ear_key: None,
            user_auth_key: None,
        }
    }

    pub(crate) fn stage(self) -> StagedGroupDiff {
        StagedGroupDiff {
            leaf_signer: self.leaf_signer,
            signature_ear_key: self.signature_ear_key,
            credential_ear_key: self.credential_ear_key,
            group_state_ear_key: self.group_state_ear_key,
            user_auth_key: self.user_auth_key,
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
}
