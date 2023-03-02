//#![allow(dead_code)]
//#![allow(unused_variables)]
#![deny(private_in_public)]
#![deny(unreachable_pub)]

pub mod auth_service;
pub mod crypto;
pub mod ds;
pub mod messages;
pub mod qs;

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unrecoverable error in this implementation.
#[derive(Debug, Error, Serialize, Deserialize)]
pub struct LibraryError;

impl LibraryError {
    pub fn missing_bound_check(_error: serde_json::Error) -> Self {
        LibraryError {}
    }

    pub fn unexpected_crypto_error(_error: &str) -> Self {
        LibraryError {}
    }
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
