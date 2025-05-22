// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rate Limiter

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use async_trait::async_trait;

#[derive(Clone)]
pub struct Config {
    max_requests: u64,
    time_window: Duration,
}

#[derive(Clone)]
pub struct Allowance {
    remaining: u64,
    last_reset: Instant,
}

impl Allowance {
    pub fn new(config: &Config) -> Self {
        Allowance {
            remaining: config.max_requests,
            last_reset: Instant::now(),
        }
    }

    fn reset(&mut self, config: &Config) {
        self.remaining = config.max_requests;
        self.last_reset = Instant::now();
    }

    fn allowed(&mut self, config: &Config) -> bool {
        // Check if the time window has passed
        if self.last_reset + config.time_window < Instant::now() {
            self.reset(config);
        }

        if self.remaining == 0 {
            false
        } else {
            self.remaining -= 1;
            true
        }
    }
}

#[async_trait]
pub trait StorageProvider {
    async fn get(&self, key: &[u8]) -> Option<Allowance>;
    async fn set(&mut self, key: Vec<u8>, allowance: Allowance);
}

pub struct RateLimiter<S: StorageProvider> {
    config: Config,
    storage: S,
}

impl<S: StorageProvider> RateLimiter<S> {
    pub fn new(config: Config, storage: S) -> Self {
        RateLimiter { config, storage }
    }

    pub async fn allowed(&mut self, key: Vec<u8>) -> bool {
        let mut allowance = self
            .storage
            .get(&key)
            .await
            .unwrap_or_else(|| Allowance::new(&self.config));

        if allowance.allowed(&self.config) {
            self.storage.set(key, allowance.clone()).await;
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct InMemoryStorage {
    data: HashMap<Vec<u8>, Allowance>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            data: HashMap::new(),
        }
    }
}

#[async_trait]
impl StorageProvider for InMemoryStorage {
    async fn get(&self, key: &[u8]) -> Option<Allowance> {
        self.data.get(key).cloned()
    }

    async fn set(&mut self, key: Vec<u8>, allowance: Allowance) {
        self.data.insert(key, allowance);
    }
}

#[tokio::test]
async fn test_rate_limiter() {
    let config = Config {
        max_requests: 5,
        time_window: Duration::from_secs(1),
    };
    let storage = InMemoryStorage::new();
    let mut rate_limiter = RateLimiter::new(config.clone(), storage);

    let key = b"user1".to_vec();

    // First 5 requests should succeed
    for _ in 0..config.max_requests {
        assert!(rate_limiter.allowed(key.clone()).await);
    }

    // 6th request should fail
    assert!(!rate_limiter.allowed(key.clone()).await);

    // Wait for the time window to reset
    std::thread::sleep(config.time_window);

    // Now it should succeed again
    assert!(rate_limiter.allowed(key).await);
}
