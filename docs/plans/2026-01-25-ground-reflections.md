# Ground Reflections Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable planar ground reflections showing mirrored geometry on the ground plane.

**Architecture:** Use stencil buffer to mask the ground plane region, then render reflected geometry only within that region. Existing shaders (`reflected_mesh.wgsl`, `ground_stencil.wgsl`) and infrastructure (`ReflectionPass`, `reflection_matrix()`) are already in place - this plan wires them together in the render loop.

**Tech Stack:** wgpu (stencil buffer), existing WGSL shaders, existing ReflectionPass

---

## Existing Infrastructure (Already Implemented)

- `crates/polyscope-render/src/reflection.rs` - `reflection_matrix()`, `ground_reflection_matrix()`
- `crates/polyscope-render/src/reflection_pass.rs` - `ReflectionPass` with uniform buffer
- `crates/polyscope-render/src/shaders/reflected_mesh.wgsl` - Shader for reflected geometry
- `crates/polyscope-render/src/shaders/ground_stencil.wgsl` - Shader for stencil marking
- `crates/polyscope-core/src/ground_plane.rs` - `GroundPlaneMode::TileReflection`, `reflection_intensity`
- `crates/polyscope-ui/src/panels.rs` - UI already supports TileReflection mode and intensity slider

---

## Task 1: Change Depth-Stencil Format to Support Stencil Buffer

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

The current depth texture uses `Depth32Float` which has no stencil bits. We need `Depth24PlusStencil8` for stencil operations.

**Step 1: Find and update depth texture format constant**

In `engine.rs`, locate the depth texture creation (around line 250-280 in `new()` and `new_headless()`).

Change:
```rust
format: wgpu::TextureFormat::Depth32Float,
```
To:
```rust
format: wgpu::TextureFormat::Depth24PlusStencil8,
```

**Step 2: Update all depth_stencil states in pipeline descriptors**

Search for all occurrences of `Depth32Float` in pipeline descriptors and change to `Depth24PlusStencil8`.

Locations to update (search for `depth_stencil: Some(wgpu::DepthStencilState`):
- Surface mesh pipeline
- Point cloud pipeline
- Curve network pipeline
- Ground plane pipeline
- Shadow pipeline
- Any other rendering pipelines

**Step 3: Run tests to verify nothing broke**

Run: `cargo test`
Expected: All tests pass

**Step 4: Build and quick visual test**

Run: `cargo run --example basic`
Expected: Renders normally (depth testing still works)

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "refactor: change depth format to Depth24PlusStencil8 for stencil support"
```

---

## Task 2: Create Ground Stencil Pipeline

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Create a pipeline for rendering the ground plane to the stencil buffer (marks where reflections should appear).

**Step 1: Add stencil pipeline field to RenderEngine struct**

Find the struct definition and add after `shadow_pipeline`:
```rust
/// Stencil pipeline for ground plane reflection mask.
ground_stencil_pipeline: Option<wgpu::RenderPipeline>,
```

**Step 2: Initialize field in constructor**

In both `new()` and `new_headless()`, initialize:
```rust
ground_stencil_pipeline: None,
```

**Step 3: Create the stencil pipeline creation method**

Add this method after `create_shadow_pipeline()`:

```rust
/// Creates the ground stencil pipeline for reflection masking.
fn create_ground_stencil_pipeline(&mut self) {
    let shader = self
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ground Stencil Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/ground_stencil.wgsl").into(),
            ),
        });

    // Use existing ground plane bind group layout (camera + ground uniforms)
    let Some(ground_data) = &self.ground_plane_data else {
        return;
    };

    let pipeline_layout = self
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ground Stencil Pipeline Layout"),
            bind_group_layouts: &[&self.ground_plane_bind_group_layout],
            push_constant_ranges: &[],
        });

    self.ground_stencil_pipeline = Some(self.device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("Ground Stencil Pipeline"),
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
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(), // No color writes
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false, // Don't write depth
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Replace, // Write stencil ref
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Replace,
                    },
                    read_mask: 0xFF,
                    write_mask: 0xFF,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        },
    ));
}
```

**Step 4: Call pipeline creation in initialization**

In both `new()` and `new_headless()`, after `engine.init_reflection_pass();`, add:
```rust
engine.create_ground_stencil_pipeline();
```

**Step 5: Build to verify compilation**

Run: `cargo build`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat: add ground stencil pipeline for reflection masking"
```

