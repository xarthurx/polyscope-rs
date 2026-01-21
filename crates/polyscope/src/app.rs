//! Application window and event loop management.

use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;
use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use polyscope_render::RenderEngine;
use polyscope_structures::PointCloud;
use polyscope_ui::EguiIntegration;

use crate::Vec3;

/// The polyscope application state.
pub struct App {
    window: Option<Arc<Window>>,
    engine: Option<RenderEngine>,
    egui: Option<EguiIntegration>,
    close_requested: bool,
    background_color: Vec3,
    // Mouse state for camera control
    mouse_pos: (f64, f64),
    mouse_down: bool,
    right_mouse_down: bool,
}

impl App {
    /// Creates a new application.
    pub fn new() -> Self {
        Self {
            window: None,
            engine: None,
            egui: None,
            close_requested: false,
            background_color: Vec3::new(0.1, 0.1, 0.1),
            mouse_pos: (0.0, 0.0),
            mouse_down: false,
            right_mouse_down: false,
        }
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Vec3) {
        self.background_color = color;
    }

    /// Renders a single frame.
    fn render(&mut self) {
        let (Some(engine), Some(egui), Some(window)) =
            (&mut self.engine, &mut self.egui, &self.window)
        else {
            return;
        };

        let Some(surface) = &engine.surface else {
            return;
        };

        // Update camera uniforms
        engine.update_camera_uniforms();

        // Initialize GPU resources for any uninitialized point clouds and vector quantities
        crate::with_context_mut(|ctx| {
            for structure in ctx.registry.iter_mut() {
                if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                        // Initialize point cloud render data
                        if pc.render_data().is_none() {
                            pc.init_gpu_resources(
                                &engine.device,
                                engine.point_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }

                        // Initialize vector quantity render data if enabled
                        let points = pc.points().to_vec();
                        if let Some(vq) = pc.active_vector_quantity_mut() {
                            if vq.render_data().is_none() {
                                vq.init_gpu_resources(
                                    &engine.device,
                                    engine.vector_bind_group_layout(),
                                    engine.camera_buffer(),
                                    &points,
                                );
                            }
                        }
                    }
                }
            }
        });

        // Update GPU buffers for point clouds and vector quantities
        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                        pc.update_gpu_buffers(&engine.queue, &engine.color_maps);

                        // Update vector quantity uniforms
                        if let Some(vq) = pc.active_vector_quantity() {
                            vq.update_uniforms(&engine.queue);
                        }
                    }
                }
            }
        });

        // Begin egui frame
        egui.begin_frame(window);

        // Build UI
        let mut bg_color = [
            self.background_color.x,
            self.background_color.y,
            self.background_color.z,
        ];

        polyscope_ui::build_left_panel(&egui.context, |ui| {
            polyscope_ui::build_controls_section(ui, &mut bg_color);

            // Collect structure info
            let structures: Vec<(String, String, bool)> = crate::with_context(|ctx| {
                ctx.registry
                    .iter()
                    .map(|s| (s.type_name().to_string(), s.name().to_string(), s.is_enabled()))
                    .collect()
            });

            polyscope_ui::build_structure_tree(ui, &structures, |type_name, name, enabled| {
                crate::with_context_mut(|ctx| {
                    if let Some(s) = ctx.registry.get_mut(type_name, name) {
                        s.set_enabled(enabled);
                    }
                });
            });
        });

        // Update background color if changed
        self.background_color = Vec3::new(bg_color[0], bg_color[1], bg_color[2]);

        // End egui frame
        let egui_output = egui.end_frame(window);

        let output = match surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                engine.resize(engine.width, engine.height);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of memory");
                self.close_requested = true;
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Surface timeout");
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background_color.x as f64,
                            g: self.background_color.y as f64,
                            b: self.background_color.z as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &engine.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw point clouds
            if let Some(pipeline) = &engine.point_pipeline {
                render_pass.set_pipeline(pipeline);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(render_data) = pc.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    // 6 vertices per quad, num_points instances
                                    render_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
                        }
                    }
                });
            }

            // Draw vector quantities
            if let Some(pipeline) = &engine.vector_pipeline {
                render_pass.set_pipeline(pipeline);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(vq) = pc.active_vector_quantity() {
                                    if let Some(render_data) = vq.render_data() {
                                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                        // 8 segments * 6 vertices = 48 vertices per arrow
                                        render_pass.draw(0..48, 0..render_data.num_vectors);
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        // Render egui on top
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
            egui_output,
        );

        engine.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("polyscope-rs")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        // Create render engine
        let engine = RenderEngine::new_windowed(window.clone())
            .block_on()
            .expect("failed to create render engine");

        // Create egui integration
        let egui = EguiIntegration::new(
            &engine.device,
            engine.surface_config.format,
            &window,
        );

        self.window = Some(window);
        self.engine = Some(engine);
        self.egui = Some(egui);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // Let egui handle events first
        if let (Some(egui), Some(window)) = (&mut self.egui, &self.window) {
            if egui.handle_event(window, &event) {
                return; // egui consumed the event
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                if let Some(engine) = &mut self.engine {
                    engine.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let delta_x = position.x - self.mouse_pos.0;
                let delta_y = position.y - self.mouse_pos.1;
                self.mouse_pos = (position.x, position.y);

                if let Some(engine) = &mut self.engine {
                    if self.mouse_down {
                        // Orbit camera
                        engine.camera.orbit(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                    } else if self.right_mouse_down {
                        // Pan camera
                        let scale = engine.camera.position.distance(engine.camera.target) * 0.002;
                        engine.camera.pan(-delta_x as f32 * scale, delta_y as f32 * scale);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.mouse_down = pressed,
                    MouseButton::Right => self.right_mouse_down = pressed,
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(engine) = &mut self.engine {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                    };
                    let scale = engine.camera.position.distance(engine.camera.target) * 0.1;
                    engine.camera.zoom(scroll * scale);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key
                    == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
                    && event.state == ElementState::Pressed
                {
                    self.close_requested = true;
                }
            }
            _ => {}
        }

        if self.close_requested {
            event_loop.exit();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the polyscope application.
pub fn run_app() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("event loop error");
}
