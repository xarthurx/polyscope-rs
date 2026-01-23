# Ground Plane Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add ground plane rendering with tile mode to polyscope-rs

**Architecture:** Ground plane is rendered as a fullscreen quad with vertices at infinity, using a shader that computes the intersection with an infinite horizontal plane and applies a checker pattern.

**Tech Stack:** wgpu, WGSL shaders, glam

---

## Task 1: Add Ground Plane Configuration to Core

**Files:**
- Create: `crates/polyscope-core/src/ground_plane.rs`
- Modify: `crates/polyscope-core/src/lib.rs`

**Step 1: Create ground plane configuration types**

```rust
// crates/polyscope-core/src/ground_plane.rs

/// Ground plane rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroundPlaneMode {
    /// No ground plane.
    #[default]
    None,
    /// Tiled/checkered ground plane.
    Tile,
    // Future: TileReflection, ShadowOnly
}

/// Ground plane configuration.
#[derive(Debug, Clone)]
pub struct GroundPlaneConfig {
    /// Rendering mode.
    pub mode: GroundPlaneMode,
    /// Height of the ground plane (Y coordinate).
    pub height: f32,
    /// Whether height is relative to scene bounds.
    pub height_is_relative: bool,
    /// Primary tile color.
    pub color1: [f32; 3],
    /// Secondary tile color (checker).
    pub color2: [f32; 3],
    /// Tile size (world units).
    pub tile_size: f32,
    /// Transparency (0 = opaque, 1 = fully transparent).
    pub transparency: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
            color1: [0.75, 0.75, 0.75],
            color2: [0.55, 0.55, 0.55],
            tile_size: 1.0,
            transparency: 0.0,
        }
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-core/src/lib.rs`:
```rust
mod ground_plane;
pub use ground_plane::{GroundPlaneConfig, GroundPlaneMode};
```

**Step 3: Run tests**

```bash
cargo test -p polyscope-core
```

**Step 4: Commit**

```bash
git add crates/polyscope-core/src/ground_plane.rs crates/polyscope-core/src/lib.rs
git commit -m "feat(core): add ground plane configuration types"
```

---

## Task 2: Create Ground Plane WGSL Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/ground_plane.wgsl`

**Step 1: Create the ground plane shader**

```wgsl
// Ground plane shader with checker pattern
// Uses ray-plane intersection to render infinite ground

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct GroundUniforms {
    color1: vec4<f32>,
    color2: vec4<f32>,
    height: f32,
    tile_size: f32,
    transparency: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_ray_dir: vec3<f32>,
}

// Fullscreen triangle vertices
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.999, 1.0);

    // Compute world-space ray direction from camera through this pixel
    let clip_pos = vec4<f32>(x, y, 1.0, 1.0);
    let world_pos = camera.inv_view_proj * clip_pos;
    let world_pos3 = world_pos.xyz / world_pos.w;
    out.world_ray_dir = normalize(world_pos3 - camera.camera_pos.xyz);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_origin = camera.camera_pos.xyz;
    let ray_dir = normalize(in.world_ray_dir);

    // Intersect ray with horizontal plane at y = height
    // Ray: P = O + t*D
    // Plane: y = height
    // Solve: O.y + t*D.y = height
    // t = (height - O.y) / D.y

    let denom = ray_dir.y;
    if (abs(denom) < 0.0001) {
        // Ray parallel to plane
        discard;
    }

    let t = (ground.height - ray_origin.y) / denom;
    if (t < 0.0) {
        // Intersection behind camera
        discard;
    }

    let hit_point = ray_origin + t * ray_dir;

    // Checker pattern
    let grid_x = floor(hit_point.x / ground.tile_size);
    let grid_z = floor(hit_point.z / ground.tile_size);
    let checker = ((i32(grid_x) + i32(grid_z)) % 2 + 2) % 2; // Handle negative

    var color: vec3<f32>;
    if (checker == 0) {
        color = ground.color1.rgb;
    } else {
        color = ground.color2.rgb;
    }

    // Fade out with distance for anti-aliasing at horizon
    let dist = length(hit_point - ray_origin);
    let fade = exp(-dist * 0.001);
    let alpha = (1.0 - ground.transparency) * fade;

    return vec4<f32>(color, alpha);
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/ground_plane.wgsl
git commit -m "feat(render): add ground plane WGSL shader"
```

---

## Task 3: Create Ground Plane Render Data and Pipeline

**Files:**
- Create: `crates/polyscope-render/src/ground_plane_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create ground plane render data**

```rust
// crates/polyscope-render/src/ground_plane_render.rs

use wgpu::util::DeviceExt;
use polyscope_core::{GroundPlaneConfig, GroundPlaneMode};

/// GPU representation of ground plane uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundPlaneUniforms {
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub height: f32,
    pub tile_size: f32,
    pub transparency: f32,
    pub _padding: f32,
}

