//! Screenshot capture and headless rendering.
//!
//! Both use the frame pipeline of the window (`render_frame`). A capture tone
//! maps the HDR scene texture into the screenshot target instead of the
//! surface, and it draws no UI.

use polyscope_render::ScreenshotOptions;

use super::{App, Options, render_frame};

impl App {
    /// Saves the frame that the window shows, without the UI.
    ///
    /// Call this after the frame is submitted. The HDR texture still holds the
    /// scene of that frame, so the capture tone maps it again into the
    /// screenshot target. A transparent background needs background alpha 0,
    /// so in that case the scene passes run again first.
    pub(super) fn capture_screenshot(
        &mut self,
        filename: &str,
        options: &Options,
        screenshot_options: &ScreenshotOptions,
    ) {
        let Some(engine) = &mut self.engine else {
            log::error!("Cannot capture screenshot: engine not initialized");
            return;
        };

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot encoder"),
            });
        if screenshot_options.transparent_background {
            render_frame::encode_scene(engine, &mut encoder, options, true);
        }
        engine.tone_map_to_screenshot_target(&mut encoder);
        engine.queue.submit(std::iter::once(encoder.finish()));

        match engine.capture_screenshot() {
            Ok(data) => {
                let (width, height) = engine.dimensions();
                match polyscope_render::save_image(filename, &data, width, height) {
                    Ok(()) => {
                        log::info!("Screenshot saved to {filename}");
                    }
                    Err(e) => {
                        log::error!("Failed to save screenshot: {e}");
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to capture screenshot: {e}");
            }
        }
    }

    /// Renders a single frame in headless mode (no window, no egui).
    ///
    /// The frame uses the same preparation and scene passes as the window
    /// frame, then tone maps the scene into the screenshot target.
    /// Call `capture_to_buffer()` after this to retrieve pixel data.
    ///
    /// With `transparent_background`, background pixels get alpha 0 and scene
    /// pixels keep the alpha of the scene.
    pub(crate) fn render_frame_headless(&mut self, transparent_background: bool) {
        let options = crate::with_context(|ctx| ctx.options.clone());
        self.configure_frame(&options);
        self.prepare_frame(false);

        let Some(engine) = &mut self.engine else {
            return;
        };
        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless render encoder"),
            });
        render_frame::encode_scene(engine, &mut encoder, &options, transparent_background);
        engine.tone_map_to_screenshot_target(&mut encoder);
        engine.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Captures the rendered frame to a raw RGBA pixel buffer.
    /// Must be called after `render_frame_headless()`.
    pub(crate) fn capture_to_buffer(&mut self) -> crate::Result<Vec<u8>> {
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| crate::PolyscopeError::RenderError("Engine not initialized".into()))?;

        engine.capture_screenshot().map_err(|e| {
            crate::PolyscopeError::RenderError(format!("Failed to capture screenshot: {e}"))
        })
    }
}
