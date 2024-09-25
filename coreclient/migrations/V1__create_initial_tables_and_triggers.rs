// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    clients::{own_client_info::OwnClientInfo, store::UserCreationState},
    contacts::{Contact, PartialContact},
    conversations::{messages::ConversationMessage, Conversation},
    groups::{
        client_auth_info::{GroupMembership, StorableClientCredential},
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
        as_credentials::AsCredentials, leaf_keys::LeafKeys,
        qs_verifying_keys::StorableQsVerifyingKey, queue_ratchets::StorableAsQueueRatchet,
    },
    user_profiles::UserProfile,
    utils::persistence::Storable,
};

const GROUP_MEMBERSHIP_TRIGGER: &str = 
    "CREATE TRIGGER IF NOT EXISTS delete_orphaned_data 
        AFTER DELETE ON group_membership
        FOR EACH ROW
        BEGIN
            -- Delete client credentials if they are not our own and not used in any group.
            DELETE FROM client_credentials
            WHERE fingerprint = OLD.client_credential_fingerprint AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_credential_fingerprint = OLD.client_credential_fingerprint
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_client_uuid = OLD.client_uuid
            );

            -- Delete user profiles of users that are not in any group and that are not our own.
            DELETE FROM users
            WHERE user_name = OLD.user_name AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE user_name = OLD.user_name
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_user_name = OLD.user_name
            );
        END;";
const PARTIAL_CONTACT_INSERT_TRIGGER: &str = 
    "DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_insert;

    CREATE TRIGGER no_partial_contact_overlap_on_insert
    BEFORE INSERT ON contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM partial_contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t insert Contact: There already exists a partial contact with this user_name')
        END;
    END;";
const PARTIAL_CONTACT_UPDATE_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_update;

    CREATE TRIGGER no_partial_contact_overlap_on_update
    BEFORE UPDATE ON contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM partial_contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t update Contact: There already exists a partial contact with this user_name')
        END;
    END;";
const CONTACT_INSERT_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_contact_overlap_on_insert;

    CREATE TRIGGER no_contact_overlap_on_insert
    BEFORE INSERT ON partial_contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t insert PartialContact: There already exists a contact with this user_name')
        END;
    END;";
const CONTACT_UPDATE_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS no_contact_overlap_on_update;

    CREATE TRIGGER no_contact_overlap_on_update
    BEFORE UPDATE ON partial_contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t update PartialContact: There already exists a contact with this user_name')
        END;
    END;";

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
        <StorableQsVerifyingKey as Storable>::CREATE_TABLE_STATEMENT,
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
