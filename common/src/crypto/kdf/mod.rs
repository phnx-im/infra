// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module and its submodules facilitate the derivation of keys throughout
//! the backend. See the individual submodules for more details.

pub mod keys;
mod traits;

use hkdf::Hkdf;

use super::Hash;

pub use traits::{KdfDerivable, KdfExtractable, KdfKey};

/// This type determines the KDF used by the backend.
pub type Kdf = Hkdf<Hash>;
// Size of a KDF key.
// TODO: This can probably be gotten generically from the Kdf type.
pub const KDF_KEY_SIZE: usize = 32;
