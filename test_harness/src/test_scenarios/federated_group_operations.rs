// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::{Fqdn, QualifiedUserName};

use crate::utils::setup::TestBackend;

type TestBed = TestBackend;

pub(super) const NUMBER_OF_SERVERS: usize = 3;

pub async fn group_operations_runner(domains: &[Fqdn]) {
    // Create three users.
    let mut test_bed = TestBed::federated();
    // Create and connect alice and bob.
    let (alice_name, bob_name) = test_bed.create_and_connect_alice_and_bob(domains).await;
    let charlie_name = format!("charlie@{}", domains[2]);

    let alice: QualifiedUserName = alice_name.parse().unwrap();
    let bob: QualifiedUserName = bob_name.parse().unwrap();
    let charlie: QualifiedUserName = charlie_name.parse().unwrap();

    test_bed.add_user(&charlie).await;

    // Connect bob and charlie.
    test_bed.connect_users(&bob, &charlie).await;

    // Have alice create a group.
    let conversation_id = test_bed.create_group(&alice).await;

    // Have alice invite bob
    test_bed
        .invite_to_group(conversation_id, &alice, vec![&bob])
        .await;

    // Have bob invite charlie
    test_bed
        .invite_to_group(conversation_id, &bob, vec![&charlie])
        .await;

    // Have charlie remove alice
    test_bed
        .remove_from_group(conversation_id, &charlie, vec![&alice])
        .await;

    // Have bob leave the group
    test_bed.leave_group(conversation_id, &bob).await;
}
