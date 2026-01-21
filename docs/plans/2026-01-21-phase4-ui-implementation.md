# Phase 4: UI Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add egui-based UI with structure tree, quantity controls, and element picking.

**Architecture:** Integrate egui into the wgpu render loop using egui-winit for input and egui-wgpu for rendering. UI panels overlay the 3D viewport. Pick buffer uses a separate render pass with color-encoded element IDs.

**Tech Stack:** egui 0.31, egui-wgpu 0.31, egui-winit 0.31, wgpu 24

**Prerequisites:** Phase 3 must be complete (point cloud rendering working).

---

## Task 1: Update Dependencies for egui

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/polyscope-ui/Cargo.toml`

**Step 1: Update workspace Cargo.toml**

Replace the dear-imgui dependencies with egui:

```toml
# Remove these lines:
# dear-imgui-rs = "0.7"
# dear-imgui-wgpu = "0.7"
# dear-imgui-winit = "0.7"
# dear-imguizmo = "0.7"
# dear-implot = "0.7"

# Add these lines:
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

Re-enable polyscope-ui in workspace members:
```toml
members = [
    "crates/polyscope-core",
    "crates/polyscope-render",
    "crates/polyscope-ui",  # Re-enabled
    "crates/polyscope-structures",
    "crates/polyscope",
]
```

**Step 2: Rewrite polyscope-ui/Cargo.toml**

```toml
[package]
name = "polyscope-ui"
description = "UI layer for polyscope-rs: egui integration and widgets"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[dependencies]
polyscope-core.workspace = true
polyscope-render.workspace = true
glam.workspace = true
egui.workspace = true
log.workspace = true

[dev-dependencies]
proptest.workspace = true

[lints]
workspace = true
```

**Step 3: Re-enable polyscope-ui in main crate**

In `crates/polyscope/Cargo.toml`, uncomment:
```toml
polyscope-ui.workspace = true
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles (polyscope-ui will have empty lib.rs warnings)

**Step 5: Commit**

```bash
git add Cargo.toml crates/polyscope-ui/Cargo.toml crates/polyscope/Cargo.toml
git commit -m "deps: switch from dear-imgui to egui"
```

---

## Task 2: Create egui Integration Module

**Files:**
- Create: `crates/polyscope-ui/src/lib.rs`
- Create: `crates/polyscope-ui/src/integration.rs`

**Step 1: Create the integration module**

`crates/polyscope-ui/src/integration.rs`:
```rust
//! egui integration with wgpu and winit.

use egui::Context;
use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::ScreenDescriptor;
use egui_winit::State as EguiWinitState;
use winit::event::WindowEvent;
use winit::window::Window;

/// Manages egui state and rendering.
pub struct EguiIntegration {
    pub context: Context,
    pub state: EguiWinitState,
    pub renderer: EguiRenderer,
}

impl EguiIntegration {
    /// Creates a new egui integration.
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let context = Context::default();

        // Configure dark theme
        context.set_visuals(egui::Visuals::dark());

        let viewport_id = context.viewport_id();
        let state = EguiWinitState::new(
            context.clone(),
            viewport_id,
            window,
            None,
            None,
            None,
        );

        let renderer = EguiRenderer::new(device, output_format, None, 1, false);

