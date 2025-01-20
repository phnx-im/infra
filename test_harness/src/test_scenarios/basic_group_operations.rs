// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::{Fqdn, QualifiedUserName};

use crate::utils::setup::TestBackend;

type TestBed = TestBackend;

pub(super) const NUMBER_OF_SERVERS: usize = 2;

impl TestBed {
    async fn create_alice_and_bob(&mut self, domains: &[Fqdn]) -> (String, String) {
        let alice_name = format!("alice@{}", domains[0]);
        self.add_user(&alice_name.parse().unwrap()).await;
        let bob_name = format!("bob@{}", domains[1]);
        self.add_user(&bob_name.parse().unwrap()).await;
        (alice_name, bob_name)
    }

    pub(crate) async fn create_and_connect_alice_and_bob(
        &mut self,
        domains: &[Fqdn],
    ) -> (String, String) {
        let (alice_name, bob_name) = self.create_alice_and_bob(domains).await;
        self.connect_users(&alice_name.parse().unwrap(), &bob_name.parse().unwrap())
            .await;
        (alice_name, bob_name)
    }
}

/// This function is meant to be called from the test container. It registers
/// two clients, one on each test server and makes them perform the requests
/// required to connect the two. Before running the test, it waits for the
/// health check to succeed on both servers.
pub async fn connect_users_runner(domains: &[Fqdn]) {
    let mut test_bed = TestBed::federated();
    test_bed.create_and_connect_alice_and_bob(domains).await;
}

pub async fn invite_to_group_runner(domains: &[Fqdn]) {
    let mut test_bed = TestBed::federated();
    let (alice_name, bob_name) = test_bed.create_and_connect_alice_and_bob(domains).await;
    let alice = alice_name.parse().unwrap();
    let bob = bob_name.parse().unwrap();
    let conversation_id = test_bed.create_group(&alice).await;
    test_bed
        .invite_to_group(conversation_id, &alice, vec![&bob])
        .await;
}

pub async fn remove_from_group_runner(domains: &[Fqdn]) {
    let mut test_bed = TestBed::federated();
    let (alice_name, bob_name) = test_bed.create_and_connect_alice_and_bob(domains).await;
    let alice: QualifiedUserName = alice_name.parse().unwrap();
    let bob: QualifiedUserName = bob_name.parse().unwrap();
    let conversation_id = test_bed.create_group(&alice).await;
    test_bed
        .invite_to_group(conversation_id, &alice, vec![&bob])
        .await;
    test_bed
        .remove_from_group(conversation_id, &alice, vec![&bob])
        .await;
}

pub async fn leave_group_runner(domains: &[Fqdn]) {
    let mut test_bed = TestBed::federated();
    let (alice_name, bob_name) = test_bed.create_and_connect_alice_and_bob(domains).await;
    let alice: QualifiedUserName = alice_name.parse().unwrap();
    let bob: QualifiedUserName = bob_name.parse().unwrap();
    let conversation_id = test_bed.create_group(&alice).await;
    test_bed
        .invite_to_group(conversation_id, &alice, vec![&bob])
        .await;
    test_bed.leave_group(conversation_id, &bob).await
}
