# SSAO (Screen Space Ambient Occlusion) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add screen-space ambient occlusion to improve depth perception and visual quality by darkening areas where geometry is close together.

**Architecture:** Multi-pass approach: (1) Render view-space normals to G-buffer during geometry pass, (2) SSAO compute pass samples depth buffer in hemisphere, (3) Blur pass smooths result, (4) Apply SSAO factor during tone mapping. Uses existing post-processing pattern from ToneMapPass.

**Tech Stack:** wgpu, WGSL shaders, existing tone mapping infrastructure

---

## Overview

SSAO approximates ambient occlusion by sampling the depth buffer around each pixel. The algorithm:

1. For each pixel, reconstruct view-space position from depth
2. Sample random points in a hemisphere oriented by the surface normal
3. For each sample, check if it's occluded (behind geometry)
4. Darken pixels with high occlusion count
5. Blur to reduce noise

---

## Task 1: Add SSAO Configuration Options

**Files:**
- Create: `crates/polyscope-core/src/ssao.rs`
- Modify: `crates/polyscope-core/src/lib.rs`
- Modify: `crates/polyscope-core/src/options.rs`

**Step 1: Create SSAO configuration struct**

Create `crates/polyscope-core/src/ssao.rs`:

```rust
//! SSAO (Screen Space Ambient Occlusion) configuration.

use serde::{Deserialize, Serialize};

/// SSAO configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsaoConfig {
    /// Whether SSAO is enabled.
    pub enabled: bool,
    /// Sample radius in world units (relative to length scale).
    pub radius: f32,
    /// Intensity/strength of the effect (0.0 = none, 1.0 = full).
    pub intensity: f32,
    /// Bias to prevent self-occlusion artifacts.
    pub bias: f32,
    /// Number of samples per pixel (higher = better quality, slower).
    pub sample_count: u32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 0.5,
            intensity: 1.0,
            bias: 0.025,
            sample_count: 16,
        }
    }
}
```

**Step 2: Export from lib.rs**

In `crates/polyscope-core/src/lib.rs`, add:
```rust
pub mod ssao;
pub use ssao::SsaoConfig;
```

**Step 3: Add to Options struct**

In `crates/polyscope-core/src/options.rs`, add field to `Options`:
```rust
/// SSAO configuration.
pub ssao: SsaoConfig,
```

And in Default impl:
```rust
ssao: SsaoConfig::default(),
```

**Step 4: Run tests**

Run: `cargo test -p polyscope-core`
Expected: Tests pass

**Step 5: Commit**

```bash
git add crates/polyscope-core/src/ssao.rs crates/polyscope-core/src/lib.rs crates/polyscope-core/src/options.rs
git commit -m "feat: add SSAO configuration options"
```

---

## Task 2: Create Normal G-Buffer Texture

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Add a texture to store view-space normals for SSAO sampling.

**Step 1: Add normal texture fields**

In RenderEngine struct, add after `hdr_view`:
```rust
/// Normal G-buffer texture for SSAO.
normal_texture: Option<wgpu::Texture>,
/// Normal G-buffer texture view.
normal_view: Option<wgpu::TextureView>,
```

**Step 2: Initialize in constructors**

In both `new()` and `new_headless()`:
```rust
normal_texture: None,
normal_view: None,
```

**Step 3: Create normal texture creation method**

Add method:
```rust
/// Creates the normal G-buffer texture for SSAO.
fn create_normal_texture(&mut self) {
    let normal_texture = self.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Normal G-Buffer"),
        size: wgpu::Extent3d {
            width: self.surface_config.width,
            height: self.surface_config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float, // View-space normals (xyz) + unused (w)
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());

    self.normal_texture = Some(normal_texture);
    self.normal_view = Some(normal_view);
}
```

**Step 4: Call in init and resize**

In `init_tone_mapping()`, after `self.create_hdr_texture();`:
```rust
self.create_normal_texture();
```

In `resize()`, after `self.create_hdr_texture();`:
```rust
self.create_normal_texture();
```

**Step 5: Add getter**

```rust
/// Returns the normal G-buffer view if available.
pub fn normal_view(&self) -> Option<&wgpu::TextureView> {
    self.normal_view.as_ref()
}
```

