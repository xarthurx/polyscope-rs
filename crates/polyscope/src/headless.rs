//! Headless rendering API for polyscope-rs.
//!
//! Provides functions to render the current scene to an image buffer or file
//! without opening a window. Useful for integration tests, batch processing,
//! and automated screenshot generation.

use crate::Result;
use crate::ScreenshotOptions;
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
    render_to_file_with_options(filename, width, height, &ScreenshotOptions::default())
}

/// Renders the current scene to a file with screenshot options.
///
/// Same as [`render_to_file`]. With `transparent_background`, a PNG file
/// gets alpha 0 where only the background shows. JPEG files have no alpha.
pub fn render_to_file_with_options(
    filename: &str,
    width: u32,
    height: u32,
    options: &ScreenshotOptions,
) -> Result<()> {
    let data = render_to_image_with_options(width, height, options)?;
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
    render_to_image_with_options(width, height, &ScreenshotOptions::default())
}

/// Renders the current scene to a raw RGBA pixel buffer with screenshot options.
///
/// Same as [`render_to_image`]. With `transparent_background`, pixels where
/// only the background shows get alpha 0, and scene pixels keep the alpha of
/// the scene.
pub fn render_to_image_with_options(
    width: u32,
    height: u32,
    options: &ScreenshotOptions,
) -> Result<Vec<u8>> {
    let mut app = App::new();

    // Create headless render engine
    let engine = RenderEngine::new_headless(width, height)
        .block_on()
        .map_err(|e| {
            crate::PolyscopeError::RenderError(format!("Failed to create headless engine: {e}"))
        })?;
    app.engine = Some(engine);

    // Each headless render creates a new wgpu device. The structures can hold
    // GPU resources of an earlier device (an earlier window session), so drop
    // them. The frame builds them again with the new device.
    crate::app::clear_structure_gpu_resources();

    // Drain any queued view-state. `apply_view_state` sets
    // `camera_fitted = true`, which makes the per-frame `auto_fit_camera`
    // call inside `render_frame_headless` a no-op — preserving the loaded
    // camera pose for reproducible scripted figures.
    let pending = with_context_mut(polyscope_core::Context::take_pending_view_apply);
    if let Some((state, transition)) = pending {
        app.apply_view_state(&state, transition);
    }

    // Render one frame and capture
    app.render_frame_headless(options.transparent_background);

    // Publish a fresh view-state snapshot for callers querying after a headless render.
    with_context_mut(|ctx| {
        let snapshot = app.view_state_from_options(&ctx.options);
        ctx.set_view_state_snapshot(snapshot);
    });

    let pixels = app.capture_to_buffer();

    // The headless device ends with this call. Drop its GPU resources from
    // the structures, so that a later window or headless render cannot use
    // them with another device.
    drop(app);
    crate::app::clear_structure_gpu_resources();

    pixels
}
