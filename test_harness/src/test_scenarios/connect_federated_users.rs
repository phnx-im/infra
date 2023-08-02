// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use once_cell::sync::Lazy;

use crate::{
    docker::{wait_until_servers_are_up, DockerTestBed},
    test_scenarios::{TEST_DOMAIN_ONE, TEST_DOMAIN_TWO},
    utils::setup::TestBed,
    TRACING,
};

pub const CONNECT_FEDERATED_USERS_SCENARIO_NAME: &str = "connect_federated_users";

/// This function spawns the containers required to test a connection between
/// two federated users.
pub async fn connect_federated_users_scenario() {
    Lazy::force(&TRACING);
    tracing::info!("Running federation test scenario");

    let mut docker = DockerTestBed::new(&[TEST_DOMAIN_ONE, TEST_DOMAIN_TWO]).await;

    docker.start_test(CONNECT_FEDERATED_USERS_SCENARIO_NAME)
}

/// This function is meant to be called from the test container. It registers
/// two clients, one on each test server and makes them perform the requests
/// required to connect the two. Before running the test, it waits for the
/// health check to succeed on both servers.
pub async fn connect_federated_users_runner() {
    // Wait until the health check succeeds before running the test container.
    let domains = [TEST_DOMAIN_ONE, TEST_DOMAIN_TWO]
        .iter()
        .map(|&d| d.into())
        .collect::<HashSet<_>>();
    wait_until_servers_are_up(domains).await;

    tracing::info!("Running federation test client");
    let mut test_bed = TestBed::new().await;
    let alice_name = "alice".to_owned() + "@" + TEST_DOMAIN_ONE;
    test_bed.add_user(alice_name.clone()).await;
    let bob_name = "bob".to_owned() + "@" + TEST_DOMAIN_TWO;
    test_bed.add_user(bob_name.clone()).await;
    test_bed.connect_users(alice_name, bob_name).await;
    tracing::info!("Done");
}