        Self {
            context,
            state,
            renderer,
        }
    }

    /// Handles a winit window event.
    /// Returns true if egui consumed the event.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Begins a new frame.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.context.begin_frame(raw_input);
    }

    /// Ends the frame and returns paint jobs.
    pub fn end_frame(&mut self, window: &Window) -> egui::FullOutput {
        let output = self.context.end_frame();
        self.state.handle_platform_output(window, output.platform_output.clone());
        output
    }

    /// Renders egui to the given render pass.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen_descriptor: ScreenDescriptor,
        output: egui::FullOutput,
    ) {
        let paint_jobs = self.context.tessellate(output.shapes, output.pixels_per_point);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,  // Don't clear - render on top
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
```

**Step 2: Create lib.rs**

`crates/polyscope-ui/src/lib.rs`:
```rust
//! UI layer for polyscope-rs using egui.

pub mod integration;

pub use integration::EguiIntegration;
```

**Step 3: Verify it compiles**

Run: `cargo check -p polyscope-ui`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/
git commit -m "feat: add egui integration module"
```

---

## Task 3: Integrate egui into App Render Loop

**Files:**
- Modify: `crates/polyscope/Cargo.toml`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add egui dependencies to polyscope crate**

In `crates/polyscope/Cargo.toml`, add:
```toml
egui.workspace = true
egui-wgpu.workspace = true
egui-winit.workspace = true
```

**Step 2: Update App struct**

In `app.rs`, add egui state:
```rust
use egui_wgpu::ScreenDescriptor;
use polyscope_ui::EguiIntegration;

pub struct App {
    window: Option<Arc<Window>>,
    engine: Option<RenderEngine>,
    egui: Option<EguiIntegration>,  // Add this
    close_requested: bool,
    background_color: Vec3,
    mouse_pos: (f64, f64),
    mouse_down: bool,
    right_mouse_down: bool,
}
```

Initialize `egui: None` in `App::new()`.

**Step 3: Initialize egui in resumed()**

After creating the render engine:
```rust
fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    // ... existing window and engine creation ...

    let egui = EguiIntegration::new(
        &engine.device,
        engine.surface_config.format,
        &window,
    );

    self.window = Some(window);
    self.engine = Some(engine);
    self.egui = Some(egui);
}
```

**Step 4: Handle egui events**

In `window_event()`, before matching on the event:
```rust
fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
    // Let egui handle events first
    if let (Some(egui), Some(window)) = (&mut self.egui, &self.window) {
        if egui.handle_event(window, &event) {
            return;  // egui consumed the event
        }
    }

    match event {
        // ... existing event handling ...
    }
}
```

**Step 5: Update render() to include egui**

```rust
fn render(&mut self) {
    let Some(engine) = &mut self.engine else { return };
    let Some(surface) = &engine.surface else { return };
    let Some(egui) = &mut self.egui else { return };
    let Some(window) = &self.window else { return };

    // Update camera uniforms
    engine.update_camera_uniforms();

    // Begin egui frame
    egui.begin_frame(window);

    // Build UI
    egui::SidePanel::left("main_panel")
        .default_width(305.0)
        .show(&egui.context, |ui| {
            ui.heading("polyscope-rs");
            ui.separator();
            ui.label("Structures will appear here");
        });

    // End egui frame
    let egui_output = egui.end_frame(window);

    let output = match surface.get_current_texture() {
        Ok(output) => output,
        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
            engine.resize(engine.width, engine.height);
            return;
        }
        Err(e) => {
            log::error!("Surface error: {:?}", e);
            return;
        }
    };

    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render encoder"),
    });

    // Render 3D scene
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

        // Draw point clouds (from Phase 3)
        // ... existing drawing code ...
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
        screen_descriptor,
        egui_output,
    );

    engine.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}
```

**Step 6: Verify it compiles and runs**

Run: `cargo run --example demo -p polyscope`
Expected: Window opens with left panel showing "polyscope-rs" heading

**Step 7: Commit**

```bash
git add crates/polyscope/
git commit -m "feat: integrate egui into render loop"
```

---

## Task 4: Create Structure Tree UI

**Files:**
- Create: `crates/polyscope-ui/src/panels.rs`
- Modify: `crates/polyscope-ui/src/lib.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Create panels module**

`crates/polyscope-ui/src/panels.rs`:
```rust
//! UI panel builders.

use egui::{CollapsingHeader, Context, SidePanel, Ui};

/// Builds the main left panel.
pub fn build_left_panel(ctx: &Context, build_contents: impl FnOnce(&mut Ui)) {
    SidePanel::left("polyscope_main_panel")
        .default_width(305.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("polyscope-rs");
            ui.separator();
            build_contents(ui);
        });
}

/// Builds the polyscope controls section.
pub fn build_controls_section(ui: &mut Ui, background_color: &mut [f32; 3]) {
    CollapsingHeader::new("View")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Background:");
                ui.color_edit_button_rgb(background_color);
            });

            if ui.button("Reset View").clicked() {
                // TODO: Reset camera
            }
        });
}

/// Builds the structure tree section.
pub fn build_structure_tree<F>(
    ui: &mut Ui,
    structures: &[(String, String, bool)],  // (type_name, name, enabled)
    mut on_toggle: F,
)
where
    F: FnMut(&str, &str, bool),  // (type_name, name, new_enabled)
{
    CollapsingHeader::new("Structures")
        .default_open(true)
        .show(ui, |ui| {
            if structures.is_empty() {
                ui.label("No structures registered");
                return;
            }

            // Group by type
            let mut by_type: std::collections::HashMap<&str, Vec<(&str, bool)>> =
                std::collections::HashMap::new();
            for (type_name, name, enabled) in structures {
                by_type
                    .entry(type_name.as_str())
                    .or_default()
                    .push((name.as_str(), *enabled));
            }

            for (type_name, instances) in &by_type {
                let header = format!("{} ({})", type_name, instances.len());
                CollapsingHeader::new(header)
                    .default_open(instances.len() <= 8)
                    .show(ui, |ui| {
                        for (name, enabled) in instances {
                            let mut enabled_mut = *enabled;
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut enabled_mut, "").changed() {
                                    on_toggle(type_name, name, enabled_mut);
                                }
                                ui.label(*name);
                            });
                        }
                    });
            }
        });
}
```

**Step 2: Update lib.rs**

```rust
//! UI layer for polyscope-rs using egui.

