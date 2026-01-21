//! Application window and event loop management.

use std::sync::Arc;

use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use polyscope_render::RenderEngine;

use crate::Vec3;

/// The polyscope application state.
pub struct App {
    window: Option<Arc<Window>>,
    engine: Option<RenderEngine>,
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
        let Some(engine) = &mut self.engine else {
            return;
        };

        let Some(surface) = &engine.surface else {
            return;
        };

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

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            // TODO: Draw structures here
        }

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

        self.window = Some(window);
        self.engine = Some(engine);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
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
