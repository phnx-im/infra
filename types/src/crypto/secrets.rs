// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains the [`Secret`] struct, which is meant to be
//! accessible only within the [`crate::crypto`] module. This
//! module should stay private, such that the [`Secret`] struct can stay public
//! and the type checker happy.
use std::fmt::Display;

use rand::{RngCore, SeedableRng};
#[cfg(feature = "sqlite")]
use rusqlite::{types::FromSql, ToSql};
use secrecy::Zeroize;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use super::RandomnessError;

/// Struct that contains a (symmetric) secret of fixed length LENGTH.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, PartialEq, Serialize, Deserialize)]
pub struct Secret<const LENGTH: usize> {
    #[serde(with = "super::serde_arrays")]
    pub secret: [u8; LENGTH],
}

impl<const LENGTH: usize> Secret<LENGTH> {
    /// Get the internal secret value
    pub fn secret(&self) -> &[u8; LENGTH] {
        &self.secret
    }

    /// Generate a fresh, random secret.
    pub fn random() -> Result<Self, RandomnessError> {
        let mut secret = [0; LENGTH];
        // TODO: Use a proper rng provider.
        rand_chacha::ChaCha20Rng::from_entropy()
            .try_fill_bytes(secret.as_mut_slice())
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self { secret })
    }
}

// Ensure that secrets are wiped from memory securely upon being dropped.
impl<const LENGTH: usize> Zeroize for Secret<LENGTH> {
    fn zeroize(&mut self) {
        self.secret.zeroize()
    }
}

// Ensures that secrets are not printed in debug outputs.
impl<const LENGTH: usize> std::fmt::Debug for Secret<LENGTH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secret: [[REDACTED]]").finish()
    }
}

// Ensures that secrets are not printed in format strings.
impl<const LENGTH: usize> Display for Secret<LENGTH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[[REDACTED]]")
    }
}

#[cfg(feature = "sqlite")]
impl ToSql for Secret<32> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.secret.to_vec()))
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for Secret<32> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let secret = value.as_blob()?;
        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(secret);
        Ok(Secret {
            secret: secret_bytes,
        })
    }
}
