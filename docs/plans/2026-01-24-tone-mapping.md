# Tone Mapping Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add tone mapping post-processing with exposure, gamma, and white level controls to match original C++ Polyscope rendering quality.

**Architecture:** Render scene to an HDR intermediate texture, then apply a fullscreen tone mapping pass that converts to LDR with configurable exposure, gamma correction, and white level. UI controls in the Appearance section.

**Tech Stack:** Rust, wgpu, WGSL shaders, egui

**Dependencies:** None - can be implemented independently.

**Parallel Execution Note:** This plan is fully independent and can run in parallel with shadow-mode and reflections plans.

---

## Task 1: Add Tone Mapping Configuration to Core

**Files:**
- Create: `crates/polyscope-core/src/tone_mapping.rs`
- Modify: `crates/polyscope-core/src/lib.rs`

**Step 1: Create tone mapping configuration**

Create `crates/polyscope-core/src/tone_mapping.rs`:

```rust
//! Tone mapping configuration.

use serde::{Deserialize, Serialize};

/// Tone mapping configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneMappingConfig {
    /// Exposure multiplier (default 1.0).
    pub exposure: f32,
    /// White level for highlight compression (default 1.0).
    pub white_level: f32,
    /// Gamma correction exponent (default 2.2).
    pub gamma: f32,
}

impl Default for ToneMappingConfig {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            white_level: 1.0,
            gamma: 2.2,
        }
    }
}

impl ToneMappingConfig {
    /// Creates a new tone mapping configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the exposure value.
    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.exposure = exposure;
        self
    }

    /// Sets the white level.
    pub fn with_white_level(mut self, white_level: f32) -> Self {
        self.white_level = white_level;
        self
    }

    /// Sets the gamma value.
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tone_mapping_default() {
        let config = ToneMappingConfig::default();
        assert_eq!(config.exposure, 1.0);
        assert_eq!(config.white_level, 1.0);
        assert_eq!(config.gamma, 2.2);
    }

    #[test]
    fn test_tone_mapping_builder() {
        let config = ToneMappingConfig::new()
            .with_exposure(1.5)
            .with_gamma(2.0);
        assert_eq!(config.exposure, 1.5);
        assert_eq!(config.gamma, 2.0);
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-core/src/lib.rs`:

```rust
mod tone_mapping;
pub use tone_mapping::ToneMappingConfig;
```

**Step 3: Run tests**

Run: `cargo test -p polyscope-core`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-core/src/tone_mapping.rs crates/polyscope-core/src/lib.rs
git commit -m "feat(core): add ToneMappingConfig"
```

---

## Task 2: Create Tone Mapping Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/tone_map.wgsl`

**Step 1: Create the tone mapping shader**

Create `crates/polyscope-render/src/shaders/tone_map.wgsl`:

```wgsl
// Tone mapping post-processing shader
// Applies exposure, Reinhard tone mapping, and gamma correction

struct ToneMapUniforms {
    exposure: f32,
    white_level: f32,
    gamma: f32,
    _padding: f32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: ToneMapUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle (covers [-1,1] x [-1,1])
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // Convert from clip space [-1,1] to UV space [0,1]
    // Note: Y is flipped for texture sampling
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return out;
}

// Reinhard tone mapping with white point
fn reinhard_extended(color: vec3<f32>, white: f32) -> vec3<f32> {
    let white_sq = white * white;
    let numerator = color * (1.0 + color / white_sq);
    return numerator / (1.0 + color);
}

// Gamma correction
fn gamma_correct(color: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / gamma));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the HDR input
    let hdr_color = textureSample(input_texture, input_sampler, in.uv);

    // Apply exposure
    var color = hdr_color.rgb * uniforms.exposure;

    // Apply Reinhard tone mapping with white point
    color = reinhard_extended(color, uniforms.white_level);

    // Apply gamma correction
    color = gamma_correct(color, uniforms.gamma);

    // Clamp to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(color, hdr_color.a);
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/tone_map.wgsl
git commit -m "feat(render): add tone mapping WGSL shader"
```

---

## Task 3: Create Tone Mapping Render Pass

**Files:**
- Create: `crates/polyscope-render/src/tone_mapping.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create tone mapping render resources**

Create `crates/polyscope-render/src/tone_mapping.rs`:

```rust
//! Tone mapping post-processing pass.

use wgpu::util::DeviceExt;

/// GPU representation of tone mapping uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ToneMapUniforms {
    pub exposure: f32,
    pub white_level: f32,
    pub gamma: f32,
    pub _padding: f32,
}

impl Default for ToneMapUniforms {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            white_level: 1.0,
            gamma: 2.2,
            _padding: 0.0,
        }
    }
}

/// Tone mapping render resources.
pub struct ToneMapPass {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl ToneMapPass {
    /// Creates a new tone mapping pass.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tone Map Bind Group Layout"),
            entries: &[
                // Input texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
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

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tone Map Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tone_map.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tone Map Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tone Map Pipeline"),
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
                    format: output_format,
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

        // Create uniform buffer
        let uniforms = ToneMapUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tone Map Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Tone Map Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            sampler,
        }
    }

