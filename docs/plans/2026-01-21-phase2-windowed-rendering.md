# Phase 2: Windowed Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the windowed event loop and basic rendering so that `polyscope::show()` opens a window and renders a colored background.

**Architecture:** Create a winit event loop that creates a window, initializes the wgpu render engine, and runs a frame loop that clears the screen. The RenderEngine will be stored in the global Context. Camera controls (orbit/pan/zoom) will respond to mouse input.

**Tech Stack:** winit 0.30, wgpu 24, pollster (for async runtime)

---

## Task 1: Add pollster dependency for async runtime

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/polyscope/Cargo.toml`

**Step 1: Add pollster to workspace dependencies**

In `Cargo.toml`, add to `[workspace.dependencies]`:
```toml
pollster = "0.4"
```

**Step 2: Add pollster to polyscope crate**

In `crates/polyscope/Cargo.toml`, add to `[dependencies]`:
```toml
pollster.workspace = true
```

**Step 3: Verify compilation**

Run: `cargo check -p polyscope`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add Cargo.toml crates/polyscope/Cargo.toml
git commit -m "deps: add pollster for async runtime"
```

---

## Task 2: Add RenderEngine to global Context

**Files:**
- Modify: `crates/polyscope-core/src/state.rs`
- Modify: `crates/polyscope-core/Cargo.toml`

**Step 1: Add polyscope-render dependency to polyscope-core**

In `crates/polyscope-core/Cargo.toml`, add:
```toml
polyscope-render = { workspace = true, optional = true }
```

And add a feature:
```toml
[features]
default = []
render = ["polyscope-render"]
```

**Step 2: Update Context to hold optional RenderEngine reference**

Actually, due to circular dependency issues, we'll store the engine in the main polyscope crate instead. Skip this task - the engine will be managed in the show() function directly.

**Step 3: Commit**

Skip - no changes needed.

---

## Task 3: Implement basic show() with window creation

**Files:**
- Modify: `crates/polyscope/src/lib.rs`
- Create: `crates/polyscope/src/app.rs`

**Step 1: Create app.rs with Application struct**

Create `crates/polyscope/src/app.rs`:

```rust
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
                if event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
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
```

**Step 2: Update lib.rs to use app module**

Add to `crates/polyscope/src/lib.rs` near the top:
```rust
mod app;
```

Update the `show()` function:
```rust
/// Shows the polyscope viewer window.
///
/// This function blocks until the window is closed.
pub fn show() {
    env_logger::init();
    app::run_app();
}
```

**Step 3: Verify compilation**

Run: `cargo check -p polyscope`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs crates/polyscope/src/lib.rs
git commit -m "feat: implement basic show() with window creation and rendering"
```

---

## Task 4: Write basic integration test

**Files:**
- Create: `tests/basics_test.rs`

**Step 1: Create the test file**

Create `tests/basics_test.rs`:

```rust
//! Basic integration tests for polyscope-rs.
//!
//! Note: Tests that require a window (show()) are marked #[ignore]
//! and should be run manually with: cargo test -- --ignored

use polyscope::*;

#[test]
fn test_init_and_shutdown() {
    init().expect("init failed");
    assert!(is_initialized());
    shutdown();
}

#[test]
fn test_register_point_cloud() {
    init().expect("init failed");

    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let _pc = register_point_cloud("test cloud", points);

    assert!(get_point_cloud("test cloud").is_some());
    assert!(get_point_cloud("nonexistent").is_none());

    shutdown();
}

#[test]
fn test_register_surface_mesh() {
    init().expect("init failed");

    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![glam::UVec3::new(0, 1, 2)];
    let _mesh = register_surface_mesh("test mesh", verts, faces);

    assert!(get_surface_mesh("test mesh").is_some());
    assert!(get_surface_mesh("nonexistent").is_none());

    shutdown();
}

#[test]
fn test_remove_structure() {
    init().expect("init failed");

    let points = vec![Vec3::new(0.0, 0.0, 0.0)];
    register_point_cloud("to_remove", points);

    assert!(get_point_cloud("to_remove").is_some());

    remove_structure("to_remove");

    assert!(get_point_cloud("to_remove").is_none());

    shutdown();
}

#[test]
fn test_remove_all_structures() {
    init().expect("init failed");

    let points = vec![Vec3::new(0.0, 0.0, 0.0)];
    register_point_cloud("cloud1", points.clone());
    register_point_cloud("cloud2", points);

    remove_all_structures();

    assert!(get_point_cloud("cloud1").is_none());
    assert!(get_point_cloud("cloud2").is_none());

    shutdown();
}

#[test]
fn test_point_cloud_quantities() {
    init().expect("init failed");

    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let pc = register_point_cloud("with_quantities", points);

    pc.add_scalar_quantity("scalars", vec![0.0, 0.5, 1.0]);
    pc.add_vector_quantity("vectors", vec![Vec3::X, Vec3::Y, Vec3::Z]);
    pc.add_color_quantity("colors", vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)]);

    shutdown();
}

/// This test requires a display and opens a window.
/// Run with: cargo test test_show_window -- --ignored
#[test]
#[ignore]
fn test_show_window() {
    init().expect("init failed");

    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    register_point_cloud("test points", points);

    show();

    shutdown();
}
```

**Step 2: Run the non-windowed tests**

Run: `cargo test --test basics_test`
Expected: All tests pass (except ignored ones)

**Step 3: Commit**

```bash
git add tests/basics_test.rs
git commit -m "test: add basic integration tests"
```

---

## Task 5: Create a demo example

**Files:**
- Create: `examples/demo.rs`

**Step 1: Create demo example**

Create `examples/demo.rs`:

```rust
//! Demo application showing basic polyscope-rs usage.

use polyscope::*;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Create a simple point cloud (cube corners)
    let points = vec![
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    let pc = register_point_cloud("cube corners", points);

    // Add scalar quantity (height)
    pc.add_scalar_quantity("height", vec![-1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0]);

    // Add color quantity
    pc.add_color_quantity("colors", vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(1.0, 1.0, 1.0),
    ]);

    // Create a simple triangle mesh
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
    ];
    let faces = vec![glam::UVec3::new(0, 1, 2)];

    register_surface_mesh("triangle", verts, faces);

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
```

**Step 2: Run the demo**

Run: `cargo run --example demo`
Expected: Window opens with dark background, mouse controls camera

**Step 3: Commit**

```bash
git add examples/demo.rs
git commit -m "examples: add demo application"
```

---

## Task 6: Final verification and push

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No errors (warnings acceptable for now)

**Step 3: Push to remote**

```bash
git push origin master
```

---

## Summary

After completing Phase 2, you will have:

1. A working `show()` function that opens a window
2. Basic camera controls (orbit, pan, zoom with mouse)
3. Clear screen rendering with dark background
4. Basic integration tests
5. A demo example application

The window will not yet render any geometry (points, meshes) - that will be Phase 3 where we implement the point sphere shader and actual structure rendering.