**Step 6: Build and test**

Run: `cargo build`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat: add normal G-buffer texture for SSAO"
```

---

## Task 3: Modify Surface Mesh Shader for Normal Output

**Files:**
- Modify: `crates/polyscope-render/src/shaders/surface_mesh.wgsl`

Update the surface mesh shader to output view-space normals to a second render target.

**Step 1: Add second output to fragment shader**

Change the fragment output struct:
```wgsl
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}
```

**Step 2: Modify fs_main to return struct**

Change the fragment shader signature and return:
```wgsl
@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> FragmentOutput {
    // ... existing code ...

    // At the end, before return:
    // Compute view-space normal
    let view_normal = (camera.view * vec4<f32>(normal, 0.0)).xyz;

    var out: FragmentOutput;
    out.color = vec4<f32>(final_color, alpha);
    out.normal = vec4<f32>(view_normal * 0.5 + 0.5, 1.0); // Encode to [0,1] range
    return out;
}
```

Note: The `normal` variable should be the world-space normal used for lighting (after front_facing flip).

**Step 3: Build to verify shader compiles**

Run: `cargo build`
Expected: Compiles (pipeline may need updating - see next task)

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shaders/surface_mesh.wgsl
git commit -m "feat: add view-space normal output to surface mesh shader"
```

---

## Task 4: Update Surface Mesh Pipeline for MRT

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Update the surface mesh pipeline to support multiple render targets (MRT).

**Step 1: Find surface mesh pipeline creation**

Locate `create_surface_mesh_pipeline()` or the pipeline creation code.

**Step 2: Add second color target**

Change the `targets` array in the pipeline descriptor:
```rust
fragment: Some(wgpu::FragmentState {
    module: &shader,
    entry_point: Some("fs_main"),
    targets: &[
        // Color output (HDR)
        Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba16Float,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        // Normal output (G-buffer)
        Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ],
    compilation_options: Default::default(),
}),
```

**Step 3: Update render pass to include normal attachment**

In `render_surface_mesh()` or wherever the render pass is created, add the normal attachment:
```rust
color_attachments: &[
    Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    }),
    Some(wgpu::RenderPassColorAttachment {
        view: normal_view, // self.normal_view.as_ref().unwrap()
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load, // Or Clear(0,0,0,0) for first mesh
            store: wgpu::StoreOp::Store,
        },
    }),
],
```

**Step 4: Build and test**

Run: `cargo build`
Run: `cargo run --example basic`
Expected: Renders normally (normal buffer populated but not yet used)

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat: update surface mesh pipeline for MRT with normal output"
```

---

## Task 5: Create SSAO Noise Texture

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

Create a small noise texture with random rotation vectors for SSAO sampling.

**Step 1: Add noise texture fields**

```rust
/// SSAO noise texture (4x4 random rotation vectors).
ssao_noise_texture: Option<wgpu::Texture>,
/// SSAO noise texture view.
ssao_noise_view: Option<wgpu::TextureView>,
```

**Step 2: Initialize fields**

```rust
ssao_noise_texture: None,
ssao_noise_view: None,
```

**Step 3: Create noise texture method**

```rust
/// Creates the SSAO noise texture.
fn create_ssao_noise_texture(&mut self) {
    use rand::Rng;

    // Generate 4x4 random rotation vectors
    let mut rng = rand::thread_rng();
    let mut noise_data = Vec::with_capacity(4 * 4 * 4); // 4x4 pixels, RGBA f16

    for _ in 0..16 {
        // Random rotation vector in tangent plane (z=0)
        let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let x = angle.cos();
        let y = angle.sin();
        // Store as f16 (using half crate or manual conversion)
        // For simplicity, use Rgba8Unorm and encode in [0,1]
        noise_data.push(((x * 0.5 + 0.5) * 255.0) as u8);
        noise_data.push(((y * 0.5 + 0.5) * 255.0) as u8);
        noise_data.push(0u8); // z = 0
        noise_data.push(255u8); // w = 1
    }

    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SSAO Noise Texture"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    self.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &noise_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * 4),
            rows_per_image: Some(4),
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    self.ssao_noise_texture = Some(texture);
    self.ssao_noise_view = Some(view);
}
```

**Step 4: Add rand dependency**

In `crates/polyscope-render/Cargo.toml`:
```toml
rand = "0.8"
```

**Step 5: Call creation in initialization**

After `self.create_normal_texture();`:
```rust
self.create_ssao_noise_texture();
```

**Step 6: Build**

Run: `cargo build`
Expected: Compiles

**Step 7: Commit**

```bash
git add crates/polyscope-render/src/engine.rs crates/polyscope-render/Cargo.toml
git commit -m "feat: add SSAO noise texture generation"
```

---

## Task 6: Create SSAO Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/ssao.wgsl`

