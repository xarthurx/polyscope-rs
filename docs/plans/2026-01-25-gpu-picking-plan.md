# GPU Picking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace screen-space approximation picking with pixel-perfect GPU picking that identifies exact elements (point #42, face #127, etc.)

**Architecture:** Render all structures to an offscreen pick buffer with encoded color IDs, read back pixel at click position, decode to identify structure + element.

**Tech Stack:** wgpu (textures, buffers, pipelines), WGSL shaders, pollster (async blocking)

---

## Task 1: Add Encoding/Decoding Utilities

**Files:**
- Modify: `crates/polyscope-render/src/pick.rs`
- Test: `crates/polyscope-render/src/pick.rs` (inline tests)

**Step 1: Write the failing test**

Add to `crates/polyscope-render/src/pick.rs`:

```rust
#[test]
fn test_encode_decode_pick_id_roundtrip() {
    // Test various combinations
    let cases = [
        (1, 0),
        (1, 1),
        (0xFFF, 0xFFF),  // max values
        (123, 456),
        (4095, 4095),
    ];
    for (struct_id, elem_id) in cases {
        let encoded = encode_pick_id(struct_id, elem_id);
        let (decoded_struct, decoded_elem) = decode_pick_id(encoded[0], encoded[1], encoded[2]);
        assert_eq!(decoded_struct, struct_id, "struct_id mismatch for ({}, {})", struct_id, elem_id);
        assert_eq!(decoded_elem, elem_id, "elem_id mismatch for ({}, {})", struct_id, elem_id);
    }
}

#[test]
fn test_encode_pick_id_background() {
    let encoded = encode_pick_id(0, 0);
    assert_eq!(encoded, [0, 0, 0], "Background should encode to black");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-render test_encode_decode_pick_id`
Expected: FAIL with "cannot find function `encode_pick_id`"

**Step 3: Write minimal implementation**

Add to `crates/polyscope-render/src/pick.rs` before the tests module:

```rust
/// Encodes a structure ID and element ID into RGB pick color.
///
/// Uses 12 bits for structure ID (max 4096) and 12 bits for element ID (max 4096).
/// Layout: R[7:0] = struct[11:4], G[7:4] = struct[3:0], G[3:0] = elem[11:8], B[7:0] = elem[7:0]
pub fn encode_pick_id(structure_id: u16, element_id: u16) -> [u8; 3] {
    let s = structure_id & 0xFFF;  // 12 bits max
    let e = element_id & 0xFFF;    // 12 bits max
    [
        (s >> 4) as u8,                           // R: struct bits 11-4
        (((s & 0xF) << 4) | (e >> 8)) as u8,      // G: struct bits 3-0 + elem bits 11-8
        (e & 0xFF) as u8,                         // B: elem bits 7-0
    ]
}

/// Decodes RGB pick color back to structure ID and element ID.
pub fn decode_pick_id(r: u8, g: u8, b: u8) -> (u16, u16) {
    let structure_id = ((r as u16) << 4) | ((g as u16) >> 4);
    let element_id = (((g & 0xF) as u16) << 8) | (b as u16);
    (structure_id, element_id)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-render test_encode_decode_pick_id`
Expected: PASS

**Step 5: Export functions in lib.rs**

Add to `crates/polyscope-render/src/lib.rs` exports:

```rust
pub use pick::{encode_pick_id, decode_pick_id};
```

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/pick.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(pick): add encode/decode utilities for structure+element IDs"
```

---

## Task 2: Add Structure ID Management to RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Test: `crates/polyscope-render/src/engine.rs` (inline tests)

**Step 1: Add fields to RenderEngine struct**

In `crates/polyscope-render/src/engine.rs`, add to `RenderEngine` struct:

```rust
use std::collections::HashMap;

pub struct RenderEngine {
    // ... existing fields ...

    // Pick system - structure ID management
    structure_id_map: HashMap<(String, String), u16>,      // (type, name) -> id
    structure_id_reverse: HashMap<u16, (String, String)>,  // id -> (type, name)
    next_structure_id: u16,
}
```

**Step 2: Initialize fields in new()**

In the `new()` or initialization function, add:

```rust
structure_id_map: HashMap::new(),
structure_id_reverse: HashMap::new(),
next_structure_id: 1,  // 0 is reserved for background
```

**Step 3: Add structure ID methods**

```rust
impl RenderEngine {
    /// Assigns a unique pick ID to a structure. Returns the assigned ID.
    pub fn assign_structure_id(&mut self, type_name: &str, name: &str) -> u16 {
        let key = (type_name.to_string(), name.to_string());
        if let Some(&id) = self.structure_id_map.get(&key) {
            return id;
        }
        let id = self.next_structure_id;
        self.next_structure_id += 1;
        self.structure_id_map.insert(key.clone(), id);
        self.structure_id_reverse.insert(id, key);
        id
    }

