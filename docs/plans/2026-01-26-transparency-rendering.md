# Transparency Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement order-independent transparency rendering using Weighted Blended OIT, with per-structure transparency control.

**Architecture:** Two-pass approach using Weighted Blended Order-Independent Transparency (WBOIT). First pass accumulates weighted color and alpha into separate textures. Second pass composites the result. This provides correct visual appearance regardless of render order with a single geometry pass.

**Tech Stack:** wgpu, WGSL shaders, existing polyscope-render infrastructure

---

## Background: Weighted Blended OIT

The McGuire and Bavoil (2013) Weighted Blended OIT algorithm:

1. **Accumulation Pass**: Render transparent surfaces to two textures:
   - `accum` (RGBA16Float): `color.rgb * color.a * weight` and `color.a * weight`
   - `reveal` (R8Unorm): `(1 - color.a)` product (how much background shows through)

2. **Composite Pass**: Fullscreen quad combines accumulated colors:
   ```
   final_color = accum.rgb / max(accum.a, 0.0001)
   final_alpha = 1.0 - reveal
   result = final_color * final_alpha + background * (1 - final_alpha)
   ```

**Weight function** (depth-based): `weight = clamp(pow(min(1.0, color.a * 10.0) + 0.01, 3.0) * 1e8 * pow(1.0 - depth * 0.9, 3.0), 1e-2, 3e3)`

---

## Phase 1: OIT Texture Infrastructure

### Task 1.1: Add OIT Textures to RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add OIT texture fields to RenderEngine struct**

After line ~143 (after ssao fields), add:

```rust
/// OIT accumulation texture (RGBA16Float) - stores weighted color sum.
oit_accum_texture: Option<wgpu::Texture>,
/// OIT accumulation texture view.
oit_accum_view: Option<wgpu::TextureView>,
/// OIT reveal texture (R8Unorm) - stores transmittance product.
oit_reveal_texture: Option<wgpu::Texture>,
/// OIT reveal texture view.
oit_reveal_view: Option<wgpu::TextureView>,
```

**Step 2: Initialize to None in constructor**

In both `new_windowed()` and `new_headless()`, after ssao field initializations, add:

```rust
oit_accum_texture: None,
oit_accum_view: None,
oit_reveal_texture: None,
oit_reveal_view: None,
```

**Step 3: Run build to verify**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add OIT texture fields to RenderEngine"
```

---

### Task 1.2: Create OIT Texture Initialization Method

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add ensure_oit_textures method**

Add after `ensure_ssao_resources()` method:

```rust
/// Ensures OIT (Order-Independent Transparency) textures exist and match viewport size.
pub fn ensure_oit_textures(&mut self) {
    let needs_create = self.oit_accum_texture.is_none()
        || self.oit_accum_texture.as_ref().map(|t| t.width()) != Some(self.width)
        || self.oit_accum_texture.as_ref().map(|t| t.height()) != Some(self.height);

    if !needs_create {
        return;
    }

    // Accumulation texture: RGBA16Float for weighted color accumulation
    let accum_texture = self.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("OIT Accumulation Texture"),
        size: wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    self.oit_accum_view = Some(accum_texture.create_view(&wgpu::TextureViewDescriptor::default()));
    self.oit_accum_texture = Some(accum_texture);

    // Reveal texture: R8Unorm for transmittance product
    let reveal_texture = self.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("OIT Reveal Texture"),
        size: wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    self.oit_reveal_view = Some(reveal_texture.create_view(&wgpu::TextureViewDescriptor::default()));
    self.oit_reveal_texture = Some(reveal_texture);
}
```

**Step 2: Add getter methods for OIT textures**

```rust
/// Returns the OIT accumulation texture view, if initialized.
pub fn oit_accum_view(&self) -> Option<&wgpu::TextureView> {
    self.oit_accum_view.as_ref()
}

/// Returns the OIT reveal texture view, if initialized.
pub fn oit_reveal_view(&self) -> Option<&wgpu::TextureView> {
    self.oit_reveal_view.as_ref()
}
```

**Step 3: Run build to verify**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add OIT texture initialization method"
```

---

## Phase 2: OIT Composite Shader and Pass

### Task 2.1: Create OIT Composite Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/oit_composite.wgsl`

