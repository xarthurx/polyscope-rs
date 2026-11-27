use super::{App, ScreenDescriptor, render_frame, render_pick};

impl App {
    /// Renders a single frame of the window.
    ///
    /// The frame prepares the scene, builds the UI, encodes the scene passes
    /// into the HDR texture, tone maps that texture to the surface and draws
    /// the UI on top. The pick pass comes last, so it uses the uniforms and
    /// the curve network tube geometry of the image on the screen.
    /// Screenshots reuse the HDR result of this frame.
    pub(super) fn render(&mut self) {
        // Drain any queued view-state application before touching the camera.
        let pending = polyscope_core::state::with_context_mut(
            polyscope_core::Context::take_pending_view_apply,
        );
        if let Some((state, transition)) = pending {
            self.apply_view_state(&state, transition);
        }

        let (Some(engine), Some(_egui), Some(_window)) = (&self.engine, &self.egui, &self.window)
        else {
            return;
        };
        if engine.surface.is_none() {
            return;
        }

        self.prepare_frame(true);

        // Build UI (take engine/egui temporarily to satisfy borrow checker)
        let mut engine_temp = self.engine.take().unwrap();
        let mut egui_temp = self.egui.take().unwrap();
        let window_temp = self.window.clone().unwrap();

        let mut ui_result = self.build_ui(&mut engine_temp, &mut egui_temp, &window_temp);

        self.engine = Some(engine_temp);
        self.egui = Some(egui_temp);

        // The options hold the render look. Read them after the UI, so that
        // this frame already shows the UI edits.
        let options = polyscope_core::with_context(|ctx| ctx.options.clone());
        self.configure_frame(&options);

        let engine = self.engine.as_mut().unwrap();
        let egui = self.egui.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();

        // Now borrow surface for rendering
        let surface = engine.surface.as_ref().expect("surface checked above");
        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            // A skipped frame drops its egui texture deltas. egui asserts in debug
            // builds that dropped deltas were cleared first.
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                ui_result.egui_output.textures_delta.clear();
                engine.resize(engine.width, engine.height);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                ui_result.egui_output.textures_delta.clear();
                log::warn!("Surface timeout");
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                ui_result.egui_output.textures_delta.clear();
                log::warn!("Surface error: other");
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        // The scene passes render into the HDR texture.
        render_frame::encode_scene(engine, &mut encoder, &options, false);

        // Apply tone mapping from HDR to surface (always runs, uses passthrough if disabled)
        engine.render_tone_mapping(&mut encoder, &view);

        // Render egui on top (directly to surface, after tone mapping)
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [engine.width, engine.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        egui.render(
            &engine.device,
            &engine.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
            ui_result.egui_output,
        );

        engine.queue.submit(std::iter::once(encoder.finish()));
        engine.queue.present(output);

        // The pick pass draws after the scene passes, with the uniforms of the
        // frame preparation and the UI, and with the tube geometry of this frame.
        self.grid_pick_element_types = render_pick::render_pick_pass(engine);
        // A click until the next pick pass uses the camera of this pick buffer.
        self.pick_view = self.prepared_pick_view.take();

        // Handle screenshot if pending (local request from F12 key)
        if let Some(filename) = self.screenshot_pending.take() {
            self.capture_screenshot(&filename, &options, &crate::ScreenshotOptions::default());
        }

        // Handle screenshot request from public API (screenshot() / screenshot_to_file())
        if let Some(request) = crate::take_screenshot_request() {
            let filename = request.filename.unwrap_or_else(|| {
                let name = format!("screenshot_{:04}.png", self.screenshot_counter);
                self.screenshot_counter += 1;
                name
            });
            self.capture_screenshot(&filename, &options, &request.options);
        }

        // Publish a fresh snapshot for save_view_to_json callers, in one
        // write-lock acquisition (gather + write inside the same closure).
        polyscope_core::state::with_context_mut(|ctx| {
            let snapshot = self.view_state_from_options(&ctx.options);
            ctx.set_view_state_snapshot(snapshot);
        });
    }
}
