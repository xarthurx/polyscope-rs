# Ground Shadows Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Render scene objects to a shadow map from the light's perspective, enabling the ground plane to display real shadows under meshes.

**Architecture:** Add a shadow render pass before the main render pass that renders SurfaceMesh objects to a depth-only shadow map texture. The existing ground plane shader already samples this texture with PCF - we just need to populate it with actual depth data.

**Tech Stack:** wgpu, WGSL shaders, bytemuck

---

## Task 1: Create Shadow Render Pipeline in Engine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add shadow pipeline fields to RenderEngine struct**

Find the struct definition (around line 80-120) and add after `shadow_map_pass`:

```rust
// Add these fields:
shadow_pipeline: Option<wgpu::RenderPipeline>,
shadow_bind_group_layout: Option<wgpu::BindGroupLayout>,
```

**Step 2: Initialize fields in constructors**

In `new()` (around line 350) and `new_headless()` (around line 570), add:

```rust
shadow_pipeline: None,
shadow_bind_group_layout: None,
```

**Step 3: Create shadow pipeline method**

Add this method after `create_curve_network_tube_pipelines`:

```rust
fn create_shadow_pipeline(&mut self) {
    let shader_source = include_str!("shaders/shadow_map.wgsl");
    let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shadow Map Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Bind group layout: light uniforms + vertex positions
    let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Shadow Bind Group Layout"),
        entries: &[
            // Light uniforms (view_proj matrix)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Vertex positions (storage buffer)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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
        label: Some("Shadow Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shadow Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: None, // Depth-only, no fragment shader needed
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2, // Bias to prevent shadow acne
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(), // No MSAA for shadow map
        multiview: None,
        cache: None,
    });

    self.shadow_pipeline = Some(pipeline);
    self.shadow_bind_group_layout = Some(bind_group_layout);
}
```

**Step 4: Call the method in constructors**

Add after other pipeline creation calls:
```rust
engine.create_shadow_pipeline();
```

**Step 5: Add accessor methods**

```rust
pub fn shadow_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
    self.shadow_pipeline.as_ref()
}

pub fn shadow_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
    self.shadow_bind_group_layout.as_ref()
}
```

**Step 6: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add shadow render pipeline to engine"
```

---

## Task 2: Verify Shadow Map Shader Exists

**Files:**
- Check: `crates/polyscope-render/src/shaders/shadow_map.wgsl`

**Step 1: Read existing shader**

The shader should already exist. Verify it has the correct structure:

```wgsl
// Shadow map vertex shader
// Renders depth from light's perspective

struct LightUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniforms;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let pos = positions[vertex_index].xyz;
    return light.view_proj * vec4<f32>(pos, 1.0);
}
```

**Step 2: Update shader if needed**

If the shader doesn't match the bind group layout (light uniforms at binding 0, positions at binding 1), update it.

**Step 3: Commit if changed**

```bash
git add crates/polyscope-render/src/shaders/shadow_map.wgsl
git commit -m "fix(render): update shadow map shader bindings"
```

---

## Task 3: Add Shadow Bind Group to SurfaceMeshRenderData

**Files:**
- Modify: `crates/polyscope-render/src/surface_mesh_render.rs`

**Step 1: Add shadow bind group field**

Add to `SurfaceMeshRenderData` struct:

```rust
/// Bind group for shadow pass rendering.
pub shadow_bind_group: Option<wgpu::BindGroup>,
```

**Step 2: Initialize as None in constructor**

In the `new()` method, add to the `Self { ... }` return:

```rust
shadow_bind_group: None,
```

**Step 3: Add method to initialize shadow resources**

```rust
/// Initializes shadow rendering resources.
pub fn init_shadow_resources(
    &mut self,
    device: &wgpu::Device,
    shadow_bind_group_layout: &wgpu::BindGroupLayout,
    light_buffer: &wgpu::Buffer,
) {
    let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Surface Mesh Shadow Bind Group"),
        layout: shadow_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.vertex_buffer.as_entire_binding(),
            },
        ],
    });

    self.shadow_bind_group = Some(shadow_bind_group);
}