**Step 1: Create the SSAO shader**

```wgsl
// SSAO (Screen Space Ambient Occlusion) shader
// Samples depth buffer in hemisphere around each pixel to estimate occlusion

struct SsaoUniforms {
    // Projection matrix for depth reconstruction
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    // SSAO parameters
    radius: f32,
    bias: f32,
    intensity: f32,
    sample_count: u32,
    // Screen dimensions for noise tiling
    screen_width: f32,
    screen_height: f32,
    _padding: vec2<f32>,
}

// Hemisphere sample kernel (precomputed, oriented along +Z)
// These are positions in tangent space to sample around each pixel
const KERNEL_SIZE: u32 = 64u;
var<private> kernel: array<vec3<f32>, 64> = array<vec3<f32>, 64>(
    vec3<f32>(0.04977, -0.04471, 0.04996),
    vec3<f32>(0.01457, 0.01653, 0.00224),
    vec3<f32>(-0.04065, -0.01937, 0.03193),
    vec3<f32>(0.01378, -0.09158, 0.04092),
    vec3<f32>(0.05599, 0.05979, 0.05766),
    vec3<f32>(0.09227, 0.04428, 0.01545),
    vec3<f32>(-0.00204, -0.05828, 0.14464),
    vec3<f32>(-0.00033, -0.00019, 0.00037),
    vec3<f32>(0.05004, -0.04665, 0.02538),
    vec3<f32>(-0.03886, 0.09849, 0.00118),
    vec3<f32>(-0.00184, -0.01569, 0.00531),
    vec3<f32>(-0.08395, -0.01566, 0.04852),
    vec3<f32>(-0.00880, -0.00226, 0.00454),
    vec3<f32>(-0.08089, -0.08662, 0.03873),
    vec3<f32>(-0.00307, 0.00372, 0.00100),
    vec3<f32>(-0.01425, 0.08400, 0.08076),
    // Add more samples... (truncated for brevity - full kernel should have 64 samples)
    // In practice, generate these programmatically or use a well-distributed set
    vec3<f32>(0.0, 0.0, 0.1),
    vec3<f32>(0.1, 0.0, 0.1),
    vec3<f32>(0.0, 0.1, 0.1),
    vec3<f32>(-0.1, 0.0, 0.1),
    vec3<f32>(0.0, -0.1, 0.1),
    vec3<f32>(0.07, 0.07, 0.1),
    vec3<f32>(-0.07, 0.07, 0.1),
    vec3<f32>(0.07, -0.07, 0.1),
    vec3<f32>(-0.07, -0.07, 0.1),
    vec3<f32>(0.0, 0.0, 0.2),
    vec3<f32>(0.14, 0.0, 0.14),
    vec3<f32>(0.0, 0.14, 0.14),
    vec3<f32>(-0.14, 0.0, 0.14),
    vec3<f32>(0.0, -0.14, 0.14),
    vec3<f32>(0.1, 0.1, 0.14),
    vec3<f32>(-0.1, 0.1, 0.14),
    vec3<f32>(0.1, -0.1, 0.14),
    vec3<f32>(-0.1, -0.1, 0.14),
    vec3<f32>(0.0, 0.0, 0.3),
    vec3<f32>(0.2, 0.0, 0.2),
    vec3<f32>(0.0, 0.2, 0.2),
    vec3<f32>(-0.2, 0.0, 0.2),
    vec3<f32>(0.0, -0.2, 0.2),
    vec3<f32>(0.14, 0.14, 0.2),
    vec3<f32>(-0.14, 0.14, 0.2),
    vec3<f32>(0.14, -0.14, 0.2),
    vec3<f32>(-0.14, -0.14, 0.2),
    vec3<f32>(0.0, 0.0, 0.4),
    vec3<f32>(0.25, 0.0, 0.25),
    vec3<f32>(0.0, 0.25, 0.25),
    vec3<f32>(-0.25, 0.0, 0.25),
    vec3<f32>(0.0, -0.25, 0.25),
    vec3<f32>(0.18, 0.18, 0.25),
    vec3<f32>(-0.18, 0.18, 0.25),
    vec3<f32>(0.18, -0.18, 0.25),
    vec3<f32>(-0.18, -0.18, 0.25),
    vec3<f32>(0.0, 0.0, 0.5),
    vec3<f32>(0.3, 0.0, 0.3),
    vec3<f32>(0.0, 0.3, 0.3),
    vec3<f32>(-0.3, 0.0, 0.3),
    vec3<f32>(0.0, -0.3, 0.3),
    vec3<f32>(0.2, 0.2, 0.3),
    vec3<f32>(-0.2, 0.2, 0.3),
    vec3<f32>(0.2, -0.2, 0.3),
    vec3<f32>(-0.2, -0.2, 0.3),
);

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var noise_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> uniforms: SsaoUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Reconstruct view-space position from depth and UV
fn view_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Convert UV to clip space
    let clip = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    // Unproject to view space
    let view = uniforms.inv_proj * clip;
    return view.xyz / view.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = vec2<f32>(1.0 / uniforms.screen_width, 1.0 / uniforms.screen_height);

    // Sample depth and normal
    let depth = textureSample(depth_texture, tex_sampler, in.uv);
    if (depth >= 1.0) {
        // Sky/background - no occlusion
        return vec4<f32>(1.0);
    }

    let normal_sample = textureSample(normal_texture, tex_sampler, in.uv);
    let normal = normalize(normal_sample.xyz * 2.0 - 1.0); // Decode from [0,1] to [-1,1]

    // Reconstruct view-space position
    let frag_pos = view_pos_from_depth(in.uv, depth);

    // Sample noise for random rotation (tile across screen)
    let noise_scale = vec2<f32>(uniforms.screen_width / 4.0, uniforms.screen_height / 4.0);
    let noise_uv = in.uv * noise_scale;
    let random_vec = textureSample(noise_texture, tex_sampler, noise_uv).xyz * 2.0 - 1.0;

    // Create TBN matrix to orient hemisphere along normal
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    // Accumulate occlusion
    var occlusion = 0.0;
    let sample_count = min(uniforms.sample_count, KERNEL_SIZE);

    for (var i = 0u; i < sample_count; i++) {
        // Get sample position in view space
        let sample_dir = tbn * kernel[i];
        let sample_pos = frag_pos + sample_dir * uniforms.radius;

        // Project sample to screen space
        let offset = uniforms.proj * vec4<f32>(sample_pos, 1.0);
        let offset_uv = (offset.xy / offset.w) * 0.5 + 0.5;

        // Sample depth at this position
        let sample_depth = textureSample(depth_texture, tex_sampler, vec2<f32>(offset_uv.x, 1.0 - offset_uv.y));
        let sample_view_pos = view_pos_from_depth(vec2<f32>(offset_uv.x, 1.0 - offset_uv.y), sample_depth);

        // Range check and accumulate
        let range_check = smoothstep(0.0, 1.0, uniforms.radius / abs(frag_pos.z - sample_view_pos.z));
        if (sample_view_pos.z >= sample_pos.z + uniforms.bias) {
            occlusion += range_check;
        }
    }

    // Average and invert
    occlusion = 1.0 - (occlusion / f32(sample_count));

    // Apply intensity
    occlusion = pow(occlusion, uniforms.intensity);

    return vec4<f32>(occlusion, occlusion, occlusion, 1.0);
}
```