    /// Updates the tone mapping uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, exposure: f32, white_level: f32, gamma: f32) {
        let uniforms = ToneMapUniforms {
            exposure,
            white_level,
            gamma,
            _padding: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Creates a bind group for rendering.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        input_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tone Map Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Renders the tone mapping pass.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tone Map Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Fullscreen triangle
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:

```rust
mod tone_mapping;
pub use tone_mapping::{ToneMapPass, ToneMapUniforms};
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/tone_mapping.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add ToneMapPass for post-processing"
```

---

## Task 4: Add HDR Intermediate Texture to RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add HDR texture fields to RenderEngine**

Add to the RenderEngine struct:

```rust
    // HDR rendering
    hdr_texture: Option<wgpu::Texture>,
    hdr_view: Option<wgpu::TextureView>,
    tone_map_pass: Option<ToneMapPass>,
```

**Step 2: Create HDR texture in resize or initialization**

Add method to create HDR texture:

```rust
    fn create_hdr_texture(&mut self) {
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // HDR format
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.hdr_texture = Some(hdr_texture);
        self.hdr_view = Some(hdr_view);
    }
```

**Step 3: Initialize tone map pass**

In the initialization or when first needed:

```rust
    if self.tone_map_pass.is_none() {
        self.tone_map_pass = Some(ToneMapPass::new(&self.device, self.surface_config.format));
    }
```

**Step 4: Call create_hdr_texture in resize()**

```rust
    pub fn resize(&mut self, width: u32, height: u32) {
        // ... existing resize code ...

        // Recreate HDR texture
        self.create_hdr_texture();
    }
```

**Step 5: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add HDR texture and tone map pass to engine"
```

---

## Task 5: Add Tone Mapping UI Controls

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Add ToneMappingSettings struct**

Add after AppearanceSettings:

```rust
/// Tone mapping settings for UI.
#[derive(Debug, Clone)]
pub struct ToneMappingSettings {
    /// Whether tone mapping is enabled.
    pub enabled: bool,
    /// Exposure value (0.1 - 4.0).
    pub exposure: f32,
    /// White level (0.5 - 4.0).
    pub white_level: f32,
    /// Gamma value (1.0 - 3.0).
    pub gamma: f32,
}

impl Default for ToneMappingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 1.0,
            white_level: 1.0,
            gamma: 2.2,
        }
    }
}
```

**Step 2: Add build_tone_mapping_section function**

```rust
/// Builds the tone mapping settings section.
pub fn build_tone_mapping_section(ui: &mut Ui, settings: &mut ToneMappingSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Tone Mapping")
        .default_open(false)
        .show(ui, |ui| {
            if ui.checkbox(&mut settings.enabled, "Enable").changed() {
                changed = true;
            }

            if settings.enabled {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Exposure:");
                    if ui
                        .add(
                            Slider::new(&mut settings.exposure, 0.1..=4.0)
                                .logarithmic(true)
                                .clamp_to_range(true),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("White Level:");
                    if ui
                        .add(
                            Slider::new(&mut settings.white_level, 0.5..=4.0)
                                .logarithmic(true)
                                .clamp_to_range(true),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Gamma:");
                    if ui
                        .add(
                            Slider::new(&mut settings.gamma, 1.0..=3.0)
                                .clamp_to_range(true),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.separator();
                if ui.button("Reset to Defaults").clicked() {
                    *settings = ToneMappingSettings::default();
                    changed = true;
                }
            }
        });

    changed
}
```

**Step 3: Export from lib.rs**

Add to exports in `crates/polyscope-ui/src/lib.rs`:

```rust
pub use panels::ToneMappingSettings;
```

**Step 4: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add tone mapping settings UI"
```

---

## Task 6: Integrate Tone Mapping into Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add tone mapping state to App**

Add to App struct:

```rust
    // Tone mapping settings
    tone_mapping_settings: polyscope_ui::ToneMappingSettings,
```

Initialize in App::new():

```rust
            tone_mapping_settings: polyscope_ui::ToneMappingSettings::default(),
```

**Step 2: Add tone mapping UI panel**

In the `render()` function inside `build_left_panel`, add after appearance section:

```rust
            // Tone mapping section
            polyscope_ui::panels::build_tone_mapping_section(ui, &mut self.tone_mapping_settings);
```

**Step 3: Apply tone mapping in render loop**

Modify the render loop to:
1. Render scene to HDR texture instead of directly to surface
2. Apply tone mapping pass to output final result

This requires modifying the main render pass target and adding a final blit.

**Step 4: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: integrate tone mapping into render loop"
```

---

## Task 7: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Visual verification**

Run: `cargo run --example basic_demo`
Expected:
- Tone Mapping panel appears in UI
- Adjusting exposure brightens/darkens scene
- Adjusting gamma affects contrast
- White level affects highlight compression

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: format code and finalize tone mapping"
```

---

## Summary

This plan adds tone mapping with:

1. **ToneMappingConfig** in polyscope-core
2. **tone_map.wgsl** shader with Reinhard operator
3. **ToneMapPass** for post-processing
4. **HDR intermediate texture** for proper tone mapping
5. **UI controls** for exposure, white level, gamma
6. **App integration** in render loop

The tone mapping uses the Reinhard extended operator with configurable white point for highlight compression.