---

## Task 3: Create Reflected Mesh Pipeline

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Create a pipeline for rendering surface meshes with reflection transform applied.

**Step 1: Add reflected mesh pipeline field**

Add to RenderEngine struct:
```rust
/// Pipeline for rendering reflected surface meshes.
reflected_mesh_pipeline: Option<wgpu::RenderPipeline>,
/// Bind group layout for reflected mesh (includes reflection uniforms).
reflected_mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,
```

**Step 2: Initialize fields in constructors**

```rust
reflected_mesh_pipeline: None,
reflected_mesh_bind_group_layout: None,
```

**Step 3: Create reflected mesh pipeline method**

Add after `create_ground_stencil_pipeline()`:

```rust
/// Creates the reflected mesh pipeline for ground reflections.
fn create_reflected_mesh_pipeline(&mut self) {
    let shader = self
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reflected Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/reflected_mesh.wgsl").into(),
            ),
        });

    // Bind group 0: camera, mesh uniforms, buffers (same as surface mesh)
    // Bind group 1: reflection uniforms
    let Some(reflection_pass) = &self.reflection_pass else {
        return;
    };

    // Create bind group layout for group 0 (mesh data)
    let mesh_bind_group_layout =
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Reflected Mesh Bind Group Layout 0"),
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
                    // Mesh uniforms
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
                    // Positions
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
                    // Normals
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
                    // Barycentrics
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Colors
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Edge is real
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
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

    self.reflected_mesh_bind_group_layout = Some(mesh_bind_group_layout.clone());

    let pipeline_layout = self
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Reflected Mesh Pipeline Layout"),
            bind_group_layouts: &[&mesh_bind_group_layout, reflection_pass.bind_group_layout()],
            push_constant_ranges: &[],
        });

    self.reflected_mesh_pipeline = Some(self.device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("Reflected Mesh Pipeline"),
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
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Front), // Cull front faces (they become back after reflection)
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal, // Only render where stencil == ref
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xFF,
                    write_mask: 0x00, // Don't modify stencil
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        },
    ));
}
```

**Step 4: Call pipeline creation**

After `engine.create_ground_stencil_pipeline();`, add:
```rust
engine.create_reflected_mesh_pipeline();
```

**Step 5: Build to verify**

Run: `cargo build`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat: add reflected mesh pipeline with stencil test"
```

---

## Task 4: Add Stencil Rendering Method

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Add a method to render the ground plane to the stencil buffer.

**Step 1: Add render_stencil_pass method**

Add this public method:

```rust
/// Renders the ground plane to the stencil buffer for reflection masking.
///
/// This should be called before rendering reflected geometry.
/// The stencil buffer will have value 1 where the ground plane is visible.
pub fn render_stencil_pass(
    &self,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    ground_height: f32,
    scene_center: [f32; 3],
    length_scale: f32,
    camera_height: f32,
) {
    let Some(pipeline) = &self.ground_stencil_pipeline else {
        return;
    };
    let Some(ground_data) = &self.ground_plane_data else {
        return;
    };

    // Update ground uniforms for stencil pass
    ground_data.update(
        &self.queue,
        scene_center,
        scene_center[1] - length_scale * 0.5, // scene_min_y estimate
        length_scale,
        camera_height,
        Some(ground_height),
        0.0,  // shadow_darkness (unused in stencil)
        0,    // shadow_mode (unused in stencil)
        self.camera.projection_mode == crate::camera::ProjectionMode::Orthographic,
    );

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Stencil Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: self.hdr_texture_view.as_ref().unwrap_or(color_view),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load, // Don't clear color
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load, // Keep existing depth
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0), // Clear stencil to 0
                store: wgpu::StoreOp::Store,
            }),
        }),
        ..Default::default()
    });

    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, ground_data.bind_group(), &[]);
    render_pass.set_stencil_reference(1); // Write 1 to stencil
    render_pass.draw(0..12, 0..1); // 4 triangles = 12 vertices
}
```

**Step 2: Build to verify**

Run: `cargo build`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat: add render_stencil_pass for reflection masking"
```

