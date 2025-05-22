// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proof of work

use argon2::{Algorithm, Argon2, Params, Version};

pub struct PoWConfig {
    argon2: Argon2<'static>,
    threshold: u64,
}

impl PoWConfig {
    pub fn new(difficulty: u64, mem_cost: u32, time_cost: u32, parallelism: u32) -> Self {
        assert!(difficulty > 0);
        let params = Params::new(mem_cost, time_cost, parallelism, Some(8)).unwrap();
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let threshold = u64::MAX / difficulty;
        Self { argon2, threshold }
    }
}

pub fn verify_nonce(data: &[u8], nonce: u64, salt: &[u8], cfg: &PoWConfig) -> bool {
    let mut input = Vec::with_capacity(data.len() + 8);
    input.extend_from_slice(data);
    input.extend_from_slice(&nonce.to_be_bytes());
    let mut hash8 = [0u8; 8];
    cfg.argon2
        .hash_password_into(&input, salt, &mut hash8)
        .unwrap();
    u64::from_be_bytes(hash8) <= cfg.threshold
}

pub fn find_nonce(data: &[u8], salt: &[u8], cfg: &PoWConfig) -> Option<u64> {
    let mut nonce = 0u64;
    while !verify_nonce(data, nonce, salt, cfg) {
        nonce = nonce.wrapping_add(1);
    }
    Some(nonce)
}

#[cfg(test)]
mod tests {
    use crate::pow::{verify_nonce, PoWConfig};

    #[test]
    fn diff1_always_passes() {
        let cfg = PoWConfig::new(1, 64, 1, 1);
        assert!(verify_nonce(b"any", 42, b"salt", &cfg));
    }

    #[test]
    fn high_difficulty_rejects_easy_nonce() {
        let cfg = PoWConfig::new(u64::MAX, 64, 1, 1);
        assert!(!verify_nonce(b"foo", 0, b"salt", &cfg));
    }
}
