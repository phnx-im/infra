// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;

use crate::clients::CoreUser;

impl CoreUser {
    pub async fn delete_account(&self) -> anyhow::Result<()> {
        bail!("not implemented");
    }
}