/// Returns whether shadow resources are initialized.
pub fn has_shadow_resources(&self) -> bool {
    self.shadow_bind_group.is_some()
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/surface_mesh_render.rs
git commit -m "feat(render): add shadow bind group to SurfaceMeshRenderData"
```

---

## Task 4: Add Shadow Resource Initialization to SurfaceMesh

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Step 1: Add method to initialize shadow resources**

```rust
/// Initializes shadow rendering resources.
pub fn init_shadow_resources(
    &mut self,
    device: &wgpu::Device,
    shadow_bind_group_layout: &wgpu::BindGroupLayout,
    light_buffer: &wgpu::Buffer,
) {
    if let Some(render_data) = &mut self.render_data {
        render_data.init_shadow_resources(device, shadow_bind_group_layout, light_buffer);
    }
}

/// Returns the shadow bind group if initialized.
pub fn shadow_bind_group(&self) -> Option<&wgpu::BindGroup> {
    self.render_data.as_ref()?.shadow_bind_group.as_ref()
}

/// Returns whether shadow resources are initialized.
pub fn has_shadow_resources(&self) -> bool {
    self.render_data.as_ref().map_or(false, |rd| rd.has_shadow_resources())
}
```

**Step 2: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(structures): add shadow resource methods to SurfaceMesh"
```

---

## Task 5: Add Shadow Pass to Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add shadow pass before main render pass**

Find the main render loop (in the `run()` method, before the main render pass). Add the shadow pass:

```rust
// Shadow pass - render objects to shadow map
if let (Some(shadow_pipeline), Some(shadow_map_pass)) =
    (engine.shadow_pipeline(), engine.shadow_map_pass())
{
    // Compute light matrix from scene bounds
    let (scene_center, scene_radius) = crate::with_context(|ctx| {
        (ctx.center(), ctx.length_scale * 2.0)
    });
    let light_dir = glam::Vec3::new(0.5, -1.0, 0.3).normalize();
    let light_matrix = polyscope_render::ShadowMapPass::compute_light_matrix(
        scene_center,
        scene_radius,
        light_dir,
    );

    // Update light uniforms
    shadow_map_pass.update_light(&engine.queue, light_matrix, light_dir);

    // Begin shadow pass
    {
        let mut shadow_pass = shadow_map_pass.begin_shadow_pass(&mut encoder);
        shadow_pass.set_pipeline(shadow_pipeline);

        // Render shadow-casting structures
        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !structure.is_enabled() {
                    continue;
                }
                if structure.type_name() == "SurfaceMesh" {
                    if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                        if let Some(shadow_bg) = mesh.shadow_bind_group() {
                            shadow_pass.set_bind_group(0, shadow_bg, &[]);
                            if let Some(rd) = mesh.render_data() {
                                shadow_pass.draw(0..rd.num_vertices, 0..1);
                            }
                        }
                    }
                }
            }
        });
    }
}
```

**Step 2: Initialize shadow resources for meshes**

In the structure initialization section (where GPU resources are created), add shadow resource initialization:

```rust
if structure.type_name() == "SurfaceMesh" {
    if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
        // ... existing render data init ...

        // Initialize shadow resources
        if mesh.render_data().is_some() && !mesh.has_shadow_resources() {
            if let (Some(shadow_layout), Some(shadow_pass)) =
                (engine.shadow_bind_group_layout(), engine.shadow_map_pass())
            {
                mesh.init_shadow_resources(
                    &engine.device,
                    shadow_layout,
                    shadow_pass.light_buffer(),
                );
            }
        }
    }
}
```

**Step 3: Add necessary imports**

At the top of app.rs, ensure these are imported:
```rust
use polyscope_render::ShadowMapPass;
```

**Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat(app): add shadow pass to render loop"
```

---

## Task 6: Test Shadow Rendering

**Step 1: Run surface mesh demo**

Run: `cargo run --example surface_mesh_demo`

**Step 2: Verify shadows appear**

- Set ground plane mode to "Tile" or "Shadow Only"
- Mesh should cast shadow on ground plane
- Shadow should be visible as darkened area under the mesh

**Step 3: Test shadow darkness slider**

- Adjust "Shadow Darkness" slider in UI
- Shadow intensity should change accordingly

**Step 4: Test from different angles**

- Rotate camera around the scene
- Shadow should remain consistent relative to light direction

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete ground shadow rendering implementation"
```

---

## Task 7: Update Documentation

**Files:**
- Modify: `docs/architecture-differences.md`

**Step 1: Update shadow status**

Change:
```markdown
| Ground Shadows | ✅ | ⚠️ Infrastructure only |
```

To:
```markdown
| Ground Shadows | ✅ | ✅ |
```

**Step 2: Remove shadow from "Partially Implemented" section**

Remove the "Ground Shadows (Infrastructure Only)" section since it's now fully implemented.

**Step 3: Commit**

```bash
git add docs/architecture-differences.md
git commit -m "docs: update shadow status to fully implemented"
```

---

## Summary

This implementation adds ground shadow rendering by:

1. Creating a shadow render pipeline (depth-only)
2. Adding shadow bind groups to SurfaceMesh
3. Adding a shadow pass before the main render pass
4. Using the existing ShadowMapPass infrastructure

The shadow pass renders all enabled SurfaceMesh structures from the light's perspective, populating the shadow map that the ground plane shader already samples.
