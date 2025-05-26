// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rate Limiter

use chrono::TimeDelta;
use sha2::{Digest, Sha256};
use sqlx::types::chrono::{DateTime, Utc};

pub(crate) mod provider;

#[derive(Debug, Clone)]
pub(crate) struct RLConfig {
    pub(crate) max_requests: u64,
    pub(crate) time_window: TimeDelta,
}

#[derive(Debug, Clone)]
pub(crate) struct RLKey {
    key: [u8; 32],
}

impl RLKey {
    pub(crate) fn new(service_name: &[u8], rpc_name: &[u8], custom: &[&[u8]]) -> Self {
        let key = {
            let mut hasher = Sha256::new();

            for part in [service_name, rpc_name]
                .into_iter()
                .chain(custom.iter().copied())
            {
                hasher.update((part.len() as u32).to_be_bytes());
                hasher.update(part);
            }

            hasher.finalize().into()
        };

        RLKey { key }
    }

    pub(crate) fn serialize(&self) -> &[u8] {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Allowance {
    remaining: u64,
    valid_until: DateTime<Utc>,
}

impl Allowance {
    pub(crate) fn new(config: &RLConfig) -> Self {
        Allowance {
            remaining: config.max_requests,
            valid_until: Utc::now() + config.time_window,
        }
    }

    fn reset(&mut self, config: &RLConfig) {
        self.remaining = config.max_requests;
        self.valid_until = Utc::now() + config.time_window;
    }

    fn allowed(&mut self, config: &RLConfig) -> bool {
        // Check if the time window has passed
        if self.valid_until < Utc::now() {
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

pub(crate) trait StorageProvider {
    async fn get(&self, key: &RLKey) -> Option<Allowance>;
    async fn set(&self, key: RLKey, allowance: Allowance);
}

pub(crate) struct RateLimiter<S: StorageProvider> {
    config: RLConfig,
    storage: S,
}

impl<S: StorageProvider> RateLimiter<S> {
    pub(crate) fn new(config: RLConfig, storage: S) -> Self {
        RateLimiter { config, storage }
    }

    pub(crate) async fn allowed(&self, key: RLKey) -> bool {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::TimeDelta;
    use tokio::sync::Mutex;

    use crate::rate_limiter::{RLConfig, RateLimiter};

    use super::{Allowance, RLKey, StorageProvider};

    #[derive(Default)]
    pub struct InMemoryStorage {
        data: Mutex<HashMap<Vec<u8>, Allowance>>,
    }

    impl InMemoryStorage {
        pub fn new() -> Self {
            InMemoryStorage {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl StorageProvider for InMemoryStorage {
        async fn get(&self, key: &RLKey) -> Option<Allowance> {
            self.data.lock().await.get(key.serialize()).cloned()
        }

        async fn set(&self, key: RLKey, allowance: Allowance) {
            self.data
                .lock()
                .await
                .insert(key.serialize().to_owned(), allowance);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RLConfig {
            max_requests: 5,
            time_window: TimeDelta::milliseconds(1),
        };
        let storage = InMemoryStorage::new();
        let rate_limiter = RateLimiter::new(config.clone(), storage);

        let key = RLKey::new(b"test_service", b"test_rpc", &[]);

        // First 5 requests should succeed
        for _ in 0..config.max_requests {
            assert!(rate_limiter.allowed(key.clone()).await);
        }

        // 6th request should fail
        assert!(!rate_limiter.allowed(key.clone()).await);

        // Wait for the time window to reset
        tokio::time::sleep(config.time_window.to_std().unwrap()).await;

        // Now it should succeed again
        assert!(rate_limiter.allowed(key).await);
    }
}
