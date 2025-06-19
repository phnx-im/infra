// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;

use crate::{
    clients::attachment::{AttachmentRecord, persistence::AttachmentImageRecord},
    utils::image::reencode_attachment_image,
};

impl AttachmentRecord {
    pub(super) async fn calculate_image_record(&self) -> anyhow::Result<AttachmentImageRecord> {
        // let (data, blurhash) =
        //     reencode_attachment_image(&self.content).context("Failed to reencode image")?;
        //
        // let content = AttachmentImageRecord {
        //     attachment_id: self.attachment_id,
        //     thumbnail: data.to_vec(),
        //     thumbnail_size: data.len() as u32,
        //     blurhash,
        //     width: self.width,
        //     height: self.height,
        // };

        todo!()
    }
}
