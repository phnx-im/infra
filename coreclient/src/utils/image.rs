// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Cursor;

use exif::{Exif, Tag};
use image::{DynamicImage, GenericImageView};
use tracing::info;

pub(crate) fn resize_profile_image(mut image_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let image = image::load_from_memory(image_bytes)?;

    // Read EXIF data
    let exif_reader = exif::Reader::new();
    let mut image_bytes_cursor = Cursor::new(&mut image_bytes);
    let exif = exif_reader
        .read_from_container(&mut image_bytes_cursor)
        .ok();

    // Resize the image
    let image = image.resize(256, 256, image::imageops::FilterType::Nearest);

    let image = rotate(exif, image);

    // Save the resized image
    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90);
    encoder.encode_image(&image)?;
    info!(
        from_bytes = image_bytes.len(),
        to_bytes = buf.len(),
        "Resized profile image",
    );
    Ok(buf)
}

const ATTACHMENT_IMAGE_QUALITY_PERCENT: f32 = 90.0;
const MAX_ATTACHMENT_IMAGE_WIDTH: u32 = 4096;
const MAX_ATTACHMENT_IMAGE_HEIGHT: u32 = 4096;
const ATTACHMENT_THUMBNAIL_WIDTH: u32 = 300;
const ATTACHMENT_THUMBNAIL_HEIGHT: u32 = 300;

pub(crate) struct ReencodedAttachmentImage {
    pub(crate) webp_image: Vec<u8>,
    pub(crate) image_dimensions: (u32, u32),
    pub(crate) webp_thumbnail: Vec<u8>,
    pub(crate) blurhash: String,
}

/// Reencodes the image to WEBP format.
///
/// This does several things:
/// - Rotates and flips the image according to the EXIF orientation
/// - Resizes the image to a maximum width and height of 4096x4096
/// - Converts the image to WebP
///
/// Returns the WebP image bytes and the blurhash of the image.
pub(crate) fn reencode_attachment_image(
    image_bytes: Vec<u8>,
) -> anyhow::Result<ReencodedAttachmentImage> {
    let mut image_bytes = image_bytes.as_slice();
    let image = image::load_from_memory(image_bytes)?;

    // Read EXIF data
    let exif_reader = exif::Reader::new();
    let mut image_bytes_cursor = Cursor::new(&mut image_bytes);
    let exif = exif_reader
        .read_from_container(&mut image_bytes_cursor)
        .ok();

    let image = rotate(exif, image);
    let image = resize(image);

    // TODO: Preserve format instead of converting to WebP

    let image_rgba = image.to_rgba8();
    let (width, height) = image_rgba.dimensions();

    let webp_image = webp::Encoder::from_rgba(&image_rgba, width, height)
        .encode(ATTACHMENT_IMAGE_QUALITY_PERCENT);

    // `blurhash::encode` can only fail if the compoments dimension is out of range
    // => We should never get an error here.
    let blurhash = blurhash::encode(4, 3, width, height, &image_rgba)?;

    let thumbnail = image
        .thumbnail(ATTACHMENT_THUMBNAIL_WIDTH, ATTACHMENT_THUMBNAIL_HEIGHT)
        .to_rgba8();
    let (thumbnail_width, thumbnail_height) = thumbnail.dimensions();
    let webp_thumbnail = webp::Encoder::from_rgba(&thumbnail, thumbnail_width, thumbnail_height)
        .encode(ATTACHMENT_IMAGE_QUALITY_PERCENT);

    info!(
        from_bytes = image_bytes.len(),
        to_bytes = webp_image.len(),
        "Reencoded attachment image as WebP",
    );

    // Note: We need to convert WebPMemory to Vec here, because the former is not Send.
    Ok(ReencodedAttachmentImage {
        webp_image: webp_image.to_vec(),
        image_dimensions: (width, height),
        webp_thumbnail: webp_thumbnail.to_vec(),
        blurhash,
    })
}

// Rotate/flip the image according to the orientation if necessary
fn rotate(exif: Option<Exif>, image: DynamicImage) -> DynamicImage {
    if let Some(exif) = exif {
        let orientation = exif
            .get_field(Tag::Orientation, exif::In::PRIMARY)
            .and_then(|field| field.value.get_uint(0))
            .unwrap_or(1);
        // TODO: roate and flip in-place
        match orientation {
            1 => image,
            2 => image.fliph(),
            3 => image.rotate180(),
            4 => image.flipv(),
            5 => image.rotate90().fliph(),
            6 => image.rotate90(),
            7 => image.rotate270().fliph(),
            8 => image.rotate270(),
            _ => image,
        }
    } else {
        image
    }
}

fn resize(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();

    if width <= MAX_ATTACHMENT_IMAGE_WIDTH && height <= MAX_ATTACHMENT_IMAGE_HEIGHT {
        return image;
    }

    let scale_x = MAX_ATTACHMENT_IMAGE_WIDTH as f32 / width as f32;
    let scale_y = MAX_ATTACHMENT_IMAGE_HEIGHT as f32 / height as f32;
    let scale = scale_x.min(scale_y);

    let new_width = (width as f32 * scale).round() as u32;
    let new_height = (height as f32 * scale).round() as u32;

    image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
}
