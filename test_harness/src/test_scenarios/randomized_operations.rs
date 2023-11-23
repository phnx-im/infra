// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::Fqdn;
use rand::{seq::SliceRandom, SeedableRng};

use crate::utils::setup::TestBackend;

type TestBed = TestBackend;

pub(super) const NUMBER_OF_SERVERS: usize = 3;

pub async fn randomized_operations_runner(domains: &[Fqdn]) {
    // Check if a specific seed was set manually.
    let randomness_seed: u64 = if let Ok(seed) = std::env::var("PHNX_TEST_RANDOM_SEED") {
        tracing::info!("setting seed manually from environment");
        seed.parse().unwrap()
    } else {
        rand::random()
    };
    tracing::info!(
        random_operation = true,
        "randomness_seed: {}",
        randomness_seed
    );
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(randomness_seed as u64);
    let mut test_bed = TestBed::federated();
    for index in 0..10 {
        // Pick a random domain
        let domain = domains.choose(&mut rng).unwrap();
        // Just count the users to avoid collisions
        let user_name = format!("{}@{}", index, domain);
        tracing::info!(
            random_operation = true,
            "Random operation: Creating user {}",
            user_name
        );
        test_bed.add_user(user_name).await;
    }
    for _index in 0..100 {
        test_bed.perform_random_operation(&mut rng).await;
    }
    tracing::info!("Done testing with randomness_seed: {}", randomness_seed);
}
