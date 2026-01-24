# Shadow Mode Ground Plane Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add ShadowOnly ground plane mode that renders shadows cast by scene objects onto the ground plane, matching original C++ Polyscope functionality.

**Architecture:** Implement shadow mapping with a directional light. Render scene from light's perspective to depth texture (shadow map), then sample this map during ground plane rendering to determine shadow regions. Includes shadow blur for soft shadows.

**Tech Stack:** Rust, wgpu, WGSL shaders, glam

**Dependencies:** Requires existing ground plane implementation.

**Parallel Execution Note:** This plan is independent of tone-mapping. Can run in parallel. Do NOT run in parallel with reflections plan (both modify ground plane shader).

---

## Task 1: Add Shadow Configuration to Core

**Files:**
- Modify: `crates/polyscope-core/src/ground_plane.rs`

**Step 1: Add shadow fields to GroundPlaneConfig**

Update `GroundPlaneConfig` struct:

```rust
/// Ground plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundPlaneConfig {
    /// Rendering mode.
    pub mode: GroundPlaneMode,
    /// Height of the ground plane (Y coordinate).
    pub height: f32,
    /// Whether height is relative to scene bounds.
    pub height_is_relative: bool,
    /// Shadow blur iterations (0-5).
    pub shadow_blur_iters: u32,
    /// Shadow darkness (0.0 = no shadow, 1.0 = full black).
    pub shadow_darkness: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
            shadow_blur_iters: 2,
            shadow_darkness: 0.4,
        }
    }
}
```

**Step 2: Add ShadowOnly mode**

Update `GroundPlaneMode` enum:

```rust
/// Ground plane rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GroundPlaneMode {
    /// No ground plane.
    #[default]
    None,
    /// Tiled ground plane with subtle grid lines.
    Tile,
    /// Shadow only (no visible ground plane, just shadows).
    ShadowOnly,
}
```

**Step 3: Run tests**

Run: `cargo test -p polyscope-core`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-core/src/ground_plane.rs
git commit -m "feat(core): add ShadowOnly mode and shadow settings to GroundPlaneConfig"
```

---

## Task 2: Create Shadow Map Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/shadow_map.wgsl`

**Step 1: Create shadow map shader**

Create `crates/polyscope-render/src/shaders/shadow_map.wgsl`:

```wgsl
// Shadow map generation shader
// Renders scene depth from light's perspective

struct LightUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniforms;
@group(0) @binding(1) var<uniform> model: ModelUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = model.model * positions[vertex_index];
    out.position = light.view_proj * world_pos;

    return out;
}

// Fragment shader just writes depth (no color output needed for shadow map)
@fragment
fn fs_main(in: VertexOutput) {
    // Depth is automatically written
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/shadow_map.wgsl
git commit -m "feat(render): add shadow map generation shader"
```

---

## Task 3: Create Shadow Blur Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/shadow_blur.wgsl`

**Step 1: Create shadow blur shader**

Create `crates/polyscope-render/src/shaders/shadow_blur.wgsl`:

```wgsl
// Shadow blur shader (separable Gaussian blur)
// Used to soften shadow edges

struct BlurUniforms {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    texel_size: vec2<f32>, // 1.0 / texture_size
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: BlurUniforms;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 5-tap Gaussian blur weights
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    var result = textureSample(input_texture, input_sampler, in.uv) * weights[0];

    let offset = uniforms.direction * uniforms.texel_size;

    for (var i = 1; i < 5; i++) {
        let sample_offset = offset * f32(i);
        result += textureSample(input_texture, input_sampler, in.uv + sample_offset) * weights[i];
        result += textureSample(input_texture, input_sampler, in.uv - sample_offset) * weights[i];
    }

    return result;
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/shadow_blur.wgsl
git commit -m "feat(render): add shadow blur shader"
```

---

## Task 4: Update Ground Plane Shader for Shadows

**Files:**
- Modify: `crates/polyscope-render/src/shaders/ground_plane.wgsl`

**Step 1: Add shadow sampling to ground plane shader**

Update the shader to include shadow map sampling:

