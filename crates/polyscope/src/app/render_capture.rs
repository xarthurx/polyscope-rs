//! Screenshot capture and headless rendering.

use super::{App, GroundPlaneMode, render_scene};

impl App {
    /// Captures a screenshot by re-rendering to a dedicated texture.
    pub(super) fn capture_screenshot(&mut self, filename: String) {
        let Some(engine) = &mut self.engine else {
            log::error!("Cannot capture screenshot: engine not initialized");
            return;
        };

        // Create screenshot target
        let screenshot_view = engine.create_screenshot_target();

        // Re-render to screenshot texture
        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.background_color.x),
                            g: f64::from(self.background_color.y),
                            b: f64::from(self.background_color.z),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
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

            // Draw surface meshes and volume meshes
            render_scene::draw_meshes_simple(&mut render_pass, engine);

            // Draw curve networks, camera views, and volume grids
            render_scene::draw_curve_networks_and_lines(&mut render_pass, engine);
        }

        // Render ground plane for screenshot
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });
        let height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        let screenshot_gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
        };
        let screenshot_reflection_intensity =
            if self.ground_plane.mode == GroundPlaneMode::TileReflection {
                self.ground_plane.reflection_intensity
            } else {
                0.0
            };
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            self.ground_plane.mode != GroundPlaneMode::None,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
            self.ground_plane.shadow_darkness,
            screenshot_gp_shadow_mode,
            screenshot_reflection_intensity,
        );

        // Apply tone mapping from HDR to final screenshot texture
        engine.apply_screenshot_tone_mapping(&mut encoder);

        engine.queue.submit(std::iter::once(encoder.finish()));

        // Capture the screenshot
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
    pub(crate) fn render_frame_headless(&mut self) {
        let Some(engine) = &mut self.engine else {
            return;
        };

        // Auto-fit camera to scene
        self.camera_fitted = super::render_init::auto_fit_camera(engine, self.camera_fitted);

        // Drain deferred material load queue
        super::render_init::drain_material_queue(engine);

        // Update camera and slice plane uniforms
        super::render_init::update_uniforms(engine);

        // Initialize GPU resources for all structures (shared function)
        super::render_init::init_structure_gpu_resources(engine);

        // Update GPU buffers (headless: no pick uniforms)
        super::render_init::update_gpu_buffers(engine, false);

        // Now render to screenshot target (reuses existing capture_screenshot rendering)
        self.capture_screenshot_headless();
    }

    /// Renders the scene to the screenshot target texture without saving to file.
    /// The pixel data can be retrieved via `capture_to_buffer()`.
    fn capture_screenshot_headless(&mut self) {
        let Some(engine) = &mut self.engine else {
            return;
        };

        let screenshot_view = engine.create_screenshot_target();

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless render encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.background_color.x),
                            g: f64::from(self.background_color.y),
                            b: f64::from(self.background_color.z),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
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
        // Ensure normal texture exists before borrowing mesh_pipeline
        if engine.mesh_pipeline.is_some() && engine.normal_view().is_none() {
            let (w, h) = engine.dimensions();
            engine.create_normal_texture_with_size(w, h);
        }
        if let Some(mesh_pipeline) = &engine.mesh_pipeline {
            if let Some(normal_view) = engine.normal_view() {
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

                render_pass.set_pipeline(mesh_pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                render_scene::draw_meshes_simple(&mut render_pass, engine);
            }
        }

        // Render ground plane
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });
        let height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        let gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
        };
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            self.ground_plane.mode != GroundPlaneMode::None,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
            self.ground_plane.shadow_darkness,
            gp_shadow_mode,
            0.0,
        );

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
