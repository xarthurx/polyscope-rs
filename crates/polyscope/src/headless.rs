//! Headless rendering API for polyscope-rs.
//!
//! Provides functions to render the current scene to an image buffer or file
//! without opening a window. Useful for integration tests, batch processing,
//! and automated screenshot generation.

use crate::Result;
use crate::app::App;
use pollster::FutureExt;
use polyscope_core::state::with_context_mut;
use polyscope_render::RenderEngine;

/// Renders the current scene to a file.
///
/// Creates a headless GPU context, renders one frame of the current scene
/// (all registered structures and quantities), and saves the result as
/// a PNG or JPEG image.
///
/// The camera is automatically fitted to the scene bounding box.
///
/// # Example
/// ```no_run
/// use polyscope_rs::*;
///
/// init().unwrap();
/// register_point_cloud("pts", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
/// render_to_file("output.png", 800, 600).unwrap();
/// ```
pub fn render_to_file(filename: &str, width: u32, height: u32) -> Result<()> {
    let data = render_to_image(width, height)?;
    polyscope_render::save_image(filename, &data, width, height)
        .map_err(|e| crate::PolyscopeError::RenderError(format!("Failed to save image: {e}")))
}

/// Renders the current scene to a raw RGBA pixel buffer.
///
/// Creates a headless GPU context, renders one frame of the current scene,
/// and returns the pixel data as `Vec<u8>` in RGBA format (4 bytes per pixel).
///
/// The returned buffer has dimensions `width * height * 4` bytes.
/// Pixels are ordered row-by-row from top-left to bottom-right.
///
/// # Example
/// ```no_run
/// use polyscope_rs::*;
///
/// init().unwrap();
/// register_point_cloud("pts", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
/// let pixels = render_to_image(800, 600).unwrap();
/// assert_eq!(pixels.len(), 800 * 600 * 4);
/// ```
pub fn render_to_image(width: u32, height: u32) -> Result<Vec<u8>> {
    let mut app = App::new();

    // Create headless render engine
    let engine = RenderEngine::new_headless(width, height)
        .block_on()
        .map_err(|e| {
            crate::PolyscopeError::RenderError(format!("Failed to create headless engine: {e}"))
        })?;
    app.engine = Some(engine);

    // Clear stale GPU resources from all structures so they get re-initialized
    // with the new wgpu device. This is necessary because each headless render
    // call creates a fresh device, but structures in the global registry may
    // retain buffers from a previous device.
    with_context_mut(|ctx| {
        for structure in ctx.registry.iter_mut() {
            structure.clear_gpu_resources();
        }
    });

    // Render one frame and capture
    app.render_frame_headless();
    app.capture_to_buffer()
}