```wgsl
// Add to uniforms
struct GroundUniforms {
    center: vec4<f32>,
    basis_x: vec4<f32>,
    basis_y: vec4<f32>,
    basis_z: vec4<f32>,
    height: f32,
    length_scale: f32,
    camera_height: f32,
    up_sign: f32,
    // New shadow fields
    shadow_darkness: f32,
    shadow_mode: u32,  // 0=none, 1=shadow_only, 2=tile_with_shadow
    _padding: vec2<f32>,
}

struct LightUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

// Add bindings for shadow map
@group(0) @binding(2) var<uniform> light: LightUniforms;
@group(0) @binding(3) var shadow_map: texture_depth_2d;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;

// Shadow calculation function
fn calculate_shadow(world_pos: vec3<f32>) -> f32 {
    // Transform to light space
    let light_space_pos = light.view_proj * vec4<f32>(world_pos, 1.0);
    let proj_coords = light_space_pos.xyz / light_space_pos.w;

    // Convert from NDC [-1,1] to texture coords [0,1]
    let shadow_uv = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        -proj_coords.y * 0.5 + 0.5  // Flip Y for texture
    );

    // Current depth from light's perspective
    let current_depth = proj_coords.z;

    // PCF shadow sampling (3x3)
    var shadow = 0.0;
    let texel_size = 1.0 / 2048.0; // Shadow map resolution

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                shadow_uv + offset,
                current_depth - 0.005 // Bias
            );
        }
    }

    return shadow / 9.0;
}

// Update fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.pos_world_homog.xyz / in.pos_world_homog.w;

    // Calculate shadow
    let shadow = calculate_shadow(world_pos);
    let shadow_factor = mix(1.0, 1.0 - ground.shadow_darkness, shadow);

    // If shadow-only mode, just draw shadow
    if (ground.shadow_mode == 1u) {
        if (shadow < 0.01) {
            discard;
        }
        return vec4<f32>(0.0, 0.0, 0.0, shadow * ground.shadow_darkness);
    }

    // Otherwise, apply shadow to tile color
    // ... existing tile calculation ...
    lit_color *= shadow_factor;

    return vec4<f32>(lit_color, fade_factor);
}
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/ground_plane.wgsl
git commit -m "feat(render): add shadow sampling to ground plane shader"
```

---

## Task 5: Create Shadow Map Render Pass

**Files:**
- Create: `crates/polyscope-render/src/shadow_map.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create shadow map render resources**

Create `crates/polyscope-render/src/shadow_map.rs`:

```rust
//! Shadow map generation and blur passes.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Shadow map resolution.
pub const SHADOW_MAP_SIZE: u32 = 2048;

/// GPU representation of light uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
}

impl Default for LightUniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.0, -1.0, 0.0, 0.0],
        }
    }
}

/// Shadow map render resources.
pub struct ShadowMapPass {
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    blur_textures: [wgpu::Texture; 2],
    blur_views: [wgpu::TextureView; 2],
    light_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    blur_bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    comparison_sampler: wgpu::Sampler,
}

impl ShadowMapPass {
    /// Creates a new shadow map pass.
    pub fn new(device: &wgpu::Device) -> Self {
        // Create depth texture for shadow map
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Depth"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create blur textures (ping-pong)
        let blur_textures: [wgpu::Texture; 2] = std::array::from_fn(|i| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Shadow Blur Texture {}", i)),
                size: wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        });

        let blur_views: [wgpu::TextureView; 2] = std::array::from_fn(|i| {
            blur_textures[i].create_view(&wgpu::TextureViewDescriptor::default())
        });

