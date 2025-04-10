// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::credentials::keys::PseudonymousCredentialSigningKey;

use super::*;

/// A struct that contains differences in group data when creating a commit.
/// The diff of a group should be merged when the pending commit of the
/// underlying MLS group is merged.
pub(crate) struct GroupDiff {
    pub(crate) leaf_signer: Option<PseudonymousCredentialSigningKey>,
    pub(crate) identity_link_key: Option<IdentityLinkWrapperKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
}

impl GroupDiff {
    pub(crate) fn new() -> Self {
        Self {
            leaf_signer: None,
            identity_link_key: None,
            group_state_ear_key: None,
        }
    }

    pub(crate) fn stage(self) -> StagedGroupDiff {
        StagedGroupDiff {
            leaf_signer: self.leaf_signer,
            identity_link_key: self.identity_link_key,
            group_state_ear_key: self.group_state_ear_key,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StagedGroupDiff {
    pub(crate) leaf_signer: Option<PseudonymousCredentialSigningKey>,
    pub(crate) identity_link_key: Option<IdentityLinkWrapperKey>,
    pub(crate) group_state_ear_key: Option<GroupStateEarKey>,
}

#[cfg(test)]
mod test {
    use std::sync::LazyLock;

    use phnxtypes::codec::PhnxCodec;

    use super::*;

    static STAGED_GROUP_DIFF: LazyLock<StagedGroupDiff> = LazyLock::new(|| {
        // Note: It is hard to construct a valid `StagedGroupDiff` deterministically.
        // Instead, we construct it from a JSON value.
        let value = serde_json::json!({
            "leaf_signer": {
                "signing_key": {
                    "signing_key": [1, 2, 3],
                    "verifying_key": [4, 5, 6],
                },
                "credential": {
                    "tbs": {
                        "identity": [7, 8, 9],
                        "expiration_data": {
                            "not_before": 0,
                            "not_after": 1,
                        },
                        "signature_scheme": "ED25519",
                        "verifying_key": {
                            "value": {
                                "vec": [10, 11, 12],
                            },
                        },
                    },
                    "identity_link_ctxt": {
                        "encrypted_signature": {
                            "ciphertext": {
                                "ciphertext": [13, 14, 15],
                                "nonce": b"nonce_1_____",
                            },
                        },
                        "encrypted_client_credential": {
                            "encrypted_client_credential": {
                                "ciphertext": [16, 17, 18],
                                "nonce": b"nonce_2_____",
                            },
                        }
                    },
                },
            },
            "identity_link_key": {
                "key": {
                    "secret": b"identity_link_key_32_bytes______",
                },
            },
            "group_state_ear_key": {
                "key": {
                    "secret": b"group_state_ear_key_32_bytes____",
                },
            },
        });
        serde_json::from_value(value).unwrap()
    });

    #[test]
    fn test_group_staged_diff_serde_codec() {
        insta::assert_binary_snapshot!(".cbor", PhnxCodec::to_vec(&*STAGED_GROUP_DIFF).unwrap());
    }

    #[test]
    fn test_group_staged_diff_serde_json() {
        insta::assert_json_snapshot!(&*STAGED_GROUP_DIFF);
    }
}
