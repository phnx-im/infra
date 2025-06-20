// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{AttachmentId, clients::CoreUser};

impl CoreUser {
    pub(crate) async fn download_attachment(
        &self,
        _attachment_id: AttachmentId,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