        // Light uniform buffer
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: bytemuck::cast_slice(&[LightUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layout for shadow sampling
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Map Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        // Blur bind group layout
        let blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Blur Bind Group Layout"),
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

        // Samplers
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Blur Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let comparison_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        // TODO: Create shadow map and blur pipelines
        // (Simplified - actual implementation would create full pipelines)

        Self {
            depth_texture,
            depth_view,
            blur_textures,
            blur_views,
            light_buffer,
            pipeline: todo!("Create shadow map pipeline"),
            blur_pipeline: todo!("Create blur pipeline"),
            blur_bind_group_layout,
            bind_group_layout,
            sampler,
            comparison_sampler,
        }
    }

    /// Computes the light view-projection matrix.
    pub fn compute_light_matrix(
        scene_center: Vec3,
        scene_radius: f32,
        light_dir: Vec3,
    ) -> Mat4 {
        let light_pos = scene_center - light_dir.normalize() * scene_radius * 2.0;
        let view = Mat4::look_at_rh(light_pos, scene_center, Vec3::Y);
        let proj = Mat4::orthographic_rh(
            -scene_radius, scene_radius,
            -scene_radius, scene_radius,
            0.1, scene_radius * 4.0,
        );
        proj * view
    }

    /// Updates the light uniforms.
    pub fn update_light(&self, queue: &wgpu::Queue, view_proj: Mat4, light_dir: Vec3) {
        let uniforms = LightUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
        };
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the shadow map depth view.
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Returns the light uniform buffer.
    pub fn light_buffer(&self) -> &wgpu::Buffer {
        &self.light_buffer
    }

    /// Returns the comparison sampler.
    pub fn comparison_sampler(&self) -> &wgpu::Sampler {
        &self.comparison_sampler
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:

```rust
mod shadow_map;
pub use shadow_map::{ShadowMapPass, LightUniforms, SHADOW_MAP_SIZE};
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors (may have todo! panics, that's OK for now)

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shadow_map.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add ShadowMapPass structure"
```

---

## Task 6: Add Shadow UI Controls

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Update build_ground_plane_section**

Update the ground plane UI to include shadow settings:

```rust
/// Builds the ground plane settings section.
pub fn build_ground_plane_section(
    ui: &mut Ui,
    mode: &mut u32, // 0=None, 1=Tile, 2=ShadowOnly
    height: &mut f32,
    height_is_relative: &mut bool,
    shadow_blur_iters: &mut u32,
    shadow_darkness: &mut f32,
) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Ground Plane")
        .default_open(false)
        .show(ui, |ui| {
            // Mode selector
            egui::ComboBox::from_label("Mode")
                .selected_text(match *mode {
                    0 => "None",
                    1 => "Tile",
                    2 => "Shadow Only",
                    _ => "Unknown",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(mode, 0, "None").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 1, "Tile").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 2, "Shadow Only").changed() {
                        changed = true;
                    }
                });

            if *mode > 0 {
                ui.separator();

                // Height settings
                if ui.checkbox(height_is_relative, "Auto height").changed() {
                    changed = true;
                }

                if !*height_is_relative {
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        if ui.add(DragValue::new(height).speed(0.1)).changed() {
                            changed = true;
                        }
                    });
                }

                // Shadow settings (for shadow and tile modes)
                ui.separator();
                ui.label("Shadow Settings:");

                ui.horizontal(|ui| {
                    ui.label("Blur iterations:");
                    if ui.add(Slider::new(shadow_blur_iters, 0..=5)).changed() {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Darkness:");
                    if ui.add(Slider::new(shadow_darkness, 0.0..=1.0)).changed() {
                        changed = true;
                    }
                });
            }
        });

    changed
}
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add shadow settings to ground plane UI"
```

---

## Task 7: Integrate Shadow Rendering into Engine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add shadow map pass to RenderEngine**

Add to RenderEngine struct:

```rust
    shadow_map_pass: Option<ShadowMapPass>,
```

**Step 2: Initialize shadow map pass**

```rust
    if self.shadow_map_pass.is_none() {
        self.shadow_map_pass = Some(ShadowMapPass::new(&self.device));
    }
```

**Step 3: Add shadow map rendering before ground plane**

In the render loop:

```rust
    // Render shadow map if needed
    if ground_plane.mode != GroundPlaneMode::None {
        if let Some(shadow_pass) = &self.shadow_map_pass {
            // Update light matrices
            let light_dir = Vec3::new(0.5, -1.0, 0.3).normalize();
            let scene_center = /* compute from bounding box */;
            let scene_radius = /* compute from bounding box */;
            let light_matrix = ShadowMapPass::compute_light_matrix(
                scene_center, scene_radius, light_dir
            );
            shadow_pass.update_light(&self.queue, light_matrix, light_dir);

            // Render scene to shadow map
            // ... shadow rendering code ...
        }
    }
```

**Step 4: Pass shadow map to ground plane rendering**

Update ground plane bind group to include shadow map texture and sampler.

**Step 5: Run compilation check**

Run: `cargo check --workspace`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/engine.rs crates/polyscope/src/app.rs
git commit -m "feat: integrate shadow map rendering into engine"
```

---

## Task 8: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt --all`

**Step 4: Visual verification**

Run: `cargo run --example basic_demo`
Expected:
- Ground Plane mode dropdown includes "Shadow Only"
- When Shadow Only is selected, shadows appear on ground
- Shadow blur setting affects shadow softness
- Shadow darkness setting affects shadow intensity

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: finalize shadow mode implementation"
```

---

## Summary

This plan adds shadow mode ground plane with:

1. **ShadowOnly mode** in GroundPlaneMode enum
2. **Shadow map generation** from light perspective
3. **Shadow blur** for soft shadows
4. **Shadow sampling** in ground plane shader
5. **UI controls** for blur iterations and darkness
6. **Engine integration** with shadow map pass

The implementation uses:
- 2048x2048 shadow map resolution
- PCF (Percentage Closer Filtering) for smooth edges
- Separable Gaussian blur for soft shadows
- Configurable shadow darkness