pub mod integration;
pub mod panels;

pub use integration::EguiIntegration;
pub use panels::*;
```

**Step 3: Use structure tree in app.rs**

Update the UI building section in `render()`:
```rust
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
            .structures()
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
```

**Step 4: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Structure tree shows registered point clouds with enable checkboxes

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/ crates/polyscope/src/app.rs
git commit -m "feat: add structure tree UI panel"
```

---

## Task 5: Add Structure-Specific UI

**Files:**
- Create: `crates/polyscope-ui/src/structure_ui.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/mod.rs`
- Modify: `crates/polyscope-core/src/structure.rs`

**Step 1: Add UI trait to Structure**

In `crates/polyscope-core/src/structure.rs`, update the trait:
```rust
/// Builds the egui UI for this structure.
/// The `ui` parameter is `&mut egui::Ui` but typed as Any for crate independence.
fn build_egui_ui(&mut self, ui: &mut dyn std::any::Any) {
    // Default: no UI
    let _ = ui;
}
```

**Step 2: Create structure_ui module**

`crates/polyscope-ui/src/structure_ui.rs`:
```rust
//! Structure-specific UI builders.

use egui::Ui;
use glam::Vec3;

/// Builds UI for a point cloud.
pub fn build_point_cloud_ui(
    ui: &mut Ui,
    num_points: usize,
    point_radius: &mut f32,
    base_color: &mut [f32; 3],
) -> bool {
    let mut changed = false;

    ui.label(format!("Points: {}", num_points));

    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(base_color).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Radius:");
        if ui.add(
            egui::DragValue::new(point_radius)
                .speed(0.001)
                .range(0.001..=0.5)
        ).changed() {
            changed = true;
        }
    });

    changed
}
```

**Step 3: Implement build_egui_ui for PointCloud**

In `crates/polyscope-structures/src/point_cloud/mod.rs`:
```rust
fn build_egui_ui(&mut self, ui: &mut dyn std::any::Any) {
    let Some(ui) = ui.downcast_mut::<egui::Ui>() else { return };

    let mut color = [self.base_color.x, self.base_color.y, self.base_color.z];
    let mut radius = self.point_radius;

    if polyscope_ui::structure_ui::build_point_cloud_ui(
        ui,
        self.points.len(),
        &mut radius,
        &mut color,
    ) {
        self.base_color = Vec3::new(color[0], color[1], color[2]);
        self.point_radius = radius;
    }

    // Build quantities UI
    for quantity in &mut self.quantities {
        // TODO: quantity UI
    }
}
```

Add egui dependency to polyscope-structures:
```toml
egui.workspace = true
polyscope-ui.workspace = true
```

**Step 4: Update structure tree to show per-structure UI**

In `panels.rs`, update `build_structure_tree` to accept a UI builder callback.

**Step 5: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Clicking on a structure shows its UI controls

**Step 6: Commit**

```bash
git add crates/
git commit -m "feat: add structure-specific UI controls"
```

---

## Task 6: Add Quantity UI Controls

**Files:**
- Create: `crates/polyscope-ui/src/quantity_ui.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/quantities.rs`

**Step 1: Create quantity_ui module**