---

## Task 5: Add Reflected Mesh Rendering Method

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope-render/src/surface_mesh_render.rs` (may need to expose bind group creation)

Add a method to render reflected surface meshes.

**Step 1: Add method to create reflected mesh bind group**

In `engine.rs`, add:

```rust
/// Creates a bind group for reflected mesh rendering.
pub fn create_reflected_mesh_bind_group(
    &self,
    mesh_render_data: &crate::surface_mesh_render::SurfaceMeshRenderData,
) -> Option<wgpu::BindGroup> {
    let layout = self.reflected_mesh_bind_group_layout.as_ref()?;

    Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Reflected Mesh Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: mesh_render_data.uniform_buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: mesh_render_data.position_buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: mesh_render_data.normal_buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: mesh_render_data.barycentric_buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: mesh_render_data.color_buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: mesh_render_data.edge_real_buffer().as_entire_binding(),
            },
        ],
    }))
}
```

**Step 2: Expose buffer getters in SurfaceMeshRenderData**

In `crates/polyscope-render/src/surface_mesh_render.rs`, add these public methods to `SurfaceMeshRenderData`:

```rust
/// Returns the uniform buffer.
pub fn uniform_buffer(&self) -> &wgpu::Buffer {
    &self.uniform_buffer
}

/// Returns the position buffer.
pub fn position_buffer(&self) -> &wgpu::Buffer {
    &self.position_buffer
}

/// Returns the normal buffer.
pub fn normal_buffer(&self) -> &wgpu::Buffer {
    &self.normal_buffer
}

/// Returns the barycentric buffer.
pub fn barycentric_buffer(&self) -> &wgpu::Buffer {
    &self.barycentric_buffer
}

/// Returns the color buffer.
pub fn color_buffer(&self) -> &wgpu::Buffer {
    &self.color_buffer
}

/// Returns the edge real buffer.
pub fn edge_real_buffer(&self) -> &wgpu::Buffer {
    &self.edge_real_buffer
}
```

**Step 3: Add render_reflected_mesh method**

In `engine.rs`:

```rust
/// Renders a single reflected mesh.
///
/// Call this for each visible surface mesh after render_stencil_pass.
pub fn render_reflected_mesh(
    &self,
    render_pass: &mut wgpu::RenderPass,
    mesh_bind_group: &wgpu::BindGroup,
    vertex_count: u32,
) {
    let Some(pipeline) = &self.reflected_mesh_pipeline else {
        return;
    };
    let Some(reflection) = &self.reflection_pass else {
        return;
    };

    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, mesh_bind_group, &[]);
    render_pass.set_bind_group(1, reflection.bind_group(), &[]);
    render_pass.set_stencil_reference(1); // Test against stencil value 1
    render_pass.draw(0..vertex_count, 0..1);
}
```

**Step 4: Build to verify**

Run: `cargo build`
Expected: Compiles (may have warnings about unused methods until integrated)

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs crates/polyscope-render/src/surface_mesh_render.rs
git commit -m "feat: add reflected mesh rendering methods"
```

---

## Task 6: Integrate Reflections into Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

Wire up the reflection rendering in the main render loop.

**Step 1: Add reflection pass to render method**

In `app.rs`, find the render loop (around line 1160-1180 where ground plane is rendered).

Before `engine.render_ground_plane(...)`, add:

```rust
// Reflection pass (only for TileReflection mode)
if self.ground_plane.mode == GroundPlaneMode::TileReflection {
    // Compute ground height
    let ground_height = if self.ground_plane.height_is_relative {
        let (bb_min, _bb_max) = polyscope_core::state::with_context(|ctx| ctx.bounding_box);
        bb_min.y - length_scale * 0.05
    } else {
        self.ground_plane.height
    };

    // Update reflection uniforms
    let reflection_matrix = polyscope_render::reflection::ground_reflection_matrix(ground_height);
    engine.update_reflection(
        reflection_matrix,
        self.ground_plane.reflection_intensity,
        ground_height,
    );

    // 1. Render stencil pass (mark ground plane region)
    engine.render_stencil_pass(
        &mut encoder,
        &view,
        ground_height,
        center.into(),
        length_scale,
        engine.camera.position.y,
    );

    // 2. Render reflected meshes
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reflected Geometry Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: engine.hdr_texture_view().unwrap_or(&view),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: engine.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep stencil from previous pass
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });

        // Render each visible surface mesh reflected
        polyscope_core::state::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !structure.is_enabled() {
                    continue;
                }
                if structure.type_name() != "SurfaceMesh" {
                    continue;
                }
                if let Some(render_data) = structure.render_data() {
                    if let Some(mesh_data) = render_data
                        .downcast_ref::<polyscope_render::surface_mesh_render::SurfaceMeshRenderData>()
                    {
                        if let Some(bind_group) = engine.create_reflected_mesh_bind_group(mesh_data) {
                            engine.render_reflected_mesh(
                                &mut render_pass,
                                &bind_group,
                                mesh_data.vertex_count(),
                            );
                        }
                    }
                }
            }
        });
    }
}
```

**Step 2: Add necessary imports at top of app.rs**

```rust
use polyscope_render::reflection;
```

**Step 3: Expose depth_view and hdr_texture_view in engine**

In `engine.rs`, add public getters:

```rust
/// Returns the depth texture view.
pub fn depth_view(&self) -> &wgpu::TextureView {
    &self.depth_view
}

/// Returns the HDR texture view if available.
pub fn hdr_texture_view(&self) -> Option<&wgpu::TextureView> {
    self.hdr_texture_view.as_ref()
}
```

**Step 4: Add vertex_count method to SurfaceMeshRenderData**

In `surface_mesh_render.rs`:

```rust
/// Returns the number of vertices.
pub fn vertex_count(&self) -> u32 {
    self.vertex_count
}
```

**Step 5: Build and test**

Run: `cargo build`
Run: `cargo run --example basic`

Expected: When switching to "Tile + Reflection" mode, should see reflected geometry on ground plane.

**Step 6: Commit**

```bash
git add crates/polyscope/src/app.rs crates/polyscope-render/src/engine.rs crates/polyscope-render/src/surface_mesh_render.rs
git commit -m "feat: integrate ground reflections into render loop"
```

---

## Task 7: Test and Polish

**Files:**
- May need minor adjustments based on testing

**Step 1: Visual testing**

Run: `cargo run --example basic`

Test cases:
1. Switch ground plane mode to "Tile + Reflection"
2. Verify reflection appears on ground
3. Adjust intensity slider - reflection should fade
4. Rotate camera - reflection should update correctly
5. Switch back to "Tile" mode - reflection should disappear

**Step 2: Test with different models**

If available, test with:
- Point cloud (should not reflect - only meshes for now)
- Multiple meshes
- Large/small models

**Step 3: Fix any visual issues**

Common issues to watch for:
- Reflection bleeding outside ground plane (stencil not working)
- Incorrect reflection position (wrong ground height)
- Missing reflection (pipeline not bound correctly)
- Dark/bright artifacts (blending issues)

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete ground reflections implementation"
```

---

## Summary

| Task | Description | Est. Complexity |
|------|-------------|-----------------|
| 1 | Change depth format to Depth24PlusStencil8 | Low |
| 2 | Create ground stencil pipeline | Medium |
| 3 | Create reflected mesh pipeline | Medium |
| 4 | Add stencil rendering method | Low |
| 5 | Add reflected mesh rendering method | Medium |
| 6 | Integrate into render loop | Medium |
| 7 | Test and polish | Low |

**Total estimated tasks:** 7 tasks with ~25-30 steps total

---

## Future Enhancements (Not in Scope)

- Reflect point clouds
- Reflect curve networks
- Fresnel effect (reflection intensity based on view angle)
- Blur/roughness for less mirror-like reflections