**Step 1: Write the OIT composite shader**

```wgsl
// OIT Composite Shader
// Combines accumulated weighted transparent fragments with the opaque scene.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var accum_texture: texture_2d<f32>;
@group(0) @binding(1) var reveal_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

// Fullscreen triangle vertices
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let accum = textureSample(accum_texture, texture_sampler, in.uv);
    let reveal = textureSample(reveal_texture, texture_sampler, in.uv).r;

    // If reveal is 1.0, nothing was drawn (fully transparent)
    if (reveal >= 1.0) {
        discard;
    }

    // Weighted average color
    let avg_color = accum.rgb / max(accum.a, 0.0001);

    // Final alpha is 1 - reveal (how much of the background is occluded)
    let alpha = 1.0 - reveal;

    return vec4<f32>(avg_color, alpha);
}
```

**Step 2: Run build to verify shader compiles**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/oit_composite.wgsl
git commit -m "feat(shaders): add OIT composite shader"
```

---

### Task 2.2: Create OIT Composite Pass Module

**Files:**
- Create: `crates/polyscope-render/src/oit_pass.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Write the OIT pass module**

```rust
//! Order-Independent Transparency composite pass.
//!
//! Combines the accumulated weighted transparent fragments with the opaque scene.

use wgpu::util::DeviceExt;

/// OIT composite pass resources.
pub struct OitCompositePass {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl OitCompositePass {
    /// Creates a new OIT composite pass.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader_source = include_str!("shaders/oit_composite.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("OIT Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("OIT Composite Bind Group Layout"),
            entries: &[
                // Accumulation texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Reveal texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("OIT Composite Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("OIT Composite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        // Blend transparent result over opaque scene
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
                ..Default::default()
            },
            depth_stencil: None, // No depth testing for fullscreen composite
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("OIT Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Creates a bind group for the OIT composite pass.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        accum_view: &wgpu::TextureView,
        reveal_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("OIT Composite Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(accum_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(reveal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Draws the OIT composite pass.
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, bind_group: &'a wgpu::BindGroup) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
```

**Step 2: Add to lib.rs exports**

In `crates/polyscope-render/src/lib.rs`, add:

```rust
pub mod oit_pass;
pub use oit_pass::OitCompositePass;
```

**Step 3: Run build to verify**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/oit_pass.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add OIT composite pass"
```

---

### Task 2.3: Add OIT Pass to RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add OIT pass field to RenderEngine**

After `oit_reveal_view` field, add:

```rust
/// OIT composite pass.
oit_composite_pass: Option<crate::oit_pass::OitCompositePass>,
```

**Step 2: Initialize to None in constructors**

In both `new_windowed()` and `new_headless()`:

```rust
oit_composite_pass: None,
```

**Step 3: Add ensure method for OIT pass**

Add method to `RenderEngine`:

```rust
/// Ensures OIT composite pass is initialized.
pub fn ensure_oit_pass(&mut self) {
    if self.oit_composite_pass.is_none() {
        self.oit_composite_pass = Some(crate::oit_pass::OitCompositePass::new(
            &self.device,
            self.surface_config.format,
        ));
    }
}

/// Returns the OIT composite pass, if initialized.
pub fn oit_composite_pass(&self) -> Option<&crate::oit_pass::OitCompositePass> {
    self.oit_composite_pass.as_ref()
}
```

**Step 4: Run build**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): integrate OIT composite pass into RenderEngine"
```

---

## Phase 3: Transparent Structure Rendering

### Task 3.1: Add Transparency Property to Structures

**Files:**
- Modify: `crates/polyscope-core/src/structure.rs`

**Step 1: Check if transparency field exists, if not add it**

The `Structure` trait should have transparency support. Check if `StructureOptions` has transparency, and add if needed:

```rust
/// Options common to all structures.
#[derive(Debug, Clone)]
pub struct StructureOptions {
    /// Whether the structure is enabled (visible).
    pub enabled: bool,
    /// Transparency value (0.0 = fully transparent, 1.0 = fully opaque).
    pub transparency: f32,
}

impl Default for StructureOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            transparency: 1.0,
        }
    }
}
```

**Step 2: Run build**