`crates/polyscope-ui/src/quantity_ui.rs`:
```rust
//! Quantity-specific UI builders.

use egui::Ui;

/// Builds UI for a scalar quantity.
pub fn build_scalar_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    colormap: &mut String,
    range_min: &mut f32,
    range_max: &mut f32,
    available_colormaps: &[&str],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            // Colormap selector
            egui::ComboBox::from_label("Colormap")
                .selected_text(colormap.as_str())
                .show_ui(ui, |ui| {
                    for &cmap in available_colormaps {
                        if ui.selectable_value(colormap, cmap.to_string(), cmap).changed() {
                            changed = true;
                        }
                    }
                });

            // Range controls
            ui.horizontal(|ui| {
                ui.label("Range:");
                if ui.add(egui::DragValue::new(range_min).speed(0.01)).changed() {
                    changed = true;
                }
                ui.label("to");
                if ui.add(egui::DragValue::new(range_max).speed(0.01)).changed() {
                    changed = true;
                }
            });
        });
    }

    changed
}

/// Builds UI for a color quantity.
pub fn build_color_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    num_colors: usize,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
        ui.label(format!("({} colors)", num_colors));
    });

    changed
}

/// Builds UI for a vector quantity.
pub fn build_vector_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    length_scale: &mut f32,
    radius: &mut f32,
    color: &mut [f32; 3],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            ui.horizontal(|ui| {
                ui.label("Length:");
                if ui.add(egui::DragValue::new(length_scale).speed(0.01).range(0.01..=5.0)).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(egui::DragValue::new(radius).speed(0.001).range(0.001..=0.1)).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_rgb(color).changed() {
                    changed = true;
                }
            });
        });
    }

    changed
}
```

**Step 2: Update lib.rs**

```rust
pub mod quantity_ui;
pub use quantity_ui::*;
```

**Step 3: Add build_egui_ui to quantity types**

In `quantities.rs`, implement for each quantity type. Example for scalar:
```rust
impl PointCloudScalarQuantity {
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let colormaps = ["viridis", "blues", "reds", "coolwarm", "rainbow"];
        polyscope_ui::build_scalar_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.colormap_name,
            &mut self.range_min,
            &mut self.range_max,
            &colormaps,
        )
    }
}
```

**Step 4: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Quantities show with enable checkbox and controls when expanded

**Step 5: Commit**

```bash
git add crates/
git commit -m "feat: add quantity UI controls"
```

---

## Task 7: Implement Pick Buffer Rendering

**Files:**
- Create: `crates/polyscope-render/src/pick.rs`
- Create: `crates/polyscope-render/src/shaders/pick.wgsl`
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Create pick shader**

`crates/polyscope-render/src/shaders/pick.wgsl`:
```wgsl
// Pick buffer shader - renders elements with unique colors encoding their ID

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    base_index: u32,
    point_radius: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick_uniforms: PickUniforms;
@group(0) @binding(2) var<storage, read> point_positions: array<vec3<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pick_color: vec3<f32>,
    @location(1) sphere_center_view: vec3<f32>,
    @location(2) quad_pos: vec2<f32>,
    @location(3) point_radius: f32,
}

const QUAD_VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

fn index_to_color(index: u32) -> vec3<f32> {
    let r = f32((index >> 16u) & 0xFFu) / 255.0;
    let g = f32((index >> 8u) & 0xFFu) / 255.0;
    let b = f32(index & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = point_positions[instance_index];
    let view_pos = (camera.view * vec4<f32>(world_pos, 1.0)).xyz;
    let quad_pos = QUAD_VERTICES[vertex_index];
    let radius = pick_uniforms.point_radius;
    let offset = vec3<f32>(quad_pos * radius, 0.0);
    let billboard_pos_view = view_pos + offset;

    out.clip_position = camera.proj * vec4<f32>(billboard_pos_view, 1.0);
    out.pick_color = index_to_color(pick_uniforms.base_index + instance_index);
    out.sphere_center_view = view_pos;
    out.quad_pos = quad_pos;
    out.point_radius = radius;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ray-sphere intersection (same as main shader)
    let ray_origin = vec3<f32>(
        in.sphere_center_view.xy + in.quad_pos * in.point_radius,
        in.sphere_center_view.z
    );
    let ray_dir = vec3<f32>(0.0, 0.0, -1.0);
    let oc = ray_origin - in.sphere_center_view;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - in.point_radius * in.point_radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        discard;
    }

    return vec4<f32>(in.pick_color, 1.0);
}
```

**Step 2: Create pick module**

`crates/polyscope-render/src/pick.rs`:
```rust
//! Pick buffer rendering for element selection.

use glam::Vec2;

/// Result of a pick operation.
#[derive(Debug, Clone, Default)]
pub struct PickResult {
    pub hit: bool,
    pub structure_type: String,
    pub structure_name: String,
    pub element_index: u64,
    pub screen_pos: Vec2,
    pub depth: f32,
}

/// Decodes a pick color back to an index.
pub fn color_to_index(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Encodes an index as a pick color.
pub fn index_to_color(index: u32) -> [u8; 3] {
    [
        ((index >> 16) & 0xFF) as u8,
        ((index >> 8) & 0xFF) as u8,
        (index & 0xFF) as u8,
    ]
}
```

**Step 3: Add to lib.rs and engine.rs**

