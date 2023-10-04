// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module and its submodules contain structs and traits that allow the
//! computation and verification of MACs over other structs.

use hmac::Hmac;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use super::Hash;

pub mod keys;
pub mod traits;

// Re-export the traits that we want to be available outside of this module.
pub use traits::{
    MacComputationError, MacVerificationError, TagVerifiable, TagVerified, Taggable, TaggedStruct,
};

/// This type determines the MAC used by the backend.
pub type Mac = Hmac<Hash>;
pub type MacError = hmac::digest::MacError;
// TODO: There might be a way to get this generically from Mac.
pub const MAC_KEY_SIZE: usize = 32;

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct MacTag {
    tag: Vec<u8>,
}
