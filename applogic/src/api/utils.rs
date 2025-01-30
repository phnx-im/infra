// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Misc. functions

use flutter_rust_bridge::frb;
use sha2::{Digest, Sha256};

pub fn delete_databases(client_db_path: String) -> anyhow::Result<()> {
    phnxcoreclient::delete_databases(client_db_path.as_str())
}

/// Computes sha256 hashsum of the data and returns it as a hex string.
#[frb(sync, positional)]
pub fn calculate_sha256(data: &[u8]) -> String {
    let sha256 = Sha256::digest(data);
    hex::encode(sha256)
}
