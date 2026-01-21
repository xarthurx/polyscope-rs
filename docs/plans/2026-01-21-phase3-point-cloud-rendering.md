# Phase 3: Point Cloud Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Render point clouds as sphere impostors with scalar, color, and vector quantities.

**Architecture:** Use instanced rendering to draw billboard quads (one per point), with fragment shader ray-casting for sphere impostors. Quantities modify per-point colors via storage buffers. Colormaps use 1D textures for scalar-to-color mapping.

**Tech Stack:** wgpu 24, WGSL shaders, glam for math

---

## Task 1: Create Point Sphere Shader (WGSL)

**Files:**
- Create: `crates/polyscope-render/src/shaders/point_sphere.wgsl`

**Step 1: Create the WGSL shader file**

```wgsl
// Point sphere impostor shader using instanced rendering
// Each instance is a point rendered as a ray-cast sphere

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PointUniforms {
    point_radius: f32,
    use_per_point_color: u32,  // 0 = base color, 1 = per-point color
    _padding: vec2<f32>,
    base_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> point_uniforms: PointUniforms;
@group(0) @binding(2) var<storage, read> point_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read> point_colors: array<vec3<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) sphere_center_view: vec3<f32>,
    @location(1) quad_pos: vec2<f32>,  // [-1, 1] on billboard quad
    @location(2) point_color: vec3<f32>,
    @location(3) point_radius: f32,
}

// Billboard quad vertices (two triangles)
const QUAD_VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Get point position
    let world_pos = point_positions[instance_index];
    let view_pos = (camera.view * vec4<f32>(world_pos, 1.0)).xyz;

    // Get quad vertex
    let quad_pos = QUAD_VERTICES[vertex_index];

    // Compute billboard offset in view space (always facing camera)
    let radius = point_uniforms.point_radius;
    let offset = vec3<f32>(quad_pos * radius, 0.0);
    let billboard_pos_view = view_pos + offset;

    // Project to clip space
    out.clip_position = camera.proj * vec4<f32>(billboard_pos_view, 1.0);
    out.sphere_center_view = view_pos;
    out.quad_pos = quad_pos;
    out.point_radius = radius;

    // Get color
    if (point_uniforms.use_per_point_color == 1u) {
        out.point_color = point_colors[instance_index];
    } else {
        out.point_color = point_uniforms.base_color.rgb;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ray-sphere intersection in view space
    // Ray starts at fragment position on billboard, goes toward -Z (into screen)
    let ray_origin = vec3<f32>(
        in.sphere_center_view.xy + in.quad_pos * in.point_radius,
        in.sphere_center_view.z
    );
    let ray_dir = vec3<f32>(0.0, 0.0, -1.0);

    // Sphere at sphere_center_view with radius point_radius
    let oc = ray_origin - in.sphere_center_view;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - in.point_radius * in.point_radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        discard;
    }

    let t = (-b - sqrt(discriminant)) / (2.0 * a);
    let hit_point = ray_origin + t * ray_dir;
    let normal = normalize(hit_point - in.sphere_center_view);

    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.3;
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;

    let color = in.point_color * lighting;

    return vec4<f32>(color, 1.0);
}
```

**Step 2: No test needed for shader file**

Shader is verified by visual inspection in later tasks.

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/
git commit -m "shader: add point sphere impostor WGSL shader"
```

---

## Task 2: Create PointCloudRenderData Structure

**Files:**
- Create: `crates/polyscope-render/src/point_cloud_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create the render data module**