    /// Removes a structure's pick ID.
    pub fn remove_structure_id(&mut self, type_name: &str, name: &str) {
        let key = (type_name.to_string(), name.to_string());
        if let Some(id) = self.structure_id_map.remove(&key) {
            self.structure_id_reverse.remove(&id);
        }
    }

    /// Looks up structure info from a pick ID.
    pub fn lookup_structure_id(&self, id: u16) -> Option<(&str, &str)> {
        self.structure_id_reverse.get(&id).map(|(t, n)| (t.as_str(), n.as_str()))
    }

    /// Gets the pick ID for a structure, if assigned.
    pub fn get_structure_id(&self, type_name: &str, name: &str) -> Option<u16> {
        let key = (type_name.to_string(), name.to_string());
        self.structure_id_map.get(&key).copied()
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p polyscope-render`
Expected: PASS (existing tests should still pass)

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(engine): add structure ID management for GPU picking"
```

---

## Task 3: Create Pick Buffer Textures

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add pick buffer fields to RenderEngine**

```rust
pub struct RenderEngine {
    // ... existing fields ...

    // Pick system - GPU resources
    pick_texture: Option<wgpu::Texture>,
    pick_texture_view: Option<wgpu::TextureView>,
    pick_depth_texture: Option<wgpu::Texture>,
    pick_depth_view: Option<wgpu::TextureView>,
    pick_staging_buffer: Option<wgpu::Buffer>,
    pick_buffer_size: (u32, u32),  // Track size for resize detection
}
```

**Step 2: Initialize as None in new()**

```rust
pick_texture: None,
pick_texture_view: None,
pick_depth_texture: None,
pick_depth_view: None,
pick_staging_buffer: None,
pick_buffer_size: (0, 0),
```

**Step 3: Add init_pick_buffers method**

```rust
impl RenderEngine {
    /// Creates or recreates pick buffer textures to match viewport size.
    pub fn init_pick_buffers(&mut self, width: u32, height: u32) {
        // Skip if size unchanged
        if self.pick_buffer_size == (width, height) && self.pick_texture.is_some() {
            return;
        }

        let device = &self.device;

        // Create pick color texture (RGBA8Unorm for exact values)
        let pick_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pick Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pick_texture_view = pick_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create pick depth texture
        let pick_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pick Depth Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let pick_depth_view = pick_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create staging buffer for single pixel readback (4 bytes RGBA)
        // Buffer size must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let pick_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pick Staging Buffer"),
            size: 256,  // Minimum aligned size, we only read 4 bytes
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.pick_texture = Some(pick_texture);
        self.pick_texture_view = Some(pick_texture_view);
        self.pick_depth_texture = Some(pick_depth_texture);
        self.pick_depth_view = Some(pick_depth_view);
        self.pick_staging_buffer = Some(pick_staging_buffer);
        self.pick_buffer_size = (width, height);
    }
}
```

**Step 4: Run build to verify compilation**

Run: `cargo build -p polyscope-render`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(engine): add pick buffer texture creation"
```

---

## Task 4: Create Pick Pipeline for PointCloud

**Files:**
- Modify: `crates/polyscope-render/src/shaders/pick.wgsl`
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Update pick.wgsl to use new encoding**

Replace the `index_to_color` function in `crates/polyscope-render/src/shaders/pick.wgsl`:

```wgsl
struct PickUniforms {
    structure_id: u32,
    point_radius: f32,
    _padding: vec2<f32>,
}

// Encode structure_id (12 bits) and element_id (12 bits) into RGB
fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}
```

Update vertex shader to use `structure_id` from uniforms:

```wgsl
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = point_positions[instance_index].xyz;
    let view_pos = (camera.view * vec4<f32>(world_pos, 1.0)).xyz;

    let quad_pos = QUAD_VERTICES[vertex_index];
    let radius = pick_uniforms.point_radius;
    let offset = vec3<f32>(quad_pos * radius, 0.0);
    let billboard_pos_view = view_pos + offset;

    out.clip_position = camera.proj * vec4<f32>(billboard_pos_view, 1.0);
    out.pick_color = encode_pick_id(pick_uniforms.structure_id, instance_index);
    out.sphere_center_view = view_pos;
    out.quad_pos = quad_pos;
    out.point_radius = radius;

    return out;
}
```

**Step 2: Add PickUniforms struct in Rust**

Add to `crates/polyscope-render/src/pick.rs`:

```rust
/// GPU uniforms for pick rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PickUniforms {
    pub structure_id: u32,
    pub point_radius: f32,
    pub _padding: [f32; 2],
}

