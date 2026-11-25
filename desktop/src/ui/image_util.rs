use bookworm_core::epub::EpubCoverData;
use gpui_kit::{Image, ImageFormat};
use std::path::Path;
use std::sync::Arc;

pub fn image_format_from_name(name: &str) -> Option<ImageFormat> {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::Webp),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        "svg" => Some(ImageFormat::Svg),
        _ => None,
    }
}

pub fn image_format_from_mime_or_path(media_type: &str, zip_path: &str) -> Option<ImageFormat> {
    let mt = media_type.trim().to_ascii_lowercase();
    if mt.contains("png") {
        Some(ImageFormat::Png)
    } else if mt.contains("jpeg") || mt.contains("jpg") {
        Some(ImageFormat::Jpeg)
    } else if mt.contains("webp") {
        Some(ImageFormat::Webp)
    } else if mt.contains("gif") {
        Some(ImageFormat::Gif)
    } else if mt.contains("bmp") {
        Some(ImageFormat::Bmp)
    } else if mt.contains("svg") {
        Some(ImageFormat::Svg)
    } else {
        image_format_from_name(zip_path)
    }
}

pub fn decode_bytes_image(media_type: &str, zip_path: &str, bytes: &[u8]) -> Option<Arc<Image>> {
    if bytes.is_empty() {
        return None;
    }
    let fmt = image_format_from_mime_or_path(media_type, zip_path)?;
    Some(Arc::new(Image::from_bytes(fmt, bytes.to_vec())))
}

pub fn decode_cover_image(cover: &EpubCoverData) -> Option<Arc<Image>> {
    decode_bytes_image(&cover.media_type, &cover.zip_path, &cover.bytes)
}
