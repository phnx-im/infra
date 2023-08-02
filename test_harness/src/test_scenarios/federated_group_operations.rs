// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use once_cell::sync::Lazy;

use crate::{
    docker::{wait_until_servers_are_up, DockerTestBed},
    test_scenarios::{TEST_DOMAIN_ONE, TEST_DOMAIN_THREE, TEST_DOMAIN_TWO},
    utils::setup::TestBed,
    TRACING,
};

pub const FEDERATED_GROUP_OPERATIONS_SCENARIO_NAME: &str = "federated_group_operations";

/// This function spawns the containers required to test a connection between
/// two federated users.
pub async fn federated_group_operations_scenario() {
    Lazy::force(&TRACING);
    tracing::info!("Running federation test scenario");

    let mut docker =
        DockerTestBed::new(&[TEST_DOMAIN_ONE, TEST_DOMAIN_TWO, TEST_DOMAIN_THREE]).await;

    docker.start_test(FEDERATED_GROUP_OPERATIONS_SCENARIO_NAME)
}

pub async fn federated_group_operations_runner() {
    // Wait until the health check succeeds before running the test container.
    let domains = [TEST_DOMAIN_ONE, TEST_DOMAIN_TWO, TEST_DOMAIN_THREE]
        .iter()
        .map(|&d| d.into())
        .collect::<HashSet<_>>();
    wait_until_servers_are_up(domains).await;

    tracing::info!("Running federation test client");
    // Create three users.
    let mut test_bed = TestBed::new().await;
    let alice_name = "alice".to_owned() + "@" + TEST_DOMAIN_ONE;
    test_bed.add_user(alice_name.clone()).await;
    let bob_name = "bob".to_owned() + "@" + TEST_DOMAIN_TWO;
    test_bed.add_user(bob_name.clone()).await;
    let charlie_name = "charlie".to_owned() + "@" + TEST_DOMAIN_THREE;
    test_bed.add_user(charlie_name.clone()).await;

    // Connect alice and bob.
    test_bed.connect_users(&alice_name, &bob_name).await;
    // Connect bob and charlie.
    test_bed.connect_users(&bob_name, &charlie_name).await;

    // Have alice create a group.
    let conversation_id = test_bed.create_group(&alice_name).await;

    // Have alice invite bob
    test_bed
        .invite_to_group(conversation_id, &alice_name, vec![&bob_name])
        .await;

    // Have bob invite charlie
    test_bed
        .invite_to_group(conversation_id, &bob_name, vec![&charlie_name])
        .await;

    // Have charlie remove alice
    test_bed
        .remove_from_group(conversation_id, &charlie_name, vec![&alice_name])
        .await;

    // Have bob leave the group
    test_bed.leave_group(conversation_id, &bob_name).await;

    tracing::info!("Done");
}
