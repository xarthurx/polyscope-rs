//! Screenshot capture and headless rendering.

use polyscope_render::ScreenshotOptions;

use super::{App, GroundPlaneMode, Options, render_peel, render_scene};

impl App {
    /// Captures a screenshot by re-rendering to a dedicated texture.
    ///
    /// It uses the same scene passes as headless rendering.
    pub(super) fn capture_screenshot(
        &mut self,
        filename: String,
        screenshot_options: &ScreenshotOptions,
    ) {
        if self.engine.is_none() {
            log::error!("Cannot capture screenshot: engine not initialized");
            return;
        }
        let options = crate::with_context(|ctx| ctx.options.clone());
        self.render_screenshot_target(&options, screenshot_options.transparent_background);

        let Some(engine) = &mut self.engine else {
            return;
        };
        match engine.capture_screenshot() {
            Ok(data) => {
                let (width, height) = engine.dimensions();
                match polyscope_render::save_image(&filename, &data, width, height) {
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
    /// Initializes GPU resources for all structures, updates uniforms,
    /// and renders the scene to the screenshot target texture.
    /// Call `capture_to_buffer()` after this to retrieve pixel data.
    pub(crate) fn render_frame_headless(&mut self, transparent_background: bool) {
        let options = crate::with_context(|ctx| ctx.options.clone());
        let Some(engine) = &mut self.engine else {
            return;
        };

        engine.set_ssaa_factor(options.ssaa_factor);

        // Headless rendering keeps the default tone mapping values and sets
        // only the SSAO flag.
        let tone = polyscope_render::tone_mapping::ToneMapUniforms::default();
        let ssao_enabled = super::ssao_active(&options, engine);
        engine.update_tone_mapping(tone.exposure, tone.white_level, tone.gamma, ssao_enabled);

        // Auto-fit camera to scene
        self.camera_fitted = super::render_init::auto_fit_camera(engine, self.camera_fitted);

        // Fall back to the home view if the view matrix is not valid
        super::render_init::ensure_view_valid(engine);

        // Drain deferred material load queue
        super::render_init::drain_material_queue(engine);

        // Update camera and slice plane uniforms
        super::render_init::update_uniforms(engine);

        // Initialize GPU resources for all structures (shared function)
        super::render_init::init_structure_gpu_resources(engine);

        // Update GPU buffers (headless: no pick uniforms)
        super::render_init::update_gpu_buffers(engine, false);

        self.render_screenshot_target(&options, transparent_background);
    }

    /// Renders the scene to the screenshot target texture without saving to file.
    /// The pixel data can be retrieved via `capture_to_buffer()`.
    ///
    /// With `transparent_background`, background pixels get alpha 0 and scene
    /// pixels keep the alpha of the scene.
    fn render_screenshot_target(&mut self, options: &Options, transparent_background: bool) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let ground_plane = &options.ground_plane;
        let use_depth_peel = options.transparency_mode == polyscope_core::TransparencyMode::Pretty;

        let screenshot_view = engine.create_screenshot_target();

        // The mesh pipeline expects 2 color attachments (HDR + normal G-buffer).
        // Ensure the normal texture exists before any mesh pass.
        if engine.mesh_pipeline.is_some() && engine.normal_view().is_none() {
            let (w, h) = engine.render_dimensions();
            engine.create_normal_texture_with_size(w, h);
        }

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless render encoder"),
            });

        // Depth peeling writes the mesh depth before the other geometry, as
        // the windowed frame does. The target must be clear before that.
        let mut scene_color_load =
            wgpu::LoadOp::Clear(background_clear_color(options, transparent_background));
        let mut scene_depth_load = wgpu::LoadOp::Clear(1.0);
        if use_depth_peel {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: scene_color_load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: scene_depth_load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            render_peel::render_mesh_depth_prepass(&mut encoder, engine, &screenshot_view);
            scene_color_load = wgpu::LoadOp::Load;
            scene_depth_load = wgpu::LoadOp::Load;
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: scene_color_load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: scene_depth_load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw point clouds
            render_scene::draw_point_clouds(&mut render_pass, engine);

            // Draw vector quantities
            render_scene::draw_vector_quantities(&mut render_pass, engine);

            // Draw curve networks, camera views, and volume grids
            render_scene::draw_curve_networks_and_lines(&mut render_pass, engine);
        }

        // Surface mesh / volume mesh pass (MRT: HDR + normal G-buffer)
        // The mesh pipeline expects 2 color attachments, so we need a separate pass.
        if let Some(mesh_pipeline) = &engine.mesh_pipeline
            && let Some(normal_view) = engine.normal_view()
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless mesh pass (MRT)"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &screenshot_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.5,
                                g: 0.5,
                                b: 1.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            if use_depth_peel {
                render_peel::draw_meshes_for_depth_peel(&mut render_pass, engine);
            } else {
                render_pass.set_pipeline(mesh_pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                render_scene::draw_meshes_simple(&mut render_pass, engine);
            }
        }

        if use_depth_peel {
            render_peel::render_depth_peel(
                &mut encoder,
                engine,
                Some(&screenshot_view),
                options.transparency_render_passes,
            );
        }

        // Render ground plane
        let (scene_center, scene_bounds, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box,
                ctx.length_scale,
            )
        });
        let height_override = if ground_plane.height_is_relative {
            None
        } else {
            Some(ground_plane.height)
        };
        let gp_shadow_mode = match ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
        };
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            ground_plane.mode != GroundPlaneMode::None,
            scene_center,
            scene_bounds,
            length_scale,
            height_override,
            ground_plane.shadow_darkness,
            gp_shadow_mode,
            0.0,
        );

        if super::ssao_active(options, engine) {
            engine.render_ssao(&mut encoder, &options.ssao);
        }

        // Apply tone mapping
        engine.apply_screenshot_tone_mapping(&mut encoder);

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

/// Clear color of the screenshot target: the background color of the options,
/// with alpha 0 for a transparent background.
fn background_clear_color(options: &Options, transparent_background: bool) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(options.background_color.x),
        g: f64::from(options.background_color.y),
        b: f64::from(options.background_color.z),
        a: if transparent_background { 0.0 } else { 1.0 },
    }
}
