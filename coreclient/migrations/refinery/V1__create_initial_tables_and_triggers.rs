// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    clients::{own_client_info::OwnClientInfo, store::UserCreationState},
    contacts::{
        persistence::{
            CONTACT_INSERT_TRIGGER, CONTACT_UPDATE_TRIGGER, PARTIAL_CONTACT_INSERT_TRIGGER,
            PARTIAL_CONTACT_UPDATE_TRIGGER,
        },
        Contact, PartialContact,
    },
    conversations::{messages::ConversationMessage, Conversation},
    groups::{
        client_auth_info::{
            persistence::GROUP_MEMBERSHIP_TRIGGER, GroupMembership, StorableClientCredential,
        },
        openmls_provider::{
            encryption_key_pairs::StorableEncryptionKeyPair,
            epoch_key_pairs::StorableEpochKeyPairs, group_data::StorableGroupData,
            key_packages::StorableKeyPackage, own_leaf_nodes::StorableLeafNode,
            proposals::StorableProposal, psks::StorablePskBundle,
            signature_key_pairs::StorableSignatureKeyPairs,
        },
        persistence::StorableGroup,
    },
    key_stores::{
        as_credentials::AsCredentials, leaf_keys::LeafKeys, queue_ratchets::StorableAsQueueRatchet,
    },
    user_profiles::UserProfile,
    utils::persistence::Storable,
};

pub fn migration() -> String {
    [
        <UserCreationState as Storable>::CREATE_TABLE_STATEMENT,
        <OwnClientInfo as Storable>::CREATE_TABLE_STATEMENT,
        <UserProfile as Storable>::CREATE_TABLE_STATEMENT,
        <StorableGroup as Storable>::CREATE_TABLE_STATEMENT,
        <StorableClientCredential as Storable>::CREATE_TABLE_STATEMENT,
        <GroupMembership as Storable>::CREATE_TABLE_STATEMENT,
        <Contact as Storable>::CREATE_TABLE_STATEMENT,
        <PartialContact as Storable>::CREATE_TABLE_STATEMENT,
        <Conversation as Storable>::CREATE_TABLE_STATEMENT,
        <ConversationMessage as Storable>::CREATE_TABLE_STATEMENT,
        <StorableLeafNode<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableSignatureKeyPairs<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableEpochKeyPairs<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableEncryptionKeyPair<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableGroupData<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableKeyPackage<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableProposal<u8, u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorablePskBundle<u8> as Storable>::CREATE_TABLE_STATEMENT,
        <StorableAsQueueRatchet as Storable>::CREATE_TABLE_STATEMENT,
        <AsCredentials as Storable>::CREATE_TABLE_STATEMENT,
        <LeafKeys as Storable>::CREATE_TABLE_STATEMENT,
        GROUP_MEMBERSHIP_TRIGGER,
        PARTIAL_CONTACT_INSERT_TRIGGER,
        PARTIAL_CONTACT_UPDATE_TRIGGER,
        CONTACT_INSERT_TRIGGER,
        CONTACT_UPDATE_TRIGGER,
    ]
    .join("\n")
}