```rust
//! Point cloud GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// GPU resources for rendering a point cloud.
pub struct PointCloudRenderData {
    /// Position buffer (storage buffer).
    pub position_buffer: wgpu::Buffer,
    /// Color buffer (storage buffer).
    pub color_buffer: wgpu::Buffer,
    /// Uniform buffer for point-specific settings.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for this point cloud.
    pub bind_group: wgpu::BindGroup,
    /// Number of points.
    pub num_points: u32,
}

/// Uniforms for point cloud rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointUniforms {
    pub point_radius: f32,
    pub use_per_point_color: u32,
    pub _padding: [f32; 2],
    pub base_color: [f32; 4],
}

impl Default for PointUniforms {
    fn default() -> Self {
        Self {
            point_radius: 0.01,
            use_per_point_color: 0,
            _padding: [0.0; 2],
            base_color: [0.2, 0.5, 0.8, 1.0], // Default blue
        }
    }
}

impl PointCloudRenderData {
    /// Creates new render data from point positions.
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        positions: &[Vec3],
        colors: Option<&[Vec3]>,
    ) -> Self {
        let num_points = positions.len() as u32;

        // Create position buffer
        let position_data: Vec<f32> = positions
            .iter()
            .flat_map(|p| [p.x, p.y, p.z, 0.0]) // pad to vec4 for alignment
            .collect();
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point positions"),
            contents: bytemuck::cast_slice(&position_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create color buffer (default white if not provided)
        let color_data: Vec<f32> = if let Some(colors) = colors {
            colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect()
        } else {
            vec![1.0; positions.len() * 4]
        };
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point colors"),
            contents: bytemuck::cast_slice(&color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create uniform buffer
        let uniforms = PointUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point cloud bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: color_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            position_buffer,
            color_buffer,
            uniform_buffer,
            bind_group,
            num_points,
        }
    }

    /// Updates the color buffer.
    pub fn update_colors(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect();
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&color_data));
    }

    /// Updates uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &PointUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}
```

**Step 2: Add bytemuck dependency to polyscope-render**

In `crates/polyscope-render/Cargo.toml`, add:
```toml
bytemuck = { version = "1.14", features = ["derive"] }
```

**Step 3: Export module from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:
```rust
pub mod point_cloud_render;
pub use point_cloud_render::{PointCloudRenderData, PointUniforms};
```

**Step 4: Verify it compiles**

Run: `cargo check -p polyscope-render`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add crates/polyscope-render/
git commit -m "feat: add PointCloudRenderData for GPU resources"
```

---

## Task 3: Create Point Cloud Render Pipeline

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add camera uniform structure**

Add to `engine.rs`:
```rust
/// Camera uniforms for GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _padding: f32,
}
```

**Step 2: Add pipeline and resources to RenderEngine**

Add fields to `RenderEngine`:
```rust
pub struct RenderEngine {
    // ... existing fields ...

    /// Point cloud render pipeline.
    pub point_pipeline: Option<wgpu::RenderPipeline>,
    /// Point cloud bind group layout.
    pub point_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Camera uniform buffer.
    pub camera_buffer: wgpu::Buffer,
}
```

**Step 3: Create pipeline initialization method**

```rust
impl RenderEngine {
    /// Initializes the point cloud render pipeline.
    pub fn init_point_pipeline(&mut self) {
        let shader_source = include_str!("shaders/point_sphere.wgsl");
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point sphere shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("point cloud bind group layout"),
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
                // Point uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Position storage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Color storage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("point sphere pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Don't cull billboards
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.point_pipeline = Some(pipeline);
        self.point_bind_group_layout = Some(bind_group_layout);
    }

    /// Updates camera uniforms.
    pub fn update_camera_uniforms(&self) {
        let view = self.camera.view_matrix();
        let proj = self.camera.projection_matrix();
        let view_proj = proj * view;
        let inv_proj = proj.inverse();

        let uniforms = CameraUniforms {
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            view_proj: view_proj.to_cols_array_2d(),
            inv_proj: inv_proj.to_cols_array_2d(),
            camera_pos: self.camera.position.to_array(),
            _padding: 0.0,
        };

        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Gets the point cloud bind group layout.
    pub fn point_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.point_bind_group_layout.as_ref().expect("point pipeline not initialized")
    }

