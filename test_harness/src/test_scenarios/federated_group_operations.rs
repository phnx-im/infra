// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::qs::Fqdn;

use crate::{docker::wait_until_servers_are_up, utils::setup::TestBed};

pub(super) const NUMBER_OF_SERVERS: usize = 3;

pub async fn federated_group_operations_runner() {
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
    // Create three users.
    let mut test_bed = TestBed::new().await;
    let alice_name = format!("alice@{}", domains[0]);
    test_bed.add_user(alice_name.clone()).await;
    let bob_name = format!("bob@{}", domains[1]);
    test_bed.add_user(bob_name.clone()).await;
    let charlie_name = format!("charlie@{}", domains[2]);
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
