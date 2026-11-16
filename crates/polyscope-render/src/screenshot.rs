//! Screenshot functionality for capturing rendered frames.

use image::{ImageBuffer, Rgba};
use std::path::Path;

/// Options for taking screenshots.
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions {
    /// Whether to use a transparent background.
    ///
    /// Pixels where only the background shows get alpha 0. Scene pixels keep
    /// the alpha of the scene (for example of transparent meshes). The ground
    /// plane is part of the scene: it stays in the image with the alpha of its
    /// shader (opaque near the scene, fading out far away; in shadow-only mode
    /// only the shadow has alpha), as in C++ Polyscope. PNG files store the
    /// alpha; JPEG files have no alpha channel.
    pub transparent_background: bool,
}

/// Saves raw RGBA pixel data to an image file.
///
/// # Arguments
/// * `filename` - Output filename (supports .png, .jpg, .jpeg)
/// * `data` - Raw RGBA pixel data (4 bytes per pixel, row by row from the top-left
///   pixel), as `RenderEngine::capture_screenshot` returns it
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
        .map(str::to_lowercase)
        .unwrap_or_default();

    let img = rgba_image(data, width, height)?;

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

/// Saves raw RGBA pixel data to a PNG buffer in memory.
///
/// # Arguments
/// * `data` - Raw RGBA pixel data (4 bytes per pixel, row by row from the top-left
///   pixel), as `RenderEngine::capture_screenshot` returns it
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
/// PNG-encoded image data as a byte vector.
///
/// # Errors
/// Returns an error if the data size does not match the dimensions or the
/// PNG encoding fails.
pub fn save_to_buffer(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ScreenshotError> {
    let img = rgba_image(data, width, height)?;

    let mut buffer = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buffer, image::ImageFormat::Png)?;

    Ok(buffer.into_inner())
}

/// Wraps RGBA pixel data in an image buffer.
///
/// wgpu uses a top-left origin, so the rows need no vertical flip.
fn rgba_image(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ScreenshotError> {
    ImageBuffer::from_raw(width, height, data.to_vec()).ok_or(ScreenshotError::InvalidImageData)
}

/// Converts pixel data of a capture texture to RGBA in place.
///
/// The capture texture has the surface format. It is BGRA on most desktops
/// and RGBA on others and in headless rendering. The function swaps red and
/// blue for the BGRA formats and leaves the data of other formats unchanged.
pub(crate) fn convert_to_rgba(format: wgpu::TextureFormat, data: &mut [u8]) {
    if matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for pixel in data.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
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

    /// Two pixels: opaque red and half-transparent blue, in RGBA order.
    const RGBA: [u8; 8] = [255, 0, 0, 255, 0, 0, 255, 128];

    #[test]
    fn bgra_capture_converts_to_rgba() {
        for format in [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            // The same two pixels, in BGRA order
            let mut data = [0, 0, 255, 255, 255, 0, 0, 128];
            convert_to_rgba(format, &mut data);
            assert_eq!(data, RGBA, "{format:?} must convert to RGBA");
        }
    }

    #[test]
    fn rgba_capture_stays_unchanged() {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ] {
            let mut data = RGBA;
            convert_to_rgba(format, &mut data);
            assert_eq!(data, RGBA, "{format:?} must stay unchanged");
        }
    }

    #[test]
    fn save_image_writes_rgba_input_unchanged() {
        let path = std::env::temp_dir().join(format!(
            "polyscope_save_image_test_{}.png",
            std::process::id()
        ));
        let filename = path.to_str().expect("UTF-8 temp path");
        save_image(filename, &RGBA, 2, 1).expect("save PNG");
        let decoded = image::open(&path).expect("decode PNG").to_rgba8();
        let _ = std::fs::remove_file(&path);
        assert_eq!(decoded.dimensions(), (2, 1));
        assert_eq!(decoded.into_raw(), RGBA.to_vec());
    }

    #[test]
    fn save_to_buffer_encodes_rgba_input_unchanged() {
        let png = save_to_buffer(&RGBA, 2, 1).expect("encode PNG");
        let decoded = image::load_from_memory(&png)
            .expect("decode PNG")
            .to_rgba8();
        assert_eq!(decoded.into_raw(), RGBA.to_vec());
    }

    #[test]
    fn save_image_rejects_wrong_data_size() {
        let result = save_to_buffer(&RGBA, 3, 1);
        assert!(matches!(result, Err(ScreenshotError::InvalidImageData)));
    }
}