**Step 2: Verify shader syntax**

Run: `cargo build`
Expected: Compiles (shader included but not yet used)

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/ssao.wgsl
git commit -m "feat: add SSAO shader"
```

---

## Task 7: Create SSAO Blur Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/ssao_blur.wgsl`

**Step 1: Create blur shader**

```wgsl
// SSAO blur shader
// Applies bilateral blur to smooth SSAO while preserving edges

struct BlurUniforms {
    texel_size: vec2<f32>,
    blur_scale: f32,
    _padding: f32,
}

@group(0) @binding(0) var ssao_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: BlurUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 4x4 box blur
    var result = 0.0;
    for (var x = -2; x < 2; x++) {
        for (var y = -2; y < 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * uniforms.texel_size * uniforms.blur_scale;
            result += textureSample(ssao_texture, tex_sampler, in.uv + offset).r;
        }
    }
    result /= 16.0;

    return vec4<f32>(result, result, result, 1.0);
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/ssao_blur.wgsl
git commit -m "feat: add SSAO blur shader"
```

---

## Task 8: Create SSAO Pass Module

**Files:**
- Create: `crates/polyscope-render/src/ssao_pass.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create SSAO pass module**

Create `crates/polyscope-render/src/ssao_pass.rs`:

```rust
//! SSAO (Screen Space Ambient Occlusion) rendering pass.

