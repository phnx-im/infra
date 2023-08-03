// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::qs::Fqdn;

use crate::{docker::wait_until_servers_are_up, utils::setup::TestBed};

pub(super) const NUMBER_OF_SERVERS: usize = 2;

/// This function is meant to be called from the test container. It registers
/// two clients, one on each test server and makes them perform the requests
/// required to connect the two. Before running the test, it waits for the
/// health check to succeed on both servers.
pub async fn connect_federated_users_runner() {
    // Wait until the health check succeeds before running the test container.
    let domains: [Fqdn; NUMBER_OF_SERVERS] = (0..NUMBER_OF_SERVERS)
        .map(|index| {
            let env_var_name = format!("PHNX_SERVER_{}", index);
            std::env::var(env_var_name).unwrap().into()
        })
        .collect::<Vec<Fqdn>>()
        .try_into()
        .unwrap();
    wait_until_servers_are_up(domains.clone()).await;

    tracing::info!("Running federation test client");
    let mut test_bed = TestBed::new().await;
    let alice_name = format!("alice@{}", domains[0]);
    test_bed.add_user(alice_name.clone()).await;
    let bob_name = format!("bob@{}", domains[1]);
    test_bed.add_user(bob_name.clone()).await;
    test_bed.connect_users(alice_name, bob_name).await;
    tracing::info!("Done");
}
