// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::Fqdn;

use crate::federation_utils::setup::TestBed;

pub(super) const NUMBER_OF_SERVERS: usize = 3;

pub async fn group_operations_runner(domains: &[Fqdn]) {
    // Create three users.
    let mut test_bed = TestBed::new().await;
    // Create and connect alice and bob.
    let (alice_name, bob_name) = test_bed.create_and_connect_alice_and_bob(domains).await;
    let charlie_name = format!("charlie@{}", domains[2]);
    test_bed.add_user(charlie_name.clone()).await;

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
}