use glam::Mat4;
use wgpu::util::DeviceExt;

/// GPU representation of SSAO uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoUniforms {
    pub proj: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub sample_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub _padding: [f32; 2],
}

impl Default for SsaoUniforms {
    fn default() -> Self {
        Self {
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
            radius: 0.5,
            bias: 0.025,
            intensity: 1.0,
            sample_count: 16,
            screen_width: 1280.0,
            screen_height: 720.0,
            _padding: [0.0; 2],
        }
    }
}

/// SSAO blur uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoBlurUniforms {
    pub texel_size: [f32; 2],
    pub blur_scale: f32,
    pub _padding: f32,
}

/// SSAO pass resources.
pub struct SsaoPass {
    // Main SSAO pass
    ssao_pipeline: wgpu::RenderPipeline,
    ssao_bind_group_layout: wgpu::BindGroupLayout,
    ssao_uniform_buffer: wgpu::Buffer,
    // Blur pass
    blur_pipeline: wgpu::RenderPipeline,
    blur_bind_group_layout: wgpu::BindGroupLayout,
    blur_uniform_buffer: wgpu::Buffer,
    // Intermediate texture for blur
    ssao_texture: wgpu::Texture,
    ssao_view: wgpu::TextureView,
    // Sampler
    sampler: wgpu::Sampler,
}