impl Default for PickUniforms {
    fn default() -> Self {
        Self {
            structure_id: 0,
            point_radius: 0.01,
            _padding: [0.0; 2],
        }
    }
}
```

Export in lib.rs:

```rust
pub use pick::PickUniforms;
```

**Step 3: Add pick pipeline creation to RenderEngine**

Add to `crates/polyscope-render/src/engine.rs`:

```rust
impl RenderEngine {
    /// Creates the pick pipeline for point clouds.
    pub fn create_point_cloud_pick_pipeline(&mut self) -> wgpu::RenderPipeline {
        let shader_source = include_str!("shaders/pick.wgsl");
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pick Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pick Pipeline Layout"),
            bind_group_layouts: &[&self.camera_bind_group_layout],  // Reuse camera layout
            push_constant_ranges: &[],
        });

        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PointCloud Pick Pipeline"),
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,  // No blending for pick buffer
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }
}
```

**Step 4: Run build**

Run: `cargo build -p polyscope-render`
Expected: Compiles (may have warnings about unused pipeline)

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/shaders/pick.wgsl crates/polyscope-render/src/pick.rs crates/polyscope-render/src/engine.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(pick): update pick shader and add pipeline for point clouds"
```

---

## Task 5: Implement pick_at() Method

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add pick_at method**

```rust
impl RenderEngine {
    /// Reads the pick buffer at (x, y) and returns the decoded structure/element.
    ///
    /// Returns None if picking system not initialized or coordinates out of bounds.
    /// Returns Some((0, 0)) for background clicks.
    pub fn pick_at(&self, x: u32, y: u32) -> Option<(u16, u16)> {
        let pick_texture = self.pick_texture.as_ref()?;
        let staging_buffer = self.pick_staging_buffer.as_ref()?;

        // Bounds check
        let (width, height) = self.pick_buffer_size;
        if x >= width || y >= height {
            return None;
        }

        // Create encoder for copy operation
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pick Readback Encoder"),
        });

        // Copy single pixel from pick texture to staging buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: pick_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(256),  // Aligned
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read pixel
        let buffer_slice = staging_buffer.slice(..4);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().ok()?;

        let data = buffer_slice.get_mapped_range();
        let pixel: [u8; 4] = [data[0], data[1], data[2], data[3]];
        drop(data);
        staging_buffer.unmap();

        let (struct_id, elem_id) = crate::pick::decode_pick_id(pixel[0], pixel[1], pixel[2]);
        Some((struct_id, elem_id))
    }
}
```

**Step 2: Run build**

Run: `cargo build -p polyscope-render`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(engine): add pick_at() method for GPU pixel readback"
```

---

## Task 6: Implement render_pick_pass() Method

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add render_pick_pass method**

This is a simplified version that renders point clouds only. Other structures will be added in subsequent tasks.

```rust
impl RenderEngine {
    /// Renders all visible structures to the pick buffer.
    pub fn render_pick_pass(
        &mut self,
        point_clouds: &[(u16, &[glam::Vec3], f32)],  // (structure_id, positions, radius)
    ) {
        let pick_view = match &self.pick_texture_view {
            Some(v) => v,
            None => return,
        };
        let pick_depth = match &self.pick_depth_view {
            Some(v) => v,
            None => return,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pick Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pick Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pick_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),  // Background = (0,0,0)
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: pick_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // TODO: Render each structure with its pick pipeline
            // For now, this is a placeholder that will be filled in during integration
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
```

**Step 2: Run build**

Run: `cargo build -p polyscope-render`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(engine): add render_pick_pass() scaffold"
```

---

## Task 7: Create Pick Shader for SurfaceMesh

**Files:**
- Create: `crates/polyscope-render/src/shaders/pick_mesh.wgsl`

**Step 1: Create the shader file**

```wgsl
// Pick shader for surface meshes - outputs encoded face index

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    structure_id: u32,
    _padding: vec3<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var<uniform> pick: PickUniforms;
@group(2) @binding(0) var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) face_index: u32,
}

fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(
    in: VertexInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;

    // Face index = vertex_index / 3 (each face has 3 vertices)
    out.face_index = vertex_index / 3u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = encode_pick_id(pick.structure_id, in.face_index);
    return vec4<f32>(color, 1.0);
}
```

**Step 2: Run build to verify shader syntax**

Run: `cargo build -p polyscope-render`
Expected: Compiles (shader not used yet, but included for syntax check if using include_str!)

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/pick_mesh.wgsl
git commit -m "feat(shaders): add pick_mesh.wgsl for surface mesh picking"
```

---

## Task 8: Create Pick Shader for CurveNetwork

**Files:**
- Create: `crates/polyscope-render/src/shaders/pick_curve.wgsl`

**Step 1: Create the shader file**

```wgsl
// Pick shader for curve networks - outputs encoded edge index

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    structure_id: u32,
    line_width: f32,
    _padding: vec2<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var<uniform> pick: PickUniforms;
@group(2) @binding(0) var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) edge_index: u32,
}

fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(
    in: VertexInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;

    // Edge index = vertex_index / 2 (each edge has 2 vertices for lines)
    // Or vertex_index / 6 if using triangle strips for thick lines
    out.edge_index = vertex_index / 2u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = encode_pick_id(pick.structure_id, in.edge_index);
    return vec4<f32>(color, 1.0);
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/pick_curve.wgsl
git commit -m "feat(shaders): add pick_curve.wgsl for curve network picking"
```

---

## Task 9: Integrate GPU Picking into App

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Initialize pick system on startup**

In the app initialization or first render, add:

```rust
// After engine is created and surface configured
engine.init_pick_buffers(width, height);
```

**Step 2: Replace pick_structure_at_screen_pos with GPU picking**

Find the `pick_structure_at_screen_pos` method and update the click handler to use GPU picking instead:

```rust
fn handle_click(&mut self, click_pos: glam::Vec2, engine: &mut RenderEngine) -> Option<(String, String, u32)> {
    let x = click_pos.x as u32;
    let y = click_pos.y as u32;

    // Ensure pick buffers are initialized
    let (width, height) = (engine.surface_config.width, engine.surface_config.height);
    engine.init_pick_buffers(width, height);

    // Render pick pass (TODO: pass actual structures)
    engine.render_pick_pass(&[]);

    // Read pixel at click position
    if let Some((struct_id, elem_id)) = engine.pick_at(x, y) {
        if struct_id == 0 {
            // Clicked background
            return None;
        }

        // Look up structure info
        if let Some((type_name, name)) = engine.lookup_structure_id(struct_id) {
            return Some((type_name.to_string(), name.to_string(), elem_id as u32));
        }
    }

    None
}
```

**Step 3: Update PickResult usage**

Update the selection code to store element info:

```rust
if let Some((type_name, name, element_index)) = self.handle_click(click_pos, engine) {
    select_structure(&type_name, &name);
    // Store element info for UI
    self.selected_element_index = Some(element_index);
} else {
    deselect_structure();
    self.selected_element_index = None;
}
```

**Step 4: Run and test manually**

Run: `cargo run --example demo`
Expected: Application runs. Clicking may not work fully yet (pick pass doesn't render structures yet).

**Step 5: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat(app): integrate GPU picking infrastructure"
```

---

## Task 10: Wire Up PointCloud Pick Rendering

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Complete the render_pick_pass for point clouds**

This requires significant integration with the existing point cloud rendering code. The implementation should:

1. Get the pick pipeline
2. For each point cloud, create pick uniforms buffer with structure_id
3. Bind camera, pick uniforms, and point position buffers
4. Draw instanced quads

This step requires careful integration with the existing PointCloudRenderData structure.

**Step 2: Test with point cloud**

Run: `cargo run --example demo`
Click on a point cloud and verify it selects correctly.

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/engine.rs crates/polyscope/src/app.rs
git commit -m "feat(pick): complete point cloud pick rendering"
```

---

## Task 11: Add Element Info to Selection UI

**Files:**
- Modify: `crates/polyscope-ui/src/selection_panel.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Update SelectionInfo struct**

Add element info fields:

```rust
pub struct SelectionInfo {
    pub has_selection: bool,
    pub type_name: String,
    pub name: String,
    // ... existing fields ...
    pub element_type: String,   // "Point", "Face", "Edge", etc.
    pub element_index: u32,
}
```

**Step 2: Update selection panel UI**

Display element info when available:

```rust
if selection.element_index > 0 || !selection.element_type.is_empty() {
    ui.label(format!("Element: {} #{}", selection.element_type, selection.element_index));
}
```

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/selection_panel.rs crates/polyscope/src/app.rs
git commit -m "feat(ui): display picked element info in selection panel"
```

---

## Task 12: Final Testing and Cleanup

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Test manual picking**

Run: `cargo run --example demo`

Test:
- Click on point cloud → selects structure, shows "Point #N"
- Click on mesh → selects structure, shows "Face #N"
- Click on background → deselects
- Click with overlapping structures → front one is selected

**Step 3: Clean up old screen-space picking code**

Remove or deprecate `pick_structure_at_screen_pos` method if no longer needed.

**Step 4: Update todo.md**

Mark GPU picking tasks as complete.

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(pick): complete GPU picking implementation"
```

---

## Summary

After completing all tasks:
- Pixel-perfect GPU picking replaces 20px threshold approximation
- Element-level selection (point #42, face #127)
- Selection panel shows clicked element
- Foundation ready for future hover highlighting