impl Default for GroundPlaneUniforms {
    fn default() -> Self {
        Self {
            color1: [0.75, 0.75, 0.75, 1.0],
            color2: [0.55, 0.55, 0.55, 1.0],
            height: 0.0,
            tile_size: 1.0,
            transparency: 0.0,
            _padding: 0.0,
        }
    }
}

impl From<&GroundPlaneConfig> for GroundPlaneUniforms {
    fn from(config: &GroundPlaneConfig) -> Self {
        Self {
            color1: [config.color1[0], config.color1[1], config.color1[2], 1.0],
            color2: [config.color2[0], config.color2[1], config.color2[2], 1.0],
            height: config.height,
            tile_size: config.tile_size,
            transparency: config.transparency,
            _padding: 0.0,
        }
    }
}

/// Ground plane render resources.
pub struct GroundPlaneRenderData {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GroundPlaneRenderData {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        ground_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let uniforms = GroundPlaneUniforms::default();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ground Plane Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ground Plane Bind Group"),
            layout: ground_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            uniform_buffer,
            bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, config: &GroundPlaneConfig, scene_min_y: f32) {
        let mut uniforms = GroundPlaneUniforms::from(config);

        // If height is relative, place below scene
        if config.height_is_relative {
            uniforms.height = scene_min_y - 0.5 * config.tile_size;
        }

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:
```rust
mod ground_plane_render;
pub use ground_plane_render::{GroundPlaneRenderData, GroundPlaneUniforms};
```

**Step 3: Run tests**

```bash
cargo test -p polyscope-render
```

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/ground_plane_render.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add ground plane render data"
```

---

## Task 4: Add Ground Plane Pipeline to RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add ground plane pipeline fields**

Add to `RenderEngine` struct:
```rust
ground_plane_pipeline: wgpu::RenderPipeline,
ground_plane_bind_group_layout: wgpu::BindGroupLayout,
ground_plane_render_data: Option<GroundPlaneRenderData>,
```

**Step 2: Create ground plane bind group layout and pipeline in `new()`**

```rust
// Ground plane bind group layout
let ground_plane_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("Ground Plane Bind Group Layout"),
    entries: &[
        // Camera uniforms
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Ground uniforms
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
});

// Ground plane shader
let ground_plane_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Ground Plane Shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ground_plane.wgsl").into()),
});

// Ground plane pipeline layout
let ground_plane_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Ground Plane Pipeline Layout"),
    bind_group_layouts: &[&ground_plane_bind_group_layout],
    push_constant_ranges: &[],
});

// Ground plane render pipeline
let ground_plane_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Ground Plane Pipeline"),
    layout: Some(&ground_plane_pipeline_layout),
    vertex: wgpu::VertexState {
        module: &ground_plane_shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
        module: &ground_plane_shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: Default::default(),
    }),
    primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
    },
    depth_stencil: Some(wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: false,  // Don't write depth
        depth_compare: wgpu::CompareFunction::Always,  // Always draw
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }),
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
    cache: None,
});
```

**Step 3: Add ground plane rendering method**

```rust
pub fn render_ground_plane(
    &mut self,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    config: &GroundPlaneConfig,
    scene_min_y: f32,
) {
    if config.mode == GroundPlaneMode::None {
        return;
    }

    // Initialize render data if needed
    if self.ground_plane_render_data.is_none() {
        self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
            &self.device,
            &self.camera_bind_group_layout,
            &self.ground_plane_bind_group_layout,
            &self.camera_buffer,
        ));
    }

    if let Some(render_data) = &self.ground_plane_render_data {
        render_data.update(&self.queue, config, scene_min_y);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ground Plane Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,  // Preserve existing content
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.ground_plane_pipeline);
        render_pass.set_bind_group(0, render_data.bind_group(), &[]);
        render_pass.draw(0..3, 0..1);  // Fullscreen triangle
    }
}
```

**Step 4: Run tests**

```bash
cargo test -p polyscope-render
```

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add ground plane pipeline to engine"
```

---

## Task 5: Add Ground Plane UI Controls

**Files:**
- Modify: `crates/polyscope-ui/src/lib.rs`

**Step 1: Add ground plane UI builder function**

