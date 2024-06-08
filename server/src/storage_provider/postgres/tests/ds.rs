// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use core::panic;

use mls_assist::openmls::group::GroupId;
use phnxbackend::ds::{group_state::EncryptedDsGroupState, DsStorageProvider, LoadState};
use phnxtypes::{
    crypto::ear::Ciphertext,
    identifiers::{Fqdn, QualifiedGroupId},
};
use sqlx::types::Uuid;
use tls_codec::Serialize;

use crate::{configurations::get_configuration, storage_provider::postgres::ds::PostgresDsStorage};

async fn initialize_test_provider() -> PostgresDsStorage {
    let mut configuration = get_configuration("../server/").expect("Could not load configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let own_domain = Fqdn::try_from("example.com").unwrap();
    PostgresDsStorage::new(&configuration.database, own_domain.clone())
        .await
        .unwrap()
}

#[actix_rt::test]
async fn reserve_group_id() {
    let storage_provider = initialize_test_provider().await;

    // Sample a random group id and reserve it
    let group_uuid = *Uuid::new_v4().as_bytes();
    let qgid = QualifiedGroupId {
        group_id: group_uuid,
        owning_domain: Fqdn::try_from("example.com").unwrap(),
    };
    let group_id = GroupId::from_slice(&qgid.tls_serialize_detached().unwrap());
    let was_reserved = storage_provider
        .reserve_group_id(&group_id)
        .await
        .expect("Error reserving group id.");
    assert!(was_reserved);

    // Try to reserve the same group id again
    let was_reserved_again = storage_provider
        .reserve_group_id(&group_id)
        .await
        .expect("Error reserving group id.");

    // This should return false
    assert!(!was_reserved_again);
}

#[actix_rt::test]
async fn group_state_lifecycle() {
    let storage_provider = initialize_test_provider().await;

    let dummy_ciphertext = Ciphertext::dummy();
    let test_state: EncryptedDsGroupState = dummy_ciphertext.into();

    // Create/store a dummy group state
    let qgid_bytes = QualifiedGroupId {
        group_id: Uuid::new_v4().into_bytes(),
        owning_domain: Fqdn::try_from("example.com").unwrap(),
    }
    .tls_serialize_detached()
    .unwrap();
    let group_id = GroupId::from_slice(&qgid_bytes);
    let was_reserved = storage_provider
        .reserve_group_id(&group_id)
        .await
        .expect("Error reserving group id.");
    assert!(was_reserved);

    // Save the group state
    storage_provider
        .save_group_state(&group_id, test_state.clone())
        .await
        .expect("Error saving group state.");

    // Load the group state again
    let loaded_group_state = storage_provider
        .load_group_state(&group_id)
        .await
        .expect("Error loading group state.");

    if let LoadState::Success(loaded_group_state) = loaded_group_state {
        assert_eq!(loaded_group_state.ciphertext, test_state.ciphertext);
    } else {
        panic!("Error loading group state.");
    }

    // Try to reserve the group id of the created group state
    let successfully_reserved = storage_provider
        .reserve_group_id(&group_id)
        .await
        .expect("Error reserving group id.");

    // This should return false
    assert!(!successfully_reserved);

    // Update that group state.
    let changed_dummy_ciphertext = Ciphertext::default();
    let changed_test_state: EncryptedDsGroupState = changed_dummy_ciphertext.into();

    storage_provider
        .save_group_state(&group_id, changed_test_state)
        .await
        .expect("Error saving group state.");
}