impl SsaoPass {
    /// Creates a new SSAO pass.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // Create SSAO shader
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSAO Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ssao.wgsl").into()),
        });

        // Create blur shader
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSAO Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ssao_blur.wgsl").into()),
        });

        // SSAO bind group layout
        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Bind Group Layout"),
                entries: &[
                    // Depth texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Normal texture
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
                    // Noise texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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

        // SSAO pipeline
        let ssao_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Pipeline Layout"),
                bind_group_layouts: &[&ssao_bind_group_layout],
                push_constant_ranges: &[],
            });

        let ssao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO Pipeline"),
            layout: Some(&ssao_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ssao_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssao_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm, // Single channel for occlusion
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Blur bind group layout
        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Blur Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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

        // Blur pipeline
        let blur_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Blur Pipeline Layout"),
                bind_group_layouts: &[&blur_bind_group_layout],
                push_constant_ranges: &[],
            });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create uniform buffers
        let ssao_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SSAO Uniform Buffer"),
            contents: bytemuck::cast_slice(&[SsaoUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let blur_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SSAO Blur Uniform Buffer"),
            contents: bytemuck::cast_slice(&[SsaoBlurUniforms {
                texel_size: [1.0 / width as f32, 1.0 / height as f32],
                blur_scale: 1.0,
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create intermediate SSAO texture
        let ssao_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ssao_view = ssao_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SSAO Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            ssao_pipeline,
            ssao_bind_group_layout,
            ssao_uniform_buffer,
            blur_pipeline,
            blur_bind_group_layout,
            blur_uniform_buffer,
            ssao_texture,
            ssao_view,
            sampler,
        }
    }

    /// Resizes the SSAO textures.
    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        // Recreate SSAO texture
        self.ssao_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.ssao_view = self
            .ssao_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update blur uniforms
        queue.write_buffer(
            &self.blur_uniform_buffer,
            0,
            bytemuck::cast_slice(&[SsaoBlurUniforms {
                texel_size: [1.0 / width as f32, 1.0 / height as f32],
                blur_scale: 1.0,
                _padding: 0.0,
            }]),
        );
    }

    /// Updates SSAO uniforms.
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        proj: Mat4,
        inv_proj: Mat4,
        radius: f32,
        bias: f32,
        intensity: f32,
        sample_count: u32,
        width: f32,
        height: f32,
    ) {
        let uniforms = SsaoUniforms {
            proj: proj.to_cols_array_2d(),
            inv_proj: inv_proj.to_cols_array_2d(),
            radius,
            bias,
            intensity,
            sample_count,
            screen_width: width,
            screen_height: height,
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.ssao_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Creates a bind group for the SSAO pass.
    pub fn create_ssao_bind_group(
        &self,
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        noise_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Bind Group"),
            layout: &self.ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.ssao_uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Creates a bind group for the blur pass.
    pub fn create_blur_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Blur Bind Group"),
            layout: &self.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.blur_uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Renders the SSAO pass.
    pub fn render_ssao(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SSAO Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.ssao_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.ssao_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    /// Renders the blur pass to the output texture.
    pub fn render_blur(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SSAO Blur Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.blur_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    /// Returns the blurred SSAO texture view.
    pub fn ssao_view(&self) -> &wgpu::TextureView {
        &self.ssao_view
    }
}
```

**Step 2: Export from lib.rs**

In `crates/polyscope-render/src/lib.rs`:
```rust
pub mod ssao_pass;
pub use ssao_pass::{SsaoPass, SsaoUniforms};
```

**Step 3: Build**

Run: `cargo build`
Expected: Compiles

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/ssao_pass.rs crates/polyscope-render/src/lib.rs
git commit -m "feat: add SSAO pass module with blur"
```

---

## Task 9: Integrate SSAO into Render Loop

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope/src/app.rs`

This task integrates SSAO into the main render loop. Due to complexity, this is outlined at a higher level.

**Step 1: Add SSAO pass to engine**

In engine.rs, add:
```rust
/// SSAO pass.
ssao_pass: Option<crate::ssao_pass::SsaoPass>,
/// SSAO output texture (blurred result).
ssao_output_texture: Option<wgpu::Texture>,
ssao_output_view: Option<wgpu::TextureView>,
```

**Step 2: Initialize SSAO pass**

Create `init_ssao_pass()` method and call it in initialization.

**Step 3: Add render_ssao method**

Method that runs SSAO computation and blur passes.

**Step 4: Modify tone mapping to incorporate SSAO**

Either multiply SSAO into the color during tone mapping, or add a separate composite pass.

**Step 5: Add UI controls**

In `crates/polyscope-ui/src/panels.rs`, add SSAO controls to the View section.

**Step 6: Wire up in app.rs**

Call SSAO rendering after geometry pass, before tone mapping.

**Step 7: Test thoroughly**

Visual testing with various models and settings.

**Step 8: Commit**

```bash
git add -A
git commit -m "feat: integrate SSAO into render loop"
```

---

## Task 10: Polish and Test

**Step 1: Visual testing**

- Test with various models
- Verify occlusion appears in corners/crevices
- Check performance impact

**Step 2: Tune default parameters**

Adjust radius, bias, intensity for good visual quality.

**Step 3: Add documentation**

Document SSAO in architecture-differences.md.

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete SSAO implementation"
```

---

## Summary

| Task | Description | Complexity |
|------|-------------|------------|
| 1 | SSAO configuration options | Low |
| 2 | Normal G-buffer texture | Low |
| 3 | Surface mesh shader MRT | Medium |
| 4 | Surface mesh pipeline MRT | Medium |
| 5 | Noise texture | Low |
| 6 | SSAO shader | High |
| 7 | Blur shader | Low |
| 8 | SSAO pass module | High |
| 9 | Render loop integration | High |
| 10 | Polish and test | Medium |

**Total estimated tasks:** 10 tasks with ~40-50 steps total

---

## Future Enhancements (Not in Scope)

- Horizon-Based Ambient Occlusion (HBAO+) for better quality
- GTAO (Ground Truth Ambient Occlusion)
- Temporal filtering to reduce noise
- Half-resolution rendering for performance
- Point cloud SSAO (would need different approach)
