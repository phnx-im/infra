// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::AssistedMessageOut,
    openmls::{
        prelude::{
            GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn, group_info::VerifiableGroupInfo,
        },
        treesync::RatchetTree,
    },
};

use crate::{
    crypto::ear::keys::{EncryptedIdentityLinkKey, EncryptedUserProfileKey},
    identifiers::QsReference,
};

use super::welcome_attribution_info::EncryptedWelcomeAttributionInfo;

pub struct ExternalCommitInfoIn {
    pub verifiable_group_info: VerifiableGroupInfo,
    pub ratchet_tree_in: RatchetTreeIn,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub encrypted_user_profile_keys: Vec<EncryptedUserProfileKey>,
}

#[derive(Debug)]
pub struct WelcomeInfoIn {
    pub ratchet_tree: RatchetTreeIn,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub encrypted_user_profile_keys: Vec<EncryptedUserProfileKey>,
}

#[derive(Debug)]
pub struct CreateGroupParamsOut {
    pub group_id: GroupId,
    pub ratchet_tree: RatchetTree,
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub encrypted_user_profile_key: EncryptedUserProfileKey,
    pub creator_client_reference: QsReference,
    pub group_info: MlsMessageOut,
}

#[derive(Debug)]
pub struct AddUsersInfoOut {
    pub welcome: MlsMessageOut,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
}

#[derive(Debug)]
pub struct GroupOperationParamsOut {
    pub commit: AssistedMessageOut,
    pub add_users_info_option: Option<AddUsersInfoOut>,
}

#[derive(Debug)]
pub struct UpdateParamsOut {
    pub commit: AssistedMessageOut,
}

#[derive(Debug)]
pub struct JoinConnectionGroupParamsOut {
    pub external_commit: AssistedMessageOut,
    pub qs_client_reference: QsReference,
}

#[derive(Debug)]
pub struct ResyncParamsOut {
    pub external_commit: AssistedMessageOut,
    pub sender: LeafNodeIndex,
}

#[derive(Debug)]
pub struct SelfRemoveParamsOut {
    pub remove_proposal: AssistedMessageOut,
}

#[derive(Debug)]
pub struct SendMessageParamsOut {
    pub message: AssistedMessageOut,
    pub sender: LeafNodeIndex,
}

#[derive(Debug)]
pub struct DeleteGroupParamsOut {
    pub commit: AssistedMessageOut,
}
