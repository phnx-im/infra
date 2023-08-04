// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::qs::Fqdn;
use rand::{seq::SliceRandom, SeedableRng};

use crate::utils::setup::TestBed;

pub(super) const NUMBER_OF_SERVERS: usize = 3;

pub async fn randomized_operations_runner(domains: &[Fqdn]) {
    let randomness_seed: u64 = rand::random();
    tracing::info!("randomness_seed: {}", randomness_seed);
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(randomness_seed as u64);
    let mut test_bed = TestBed::new().await;
    for index in 0..10 {
        // Pick a random domain
        let domain = domains.choose(&mut rng).unwrap();
        // Just count the users to avoid collisions
        let user_name = format!("{}@{}", index, domain);
        test_bed.add_user(user_name).await;
    }
    for _index in 0..100 {
        test_bed.perform_random_operation(&mut rng).await;
    }
    tracing::info!("Done testing with randomness_seed: {}", randomness_seed);
}
