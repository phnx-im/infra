// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing keys that other keys can be
//! derived from. For keys (or other values) to be derived from one of these
//! keys, the target key (or value) needs to implement the [`KdfDerivable`]
//! trait.

use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::{
    indexed_aead::keys::{Key, RandomlyGeneratable},
    secrets::Secret,
};

use super::{KDF_KEY_SIZE, KdfDerivable, traits::KdfKey};

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetSecretKeyType;
pub type RatchetSecret = Key<RatchetSecretKeyType>;

impl RandomlyGeneratable for RatchetSecretKeyType {}

impl KdfKey for RatchetSecret {
    const ADDITIONAL_LABEL: &'static str = "RatchetSecret";
}

impl KdfDerivable<RatchetSecret, Vec<u8>, KDF_KEY_SIZE> for RatchetSecret {
    const LABEL: &'static str = "RatchetSecret derive";
}

pub type ConnectionKeyKey = Secret<KDF_KEY_SIZE>;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct ConnectionKeyType;
pub type ConnectionKey = Key<ConnectionKeyType>;
impl RandomlyGeneratable for ConnectionKeyType {}

impl KdfKey for ConnectionKey {
    const ADDITIONAL_LABEL: &'static str = "FriendshipSecret";
}