Add pick buffer texture and pipeline to RenderEngine.

**Step 4: Commit**

```bash
git add crates/polyscope-render/
git commit -m "feat: add pick buffer rendering"
```

---

## Task 8: Implement Selection Logic

**Files:**
- Modify: `crates/polyscope/src/app.rs`
- Create: `crates/polyscope-ui/src/selection_panel.rs`

**Step 1: Add selection state to App**

```rust
pub struct App {
    // ... existing fields ...
    selection: Option<polyscope_render::pick::PickResult>,
}
```

**Step 2: Handle click for picking**

In `window_event()`, after egui handling:
```rust
WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
    // If we didn't drag much, it's a click - do picking
    if self.drag_distance < 5.0 {
        self.selection = self.pick_at_cursor();
    }
}
WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Right, .. } => {
    self.selection = None;  // Clear selection
}
```

**Step 3: Create selection panel**

`crates/polyscope-ui/src/selection_panel.rs`:
```rust
//! Selection/pick results panel.

use egui::{Context, SidePanel, Ui};

pub fn build_selection_panel(
    ctx: &Context,
    selection: &polyscope_render::pick::PickResult,
    build_structure_pick_ui: impl FnOnce(&mut Ui),
) {
    SidePanel::right("selection_panel")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Selection");
            ui.separator();

            ui.label(format!("Screen: ({:.0}, {:.0})",
                selection.screen_pos.x, selection.screen_pos.y));
            ui.label(format!("Depth: {:.4}", selection.depth));

            ui.separator();
            ui.label(format!("{}: {}",
                selection.structure_type, selection.structure_name));
            ui.label(format!("Element #{}", selection.element_index));

            ui.separator();
            build_structure_pick_ui(ui);
        });
}
```

**Step 4: Show selection panel in render loop**

```rust
if let Some(ref selection) = self.selection {
    if selection.hit {
        polyscope_ui::build_selection_panel(&egui.context, selection, |ui| {
            // Structure-specific pick UI
            crate::with_context(|ctx| {
                if let Some(s) = ctx.registry.get(&selection.structure_type, &selection.structure_name) {
                    s.build_pick_ui(ui, selection);
                }
            });
        });
    }
}
```

**Step 5: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Clicking on a point shows selection panel on right

**Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement element selection with pick buffer"
```

---

## Task 9: Final Integration Test

**Files:**
- Modify: `examples/demo.rs`

**Step 1: Update demo with full UI interaction**

```rust
use polyscope::*;

fn main() -> Result<()> {
    init()?;

    // Create a sphere of points
    let mut points = Vec::new();
    let n = 15;
    for i in 0..n {
        for j in 0..n {
            let theta = std::f32::consts::PI * i as f32 / (n - 1) as f32;
            let phi = 2.0 * std::f32::consts::PI * j as f32 / n as f32;
            let r = 0.5;
            points.push(Vec3::new(
                r * theta.sin() * phi.cos(),
                r * theta.sin() * phi.sin(),
                r * theta.cos(),
            ));
        }
    }

    let handle = register_point_cloud("sphere", points.clone());

    // Add scalar quantity (latitude)
    let scalars: Vec<f32> = points.iter().map(|p| p.z).collect();
    handle.add_scalar_quantity("latitude", scalars);

    // Add color quantity
    let colors: Vec<Vec3> = points
        .iter()
        .map(|p| Vec3::new(p.x + 0.5, p.y + 0.5, p.z + 0.5))
        .collect();
    handle.add_color_quantity("position", colors);

    // Show viewer - UI now available!
    show();

    Ok(())
}
```

**Step 2: Run and verify**

Run: `cargo run --example demo -p polyscope`
Expected:
- Left panel shows structure tree with "sphere" point cloud
- Can toggle point cloud visibility
- Can expand to see quantities
- Can enable/disable quantities
- Can adjust point radius and color
- Clicking on points shows selection panel

**Step 3: Commit**

```bash
git add examples/
git commit -m "feat: complete Phase 4 - UI integration with egui"
```

---

## Summary

Phase 4 implements:
1. **egui integration** - Pure Rust UI, no native dependencies
2. **Left panel** - Polyscope controls and structure tree
3. **Structure UI** - Per-structure controls (color, radius)
4. **Quantity UI** - Scalar (colormap, range), vector, color controls
5. **Pick buffer** - Offscreen rendering with color-encoded IDs
6. **Selection panel** - Right panel showing pick results

**Note:** This plan assumes Phase 3 is complete. The pick buffer uses the same sphere impostor geometry as the main render.