Run: `cargo build -p polyscope-core`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-core/src/structure.rs
git commit -m "feat(core): add transparency property to structure options"
```

---

### Task 3.2: Create OIT Accumulation Pipeline for Surface Mesh

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add OIT surface mesh pipeline field**

```rust
/// Surface mesh OIT accumulation pipeline.
mesh_oit_pipeline: Option<wgpu::RenderPipeline>,
```

**Step 2: Create the OIT pipeline**

The OIT pipeline differs from the normal pipeline:
- Outputs to two color attachments (accum + reveal)
- Uses additive blending for accum, multiplicative for reveal
- Disables depth write (but still tests depth)

Add method:

```rust
/// Creates the surface mesh OIT accumulation pipeline.
fn create_mesh_oit_pipeline(&mut self) {
    let shader_source = include_str!("shaders/surface_mesh_oit.wgsl");
    let shader_module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Surface Mesh OIT Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let bind_group_layout = self.mesh_bind_group_layout.as_ref().unwrap();
    let slice_plane_layout = &self.slice_plane_bind_group_layout;

    let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Surface Mesh OIT Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout, slice_plane_layout],
        push_constant_ranges: &[],
    });

    self.mesh_oit_pipeline = Some(self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Surface Mesh OIT Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[
                // Accumulation texture (RGBA16Float) - additive blending
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                // Reveal texture (R8Unorm) - multiplicative blending (1 - alpha)
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::OneMinusSrc,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None, // Render both sides for transparency
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: false, // Don't write depth for transparent objects
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    }));
}
```

**Step 3: Run build (will fail - shader doesn't exist yet)**

Run: `cargo build -p polyscope-render`
Expected: FAIL (shader file not found)

**Step 4: Commit partial progress**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "wip(render): add OIT pipeline structure for surface mesh"
```

---

### Task 3.3: Create Surface Mesh OIT Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/surface_mesh_oit.wgsl`

**Step 1: Create the OIT shader based on existing surface_mesh.wgsl**

The shader should:
- Output to two render targets (accum, reveal)
- Apply weight function based on depth
- Use structure transparency uniform

```wgsl
// Surface Mesh OIT Accumulation Shader
// Outputs weighted color to accumulation buffer and transmittance to reveal buffer.

// [Include same struct definitions as surface_mesh.wgsl: CameraUniforms, MeshUniforms, etc.]
// ... copy from surface_mesh.wgsl ...

struct OitOutput {
    @location(0) accum: vec4<f32>,
    @location(1) reveal: f32,
}

// Weight function for OIT (from McGuire and Bavoil 2013)
fn oit_weight(depth: f32, alpha: f32) -> f32 {
    // Depth-based weight function
    let a = min(1.0, alpha * 10.0) + 0.01;
    let b = 1.0 - depth * 0.9;
    return clamp(a * a * a * 1e8 * b * b * b, 1e-2, 3e3);
}

@fragment
fn fs_main(in: VertexOutput) -> OitOutput {
    // ... same lighting calculation as surface_mesh.wgsl ...

    let base_alpha = mesh.surface_color.a * mesh.transparency;

    // Skip fully transparent fragments
    if (base_alpha < 0.001) {
        discard;
    }

    // Compute final color with lighting
    let final_color = vec4<f32>(lit_color, base_alpha);

    // Compute weight
    let weight = oit_weight(in.clip_position.z, base_alpha);

    var out: OitOutput;
    out.accum = vec4<f32>(final_color.rgb * final_color.a, final_color.a) * weight;
    out.reveal = final_color.a;

    return out;
}
```

**Note:** The actual shader will need to copy the full vertex shader and lighting code from `surface_mesh.wgsl`. This is a sketch showing the key differences.