    /// Gets the camera buffer.
    pub fn camera_buffer(&self) -> &wgpu::Buffer {
        &self.camera_buffer
    }
}
```

**Step 4: Update constructors to create camera buffer and initialize pipeline**

In `new_windowed` and `new_headless`, add camera buffer creation:
```rust
let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("camera uniforms"),
    contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
});
```

Add `camera_buffer` field and initialize `point_pipeline` and `point_bind_group_layout` to `None`.

Then after creating RenderEngine, call:
```rust
engine.init_point_pipeline();
```

**Step 5: Add Default impl for CameraUniforms**

```rust
impl Default for CameraUniforms {
    fn default() -> Self {
        Self {
            view: glam::Mat4::IDENTITY.to_cols_array_2d(),
            proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 5.0],
            _padding: 0.0,
        }
    }
}
```

**Step 6: Verify it compiles**

Run: `cargo check -p polyscope-render`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add crates/polyscope-render/
git commit -m "feat: add point cloud render pipeline"
```

---

## Task 4: Integrate Point Cloud Rendering into App

**Files:**
- Modify: `crates/polyscope-structures/src/point_cloud/mod.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add render data to PointCloud**

In `point_cloud/mod.rs`, add field and imports:
```rust
use polyscope_render::{PointCloudRenderData, PointUniforms};

