// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use ds_lib::ClientKeyPackages;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Contact {
    username: String,
    id: Vec<u8>,
    // We store multiple key package bundles here but always only use the first one right now.
    public_keys: ClientKeyPackages,
}