**Step 2: Run build**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/surface_mesh_oit.wgsl
git commit -m "feat(shaders): add surface mesh OIT accumulation shader"
```

---

## Phase 4: Render Loop Integration

### Task 4.1: Add Transparency Mode to Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Check transparency mode and route to appropriate render path**

In the main render function, add logic to:
1. Check `options.transparency_mode`
2. For `WeightedBlended`:
   - First render opaque objects normally
   - Then render transparent objects to OIT buffers
   - Composite OIT result over scene
3. For `Simple`: use current behavior
4. For `None`: disable alpha blending

**Step 2: Add OIT render pass**

```rust
// After opaque rendering, if using WeightedBlended OIT:
if transparency_mode == TransparencyMode::WeightedBlended {
    // Ensure OIT resources exist
    engine.ensure_oit_textures();
    engine.ensure_oit_pass();

    // Clear OIT buffers
    // accum: (0, 0, 0, 0)
    // reveal: 1.0 (fully transparent initially)

    // Render transparent structures to OIT buffers
    {
        let mut oit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("OIT Accumulation Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: engine.oit_accum_view().unwrap(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: engine.oit_reveal_view().unwrap(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), // 1.0 = fully transparent
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &engine.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep opaque depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        // Render transparent structures with OIT pipeline
        // ...
    }

    // Composite OIT result over opaque scene
    {
        let oit_bind_group = engine.oit_composite_pass().unwrap().create_bind_group(
            &engine.device,
            engine.oit_accum_view().unwrap(),
            engine.oit_reveal_view().unwrap(),
        );

        let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("OIT Composite Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &hdr_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep opaque content
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        engine.oit_composite_pass().unwrap().draw(&mut composite_pass, &oit_bind_group);
    }
}
```

**Step 3: Run build**

Run: `cargo build -p polyscope`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: integrate OIT rendering into main render loop"
```

---

## Phase 5: Per-Structure Transparency UI

### Task 5.1: Add Transparency Slider to Structure Panel

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Add transparency slider to structure options**

In the structure options panel, add:

```rust
// Transparency slider
ui.horizontal(|ui| {
    ui.label("Transparency:");
    if ui.add(egui::Slider::new(&mut options.transparency, 0.0..=1.0)).changed() {
        // Mark structure as needing update
    }
});
```

**Step 2: Run build**

Run: `cargo build -p polyscope-ui`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add transparency slider to structure panel"
```

---

## Phase 6: Testing and Demo

### Task 6.1: Create Transparency Demo

**Files:**
- Create: `examples/transparency_demo.rs`

**Step 1: Write demo showing transparency features**

```rust
//! Transparency Demo
//!
//! Demonstrates order-independent transparency rendering.

use glam::Vec3;
use polyscope::*;

fn main() {
    init();

    // Create overlapping transparent meshes
    let (vertices1, indices1) = create_sphere(Vec3::new(-0.3, 0.0, 0.0), 0.5, 16);
    let mut mesh1 = register_surface_mesh("sphere1", vertices1, indices1);
    mesh1.set_color(Vec3::new(1.0, 0.0, 0.0)); // Red
    mesh1.set_transparency(0.5);

    let (vertices2, indices2) = create_sphere(Vec3::new(0.3, 0.0, 0.0), 0.5, 16);
    let mut mesh2 = register_surface_mesh("sphere2", vertices2, indices2);
    mesh2.set_color(Vec3::new(0.0, 0.0, 1.0)); // Blue
    mesh2.set_transparency(0.5);

    // Set transparency mode
    set_transparency_mode(TransparencyMode::WeightedBlended);

    show();
}

fn create_sphere(center: Vec3, radius: f32, segments: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    // Generate sphere geometry
    // ...
}
```

**Step 2: Run demo**

Run: `cargo run --example transparency_demo`
Expected: Two overlapping transparent spheres rendering correctly

**Step 3: Commit**

```bash
git add examples/transparency_demo.rs
git commit -m "example: add transparency demo"
```

---

## Summary

This plan implements Weighted Blended Order-Independent Transparency (WBOIT):

1. **Phase 1**: Create OIT accumulation and reveal textures
2. **Phase 2**: Create OIT composite shader and pass
3. **Phase 3**: Create OIT-specific render pipelines for structures
4. **Phase 4**: Integrate OIT rendering into the main render loop
5. **Phase 5**: Add per-structure transparency UI controls
6. **Phase 6**: Create demo and test

**Key advantages of WBOIT over depth peeling:**
- Single geometry pass (better performance)
- No iteration count to tune
- Works well with wgpu's pipeline model
- Handles many overlapping layers gracefully

**Limitations:**
- Approximate (not exact order) - but visually acceptable for most cases
- Weight function may need tuning for specific use cases