```rust
/// Builds UI for ground plane settings.
pub fn build_ground_plane_ui(
    ui: &mut egui::Ui,
    mode: &mut u32,  // 0=None, 1=Tile
    height: &mut f32,
    height_is_relative: &mut bool,
    color1: &mut [f32; 3],
    color2: &mut [f32; 3],
    tile_size: &mut f32,
    transparency: &mut f32,
) -> bool {
    let mut changed = false;

    // Mode selector
    egui::ComboBox::from_label("Mode")
        .selected_text(match *mode {
            0 => "None",
            _ => "Tile",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(mode, 0, "None").changed() {
                changed = true;
            }
            if ui.selectable_value(mode, 1, "Tile").changed() {
                changed = true;
            }
        });

    if *mode > 0 {
        ui.separator();

        // Height
        if ui.checkbox(height_is_relative, "Auto height").changed() {
            changed = true;
        }

        if !*height_is_relative {
            ui.horizontal(|ui| {
                ui.label("Height:");
                if ui.add(egui::DragValue::new(height).speed(0.1)).changed() {
                    changed = true;
                }
            });
        }

        ui.separator();

        // Colors
        ui.horizontal(|ui| {
            ui.label("Color 1:");
            if ui.color_edit_button_rgb(color1).changed() {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Color 2:");
            if ui.color_edit_button_rgb(color2).changed() {
                changed = true;
            }
        });

        // Tile size
        ui.horizontal(|ui| {
            ui.label("Tile size:");
            if ui.add(egui::DragValue::new(tile_size).speed(0.1).range(0.1..=100.0)).changed() {
                changed = true;
            }
        });

        // Transparency
        ui.horizontal(|ui| {
            ui.label("Opacity:");
            if ui.add(egui::Slider::new(transparency, 0.0..=1.0)).changed() {
                // Note: we're using transparency internally, so invert
                changed = true;
            }
        });
    }

    changed
}
```

**Step 2: Run tests**

```bash
cargo test -p polyscope-ui
```

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add ground plane UI controls"
```

---

## Task 6: Integrate Ground Plane into Main Crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs`
- Modify: `crates/polyscope/src/context.rs` (if exists, or wherever Context is defined)

**Step 1: Add ground plane config to Context**

Add to Context struct:
```rust
pub ground_plane: GroundPlaneConfig,
```

Initialize in `Context::new()`:
```rust
ground_plane: GroundPlaneConfig::default(),
```

**Step 2: Add ground plane rendering in main loop**

In the render function, after rendering structures but before UI:
```rust
// Render ground plane
let scene_min_y = self.compute_scene_min_y();
self.engine.render_ground_plane(
    encoder,
    view,
    depth_view,
    &self.ground_plane,
    scene_min_y,
);
```

**Step 3: Add ground plane UI in main UI panel**

Add a collapsing header for ground plane settings:
```rust
egui::CollapsingHeader::new("Ground Plane")
    .default_open(false)
    .show(ui, |ui| {
        let mut mode = match ctx.ground_plane.mode {
            GroundPlaneMode::None => 0,
            GroundPlaneMode::Tile => 1,
        };
        let mut transparency = 1.0 - ctx.ground_plane.transparency; // UI shows opacity

        if polyscope_ui::build_ground_plane_ui(
            ui,
            &mut mode,
            &mut ctx.ground_plane.height,
            &mut ctx.ground_plane.height_is_relative,
            &mut ctx.ground_plane.color1,
            &mut ctx.ground_plane.color2,
            &mut ctx.ground_plane.tile_size,
            &mut transparency,
        ) {
            ctx.ground_plane.mode = match mode {
                0 => GroundPlaneMode::None,
                _ => GroundPlaneMode::Tile,
            };
            ctx.ground_plane.transparency = 1.0 - transparency;
        }
    });
```

**Step 4: Re-export types from main crate**

Add to `crates/polyscope/src/lib.rs`:
```rust
pub use polyscope_core::{GroundPlaneConfig, GroundPlaneMode};
```

**Step 5: Run full build and tests**

```bash
cargo build
cargo test
```

**Step 6: Commit**

```bash
git add crates/polyscope/src/
git commit -m "feat: integrate ground plane into main crate"
```

---

## Task 7: Add Example and Final Testing

**Files:**
- Modify: `examples/surface_mesh_demo.rs` (or create new ground plane example)

**Step 1: Update example to demonstrate ground plane**

Add after mesh registration:
```rust
// Enable ground plane
polyscope::set_ground_plane_mode(polyscope::GroundPlaneMode::Tile);
```

Or if using context directly:
```rust
polyscope::with_context_mut(|ctx| {
    ctx.ground_plane.mode = polyscope::GroundPlaneMode::Tile;
});
```

**Step 2: Run example and verify visually**

```bash
cargo run --example surface_mesh_demo
```

Verify:
- Ground plane appears below the mesh
- Checker pattern is visible
- UI controls work (can toggle mode, change colors, adjust height)
- Ground plane fades at horizon

**Step 3: Run full test suite**

```bash
cargo test
cargo clippy
```

**Step 4: Final commit**

```bash
git add examples/
git commit -m "docs: add ground plane to surface mesh example"
```

---

## Summary

After completing all tasks:
- Ground plane configuration in polyscope-core
- WGSL shader with ray-plane intersection and checker pattern
- GPU render data and pipeline in polyscope-render
- UI controls in polyscope-ui
- Full integration in main polyscope crate
- Example demonstrating the feature

Future enhancements (not in this plan):
- Shadow mode (requires shadow mapping)
- Reflection mode (requires scene re-render with stencil)
