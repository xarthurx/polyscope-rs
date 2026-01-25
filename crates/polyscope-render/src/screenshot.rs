//! Screenshot functionality for capturing rendered frames.

use image::{ImageBuffer, Rgba};
use std::path::Path;

/// Options for taking screenshots.
#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    /// Whether to use transparent background (PNG only).
    pub transparent_background: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            transparent_background: false,
        }
    }
}

/// Saves raw BGRA pixel data to an image file.
///
/// # Arguments
/// * `filename` - Output filename (supports .png, .jpg, .jpeg)
/// * `data` - Raw BGRA pixel data (4 bytes per pixel, as from wgpu Bgra8UnormSrgb format)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Errors
/// Returns an error if the file cannot be written or format is unsupported.
pub fn save_image(
    filename: &str,
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<(), ScreenshotError> {
    let path = Path::new(filename);
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // Convert BGRA to RGBA (wgpu surface format is Bgra8UnormSrgb)
    let mut rgba_data = data.to_vec();
    for chunk in rgba_data.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap B and R
    }

    // Create image buffer from converted RGBA data
    // Note: wgpu uses top-left origin, so no vertical flip needed
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba_data)
        .ok_or(ScreenshotError::InvalidImageData)?;

    match extension.as_str() {
        "png" => {
            img.save_with_format(path, image::ImageFormat::Png)?;
        }
        "jpg" | "jpeg" => {
            // Convert to RGB for JPEG (no alpha)
            let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
            rgb_img.save_with_format(path, image::ImageFormat::Jpeg)?;
        }
        _ => {
            return Err(ScreenshotError::UnsupportedFormat(extension));
        }
    }

    Ok(())
}

/// Saves raw BGRA pixel data to a PNG buffer in memory.
///
/// # Arguments
/// * `data` - Raw BGRA pixel data (4 bytes per pixel, as from wgpu Bgra8UnormSrgb format)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
/// PNG-encoded image data as a byte vector.
pub fn save_to_buffer(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ScreenshotError> {
    // Convert BGRA to RGBA
    let mut rgba_data = data.to_vec();
    for chunk in rgba_data.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap B and R
    }

    // Note: wgpu uses top-left origin, so no vertical flip needed
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba_data)
        .ok_or(ScreenshotError::InvalidImageData)?;

    let mut buffer = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buffer, image::ImageFormat::Png)?;

    Ok(buffer.into_inner())
}

/// Error type for screenshot operations.
#[derive(Debug, thiserror::Error)]
pub enum ScreenshotError {
    #[error("Failed to save image: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image encoding error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid image data")]
    InvalidImageData,

    #[error("GPU buffer mapping failed")]
    BufferMapFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_options_default() {
        let opts = ScreenshotOptions::default();
        assert!(!opts.transparent_background);
    }
}
