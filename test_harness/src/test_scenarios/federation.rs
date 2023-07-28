// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use phnxapiclient::{ApiClient, DomainOrAddress, TransportEncryption};
use phnxbackend::qs::Fqdn;

use crate::{utils::setup::TestBed, FEDERATION_TEST_GUEST_DOMAIN, FEDERATION_TEST_OWNER_DOMAIN};

pub async fn run_federation_scenario() {
    // Wait until the health check succeeds before running the test container.
    let mut domains: HashSet<Fqdn> = [
        FEDERATION_TEST_OWNER_DOMAIN.into(),
        FEDERATION_TEST_GUEST_DOMAIN.into(),
    ]
    .into();
    let clients: Vec<ApiClient> = domains
        .iter()
        .map(|domain| ApiClient::initialize(domain.clone(), TransportEncryption::Off).unwrap())
        .collect::<Vec<ApiClient>>();

    // Do the health check
    while !domains.is_empty() {
        for client in &clients {
            if client.health_check().await {
                if let DomainOrAddress::Domain(domain) = client.domain_or_address() {
                    domains.remove(domain);
                } else {
                    panic!("Expected domain")
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2))
    }

    tracing::info!("Running federation test scenario");
    let mut test_bed = TestBed::new([
        FEDERATION_TEST_OWNER_DOMAIN.into(),
        FEDERATION_TEST_GUEST_DOMAIN.into(),
    ])
    .await;
    let alice_name = "alice".to_owned() + "@" + FEDERATION_TEST_OWNER_DOMAIN;
    test_bed.add_user(alice_name.clone()).await;
    let bob_name = "bob".to_owned() + "@" + FEDERATION_TEST_GUEST_DOMAIN;
    test_bed.add_user(bob_name.clone()).await;
    test_bed.connect_users(alice_name, bob_name).await;
    tracing::info!("Done");
}