pub struct PointCloud {
    // ... existing fields ...
    render_data: Option<PointCloudRenderData>,
    point_radius: f32,
    base_color: Vec3,
}
```

Update `new()`:
```rust
pub fn new(name: impl Into<String>, points: Vec<Vec3>) -> Self {
    Self {
        name: name.into(),
        points,
        enabled: true,
        transform: Mat4::IDENTITY,
        quantities: Vec::new(),
        render_data: None,
        point_radius: 0.01,
        base_color: Vec3::new(0.2, 0.5, 0.8),
    }
}
```

**Step 2: Add GPU initialization method to PointCloud**

```rust
impl PointCloud {
    /// Initializes GPU resources for this point cloud.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        self.render_data = Some(PointCloudRenderData::new(
            device,
            bind_group_layout,
            bind_group_layout, // Reuse for now
            camera_buffer,
            &self.points,
            None, // No per-point colors yet
        ));
    }

    /// Returns the render data if initialized.
    pub fn render_data(&self) -> Option<&PointCloudRenderData> {
        self.render_data.as_ref()
    }

    /// Sets the point radius.
    pub fn set_point_radius(&mut self, radius: f32) {
        self.point_radius = radius;
    }

    /// Gets the point radius.
    pub fn point_radius(&self) -> f32 {
        self.point_radius
    }

    /// Sets the base color.
    pub fn set_base_color(&mut self, color: Vec3) {
        self.base_color = color;
    }
}
```

**Step 3: Update app.rs render method to draw point clouds**

In `app.rs`, modify the `render` method:
```rust
fn render(&mut self) {
    let Some(engine) = &mut self.engine else {
        return;
    };

    let Some(surface) = &engine.surface else {
        return;
    };

    // Update camera uniforms
    engine.update_camera_uniforms();

    let output = match surface.get_current_texture() {
        // ... existing error handling ...
    };

    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                for structure in ctx.registry.structures() {
                    if !structure.is_enabled() {
                        continue;
                    }
                    if structure.type_name() == "PointCloud" {
                        if let Some(pc) = (structure as &dyn std::any::Any).downcast_ref::<PointCloud>() {
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
    }

    engine.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}
```

**Step 4: Initialize GPU resources when point cloud is registered**

Update `register_point_cloud` in `lib.rs` to initialize GPU resources. This requires access to the render engine, so we need to defer initialization until the app runs. For now, add initialization in `App::resumed()` or on first render.

Alternative: Add a method to initialize all uninitialized point clouds during render:

```rust
// In app.rs, before the render pass:
crate::with_context_mut(|ctx| {
    for structure in ctx.registry.structures_mut() {
        if structure.type_name() == "PointCloud" {
            if let Some(pc) = (structure as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                if pc.render_data().is_none() {
                    pc.init_gpu_resources(
                        &engine.device,
                        engine.point_bind_group_layout(),
                        engine.camera_buffer(),
                    );
                }
            }
        }
    }
});
```

**Step 5: Verify it compiles**

Run: `cargo check -p polyscope`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add crates/
git commit -m "feat: integrate point cloud rendering into app"
```

---

## Task 5: Test Basic Point Cloud Rendering

**Files:**
- Modify: `examples/demo.rs`

**Step 1: Update demo to render visible point cloud**

```rust
use polyscope::*;

fn main() -> Result<()> {
    init()?;

    // Create a grid of points
    let mut points = Vec::new();
    for i in 0..10 {
        for j in 0..10 {
            for k in 0..10 {
                points.push(Vec3::new(
                    i as f32 * 0.1 - 0.45,
                    j as f32 * 0.1 - 0.45,
                    k as f32 * 0.1 - 0.45,
                ));
            }
        }
    }

    register_point_cloud("my points", points);

    show();

    Ok(())
}
```

**Step 2: Run the demo**

Run: `cargo run --example demo -p polyscope`
Expected: Window opens showing 1000 blue spheres in a 10x10x10 grid

**Step 3: Commit**

```bash
git add examples/
git commit -m "test: update demo with 3D point grid"
```

---

## Task 6: Implement Color Quantity Rendering

**Files:**
- Modify: `crates/polyscope-structures/src/point_cloud/quantities.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/mod.rs`

**Step 1: Add GPU update to ColorQuantity**

In `quantities.rs`, update `PointCloudColorQuantity`:
```rust
impl PointCloudColorQuantity {
    /// Applies this color quantity to the point cloud render data.
    pub fn apply_to_render_data(&self, queue: &wgpu::Queue, render_data: &PointCloudRenderData) {
        render_data.update_colors(queue, &self.colors);
    }
}
```

**Step 2: Update PointCloud to use active color quantity**

In `mod.rs`, add method to get dominant color quantity:
```rust
impl PointCloud {
    /// Returns the currently active color quantity, if any.
    pub fn active_color_quantity(&self) -> Option<&PointCloudColorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = (q.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<PointCloudColorQuantity>()
                {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Updates GPU buffers based on current state.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
        let Some(render_data) = &self.render_data else { return };

        let mut uniforms = PointUniforms {
            point_radius: self.point_radius,
            use_per_point_color: 0,
            _padding: [0.0; 2],
            base_color: [self.base_color.x, self.base_color.y, self.base_color.z, 1.0],
        };

        if let Some(color_q) = self.active_color_quantity() {
            uniforms.use_per_point_color = 1;
            color_q.apply_to_render_data(queue, render_data);
        }

        render_data.update_uniforms(queue, &uniforms);
    }
}
```

**Step 3: Call update in render loop**

In `app.rs`, after initializing GPU resources:
```rust
// Update GPU buffers
crate::with_context(|ctx| {
    for structure in ctx.registry.structures() {
        if structure.type_name() == "PointCloud" {
            if let Some(pc) = (structure as &dyn std::any::Any).downcast_ref::<PointCloud>() {
                pc.update_gpu_buffers(&engine.queue);
            }
        }
    }
});
```

**Step 4: Test with demo**

Update demo to add color quantity:
```rust
let handle = register_point_cloud("my points", points);

// Add color based on position
let colors: Vec<Vec3> = (0..1000)
    .map(|i| {
        let x = (i % 10) as f32 / 9.0;
        let y = ((i / 10) % 10) as f32 / 9.0;
        let z = (i / 100) as f32 / 9.0;
        Vec3::new(x, y, z)
    })
    .collect();
handle.add_color_quantity("position colors", colors);

// Enable the color quantity
with_context_mut(|ctx| {
    if let Some(pc) = ctx.registry.get_mut("PointCloud", "my points") {
        if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
            if let Some(q) = pc.get_quantity_mut("position colors") {
                q.set_enabled(true);
            }
        }
    }
});
```

**Step 5: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Points colored by their XYZ position (rainbow gradient)

**Step 6: Commit**

```bash
git add crates/ examples/
git commit -m "feat: implement color quantity rendering for point clouds"
```

---

## Task 7: Implement Scalar Quantity with Colormap

**Files:**
- Modify: `crates/polyscope-render/src/color_maps.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/quantities.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/mod.rs`

**Step 1: Add colormap texture generation**

In `color_maps.rs`:
```rust
impl ColorMap {
    /// Creates a 1D texture from this colormap.
    pub fn create_texture(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
        const RESOLUTION: u32 = 256;
        let mut data = Vec::with_capacity((RESOLUTION * 4) as usize);

        for i in 0..RESOLUTION {
            let t = i as f32 / (RESOLUTION - 1) as f32;
            let color = self.sample(t);
            data.push((color.x * 255.0) as u8);
            data.push((color.y * 255.0) as u8);
            data.push((color.z * 255.0) as u8);
            data.push(255u8);
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("colormap {}", self.name)),
            size: wgpu::Extent3d {
                width: RESOLUTION,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(RESOLUTION * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: RESOLUTION,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        texture
    }
}
```

**Step 2: Add range and colormap to ScalarQuantity**

In `quantities.rs`:
```rust
pub struct PointCloudScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl PointCloudScalarQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            colormap_name: "viridis".to_string(),
            range_min: min,
            range_max: max,
        }
    }

    /// Maps scalar values to colors using the colormap.
    pub fn compute_colors(&self, colormap: &ColorMap) -> Vec<Vec3> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        self.values
            .iter()
            .map(|&v| {
                let t = (v - self.range_min) / range;
                colormap.sample(t)
            })
            .collect()
    }

    /// Gets the colormap name.
    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    /// Sets the colormap name.
    pub fn set_colormap(&mut self, name: impl Into<String>) {
        self.colormap_name = name.into();
    }
}
```

**Step 3: Add scalar quantity support to PointCloud**

In `mod.rs`:
```rust
impl PointCloud {
    /// Returns the currently active scalar quantity, if any.
    pub fn active_scalar_quantity(&self) -> Option<&PointCloudScalarQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = (q.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<PointCloudScalarQuantity>()
                {
                    return Some(sq);
                }
            }
        }
        None
    }
}
```

Update `update_gpu_buffers` to handle scalar quantities:
```rust
pub fn update_gpu_buffers(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
    let Some(render_data) = &self.render_data else { return };

    let mut uniforms = PointUniforms {
        point_radius: self.point_radius,
        use_per_point_color: 0,
        _padding: [0.0; 2],
        base_color: [self.base_color.x, self.base_color.y, self.base_color.z, 1.0],
    };

    // Priority: color quantity > scalar quantity > base color
    if let Some(color_q) = self.active_color_quantity() {
        uniforms.use_per_point_color = 1;
        color_q.apply_to_render_data(queue, render_data);
    } else if let Some(scalar_q) = self.active_scalar_quantity() {
        if let Some(colormap) = color_maps.get(scalar_q.colormap_name()) {
            uniforms.use_per_point_color = 1;
            let colors = scalar_q.compute_colors(colormap);
            render_data.update_colors(queue, &colors);
        }
    }

    render_data.update_uniforms(queue, &uniforms);
}
```

**Step 4: Update app.rs to pass colormaps**

```rust
pc.update_gpu_buffers(&engine.queue, &engine.color_maps);
```

**Step 5: Test with demo**

```rust
// Add scalar quantity
let scalars: Vec<f32> = (0..1000)
    .map(|i| {
        let x = (i % 10) as f32 / 9.0;
        let y = ((i / 10) % 10) as f32 / 9.0;
        x + y  // Scalar = x + y position
    })
    .collect();
handle.add_scalar_quantity("height", scalars);

// Enable scalar quantity
with_context_mut(|ctx| {
    if let Some(pc) = ctx.registry.get_mut("PointCloud", "my points") {
        if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
            if let Some(q) = pc.get_quantity_mut("height") {
                q.set_enabled(true);
            }
        }
    }
});
```

**Step 6: Verify**

Run: `cargo run --example demo -p polyscope`
Expected: Points colored by viridis colormap based on x+y position

**Step 7: Commit**

```bash
git add crates/ examples/
git commit -m "feat: implement scalar quantity with colormap for point clouds"
```

---

## Task 8: Implement Vector Quantity with Arrows

**Files:**
- Create: `crates/polyscope-render/src/shaders/vector_arrow.wgsl`
- Create: `crates/polyscope-render/src/vector_render.rs`
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope-structures/src/point_cloud/quantities.rs`

**Step 1: Create vector arrow shader**

Create `vector_arrow.wgsl`:
```wgsl
// Vector arrow shader using instanced rendering
// Each instance is an arrow from a base point in a direction

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct VectorUniforms {
    length_scale: f32,
    radius: f32,
    _padding: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> vector_uniforms: VectorUniforms;
@group(0) @binding(2) var<storage, read> base_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read> vectors: array<vec3<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
}

// Arrow cylinder mesh (simplified - 8 segments)
// This creates a cylinder from (0,0,0) to (0,0,1) with radius 1
const CYLINDER_SEGMENTS: u32 = 8u;
const VERTICES_PER_ARROW: u32 = 36u; // 8 segments * 2 tris * 3 verts for body + caps

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let base_pos = base_positions[instance_index];
    let vec = vectors[instance_index];
    let vec_length = length(vec);

    if (vec_length < 0.0001) {
        // Zero vector - place vertex at origin (will be degenerate)
        out.clip_position = camera.view_proj * vec4<f32>(base_pos, 1.0);
        out.normal = vec3<f32>(0.0, 1.0, 0.0);
        out.color = vector_uniforms.color.rgb;
        return out;
    }

    let vec_dir = vec / vec_length;
    let scaled_length = vec_length * vector_uniforms.length_scale;
    let radius = vector_uniforms.radius;

    // Build orthonormal basis
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(vec_dir, up)) > 0.99) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let right = normalize(cross(vec_dir, up));
    let forward = cross(right, vec_dir);

    // Generate cylinder vertex
    let segment = vertex_index / 6u;
    let tri_vert = vertex_index % 6u;

    let angle0 = f32(segment) / f32(CYLINDER_SEGMENTS) * 6.283185;
    let angle1 = f32(segment + 1u) / f32(CYLINDER_SEGMENTS) * 6.283185;

    var local_pos: vec3<f32>;
    var local_normal: vec3<f32>;

    // Two triangles per segment
    if (tri_vert == 0u) {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    } else if (tri_vert == 1u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else if (tri_vert == 2u) {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    } else if (tri_vert == 3u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else if (tri_vert == 4u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    }

    // Transform to world space
    let world_pos = base_pos
        + right * local_pos.x
        + forward * local_pos.y
        + vec_dir * local_pos.z;

    let world_normal = right * local_normal.x + forward * local_normal.y;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.normal = world_normal;
    out.color = vector_uniforms.color.rgb;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.3;
    let diffuse = max(dot(normalize(in.normal), light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;

    return vec4<f32>(in.color * lighting, 1.0);
}
```

**Step 2: Create VectorRenderData**

Create `vector_render.rs`:
```rust
//! Vector arrow GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// GPU resources for rendering vectors.
pub struct VectorRenderData {
    pub base_buffer: wgpu::Buffer,
    pub vector_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub num_vectors: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VectorUniforms {
    pub length_scale: f32,
    pub radius: f32,
    pub _padding: [f32; 2],
    pub color: [f32; 4],
}

impl Default for VectorUniforms {
    fn default() -> Self {
        Self {
            length_scale: 1.0,
            radius: 0.005,
            _padding: [0.0; 2],
            color: [0.8, 0.2, 0.2, 1.0], // Red
        }
    }
}

impl VectorRenderData {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        bases: &[Vec3],
        vectors: &[Vec3],
    ) -> Self {
        let num_vectors = bases.len().min(vectors.len()) as u32;

        let base_data: Vec<f32> = bases.iter().flat_map(|p| [p.x, p.y, p.z, 0.0]).collect();
        let base_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector bases"),
            contents: bytemuck::cast_slice(&base_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let vector_data: Vec<f32> = vectors.iter().flat_map(|v| [v.x, v.y, v.z, 0.0]).collect();
        let vector_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vectors"),
            contents: bytemuck::cast_slice(&vector_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let uniforms = VectorUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vector bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vector_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            base_buffer,
            vector_buffer,
            uniform_buffer,
            bind_group,
            num_vectors,
        }
    }
}
```

**Step 3: Add vector pipeline to RenderEngine**

In `engine.rs`, add:
- `vector_pipeline: Option<wgpu::RenderPipeline>`
- `vector_bind_group_layout: Option<wgpu::BindGroupLayout>`
- `init_vector_pipeline()` method (similar to point pipeline)

**Step 4: Add vector rendering support to PointCloud**

Store vector render data and render in the app loop.

**Step 5: Test**

Add vector quantity to demo:
```rust
let vectors: Vec<Vec3> = (0..1000)
    .map(|i| {
        let x = (i % 10) as f32 / 9.0 - 0.5;
        let y = ((i / 10) % 10) as f32 / 9.0 - 0.5;
        Vec3::new(x, y, 0.0).normalize() * 0.05
    })
    .collect();
handle.add_vector_quantity("flow", vectors);
```

**Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement vector quantity with arrow rendering"
```

---

## Task 9: Final Integration Test

**Files:**
- Modify: `examples/demo.rs`
- Add: `crates/polyscope/tests/point_cloud_render_test.rs`

**Step 1: Create comprehensive demo**

```rust
use polyscope::*;

fn main() -> Result<()> {
    init()?;

    // Create a sphere of points
    let mut points = Vec::new();
    let n = 20;
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
        .map(|p| Vec3::new((p.x + 0.5), (p.y + 0.5), (p.z + 0.5)))
        .collect();
    handle.add_color_quantity("position", colors);

    // Add vector quantity (normal vectors)
    let vectors: Vec<Vec3> = points.iter().map(|p| p.normalize() * 0.1).collect();
    handle.add_vector_quantity("normals", vectors);

    // Enable scalar quantity by default
    with_context_mut(|ctx| {
        if let Some(pc) = ctx.registry.get_mut("PointCloud", "sphere") {
            if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                if let Some(q) = pc.get_quantity_mut("latitude") {
                    q.set_enabled(true);
                }
            }
        }
    });

    show();

    Ok(())
}
```

**Step 2: Run and verify visually**

Run: `cargo run --example demo -p polyscope`
Expected: Sphere of points colored by latitude (z-coordinate) using viridis

**Step 3: Final commit**

```bash
git add .
git commit -m "feat: complete Phase 3 - point cloud rendering with quantities"
```

---

## Summary

Phase 3 implements:
1. **Sphere impostor rendering** - Instanced billboards with ray-cast spheres in fragment shader
2. **Color quantities** - Direct per-point RGB colors
3. **Scalar quantities** - Colormap-based coloring with automatic range detection
4. **Vector quantities** - Instanced cylinder arrows

Key architectural choices:
- Instanced rendering (no geometry shaders in wgpu)
- Storage buffers for per-point data
- CPU-side colormap application (simpler than 1D texture in initial implementation)
- Unified camera uniform buffer shared across pipelines
