# Ground Plane Reflections Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add TileReflection ground plane mode that renders reflected scene geometry on the ground plane with configurable reflection intensity, matching original C++ Polyscope functionality.

**Architecture:** Implement planar reflections using stencil buffer. First render ground plane to stencil, then render scene geometry mirrored about the ground plane (only where stencil is set), finally render normal ground plane with reflection blended. Uses a reflection matrix to flip scene about ground plane.

**Tech Stack:** Rust, wgpu, WGSL shaders, glam

**Dependencies:** Requires existing ground plane and shadow mode implementations.

**Parallel Execution Note:** Do NOT run in parallel with shadow-mode plan (both modify ground plane shader). Can run in parallel with tone-mapping plan.

---

## Task 1: Add Reflection Configuration to Core

**Files:**
- Modify: `crates/polyscope-core/src/ground_plane.rs`

**Step 1: Add TileReflection mode**

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
    /// Tiled ground plane with reflections.
    TileReflection,
}
```

**Step 2: Add reflection settings to GroundPlaneConfig**

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
    /// Reflection intensity (0.0 = none, 1.0 = full mirror).
    pub reflection_intensity: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
            shadow_blur_iters: 2,
            shadow_darkness: 0.4,
            reflection_intensity: 0.25,
        }
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p polyscope-core`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-core/src/ground_plane.rs
git commit -m "feat(core): add TileReflection mode and reflection settings"
```

---

## Task 2: Create Reflection Matrix Utility

**Files:**
- Create: `crates/polyscope-render/src/reflection.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create reflection matrix computation**

Create `crates/polyscope-render/src/reflection.rs`:

```rust
//! Planar reflection utilities.

use glam::{Mat4, Vec3, Vec4};

/// Computes a reflection matrix for a plane.
///
/// The plane is defined by a point on the plane and its normal.
/// The resulting matrix reflects points across this plane.
pub fn reflection_matrix(plane_point: Vec3, plane_normal: Vec3) -> Mat4 {
    let n = plane_normal.normalize();
    let d = -plane_point.dot(n);

    // Reflection matrix formula:
    // | 1-2nx²   -2nxny   -2nxnz   -2nxd |
    // | -2nxny   1-2ny²   -2nynz   -2nyd |
    // | -2nxnz   -2nynz   1-2nz²   -2nzd |
    // |    0        0        0       1   |

    Mat4::from_cols(
        Vec4::new(1.0 - 2.0 * n.x * n.x, -2.0 * n.x * n.y, -2.0 * n.x * n.z, 0.0),
        Vec4::new(-2.0 * n.x * n.y, 1.0 - 2.0 * n.y * n.y, -2.0 * n.y * n.z, 0.0),
        Vec4::new(-2.0 * n.x * n.z, -2.0 * n.y * n.z, 1.0 - 2.0 * n.z * n.z, 0.0),
        Vec4::new(-2.0 * n.x * d, -2.0 * n.y * d, -2.0 * n.z * d, 1.0),
    )
}

/// Computes a reflection matrix for a horizontal ground plane at given height.
///
/// Assumes Y-up coordinate system.
pub fn ground_reflection_matrix(height: f32) -> Mat4 {
    reflection_matrix(Vec3::new(0.0, height, 0.0), Vec3::Y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_matrix_identity_at_origin() {
        let mat = reflection_matrix(Vec3::ZERO, Vec3::Y);

        // Point above plane should reflect below
        let point = Vec3::new(1.0, 2.0, 3.0);
        let reflected = mat.transform_point3(point);

        assert!((reflected.x - point.x).abs() < 0.001);
        assert!((reflected.y - (-point.y)).abs() < 0.001);
        assert!((reflected.z - point.z).abs() < 0.001);
    }

    #[test]
    fn test_ground_reflection_at_height() {
        let height = 1.0;
        let mat = ground_reflection_matrix(height);

        // Point at height 3 should reflect to height -1
        let point = Vec3::new(0.0, 3.0, 0.0);
        let reflected = mat.transform_point3(point);

        // Distance from plane is 2, so reflected should be 2 below plane
        assert!((reflected.y - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_reflection_is_involution() {
        let mat = reflection_matrix(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
        let double = mat * mat;

        // Reflecting twice should give identity
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((double.col(j)[i] - expected).abs() < 0.001);
            }
        }
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:

```rust
mod reflection;
pub use reflection::{reflection_matrix, ground_reflection_matrix};
```

**Step 3: Run tests**

Run: `cargo test -p polyscope-render`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/reflection.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add reflection matrix utilities"
```

---

## Task 3: Create Stencil Ground Plane Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/ground_stencil.wgsl`

**Step 1: Create stencil shader**

Create `crates/polyscope-render/src/shaders/ground_stencil.wgsl`:

```wgsl
// Ground plane stencil shader
// Writes to stencil buffer to mark ground plane pixels for reflection

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct GroundUniforms {
    center: vec4<f32>,
    basis_x: vec4<f32>,
    basis_y: vec4<f32>,
    basis_z: vec4<f32>,
    height: f32,
    length_scale: f32,
    camera_height: f32,
    up_sign: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

// Same vertex shader as ground plane (infinite plane geometry)
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let tri_idx = vertex_index / 3u;
    let vert_idx = vertex_index % 3u;

    let center = vec4<f32>(ground.basis_z.xyz * ground.height, 1.0);

    var corners: array<vec4<f32>, 4>;
    corners[0] = vec4<f32>( ground.basis_x.xyz + ground.basis_y.xyz, 0.0);
    corners[1] = vec4<f32>(-ground.basis_x.xyz + ground.basis_y.xyz, 0.0);
    corners[2] = vec4<f32>(-ground.basis_x.xyz - ground.basis_y.xyz, 0.0);
    corners[3] = vec4<f32>( ground.basis_x.xyz - ground.basis_y.xyz, 0.0);

    var world_pos: vec4<f32>;
    if (vert_idx == 0u) {
        world_pos = center;
    } else if (vert_idx == 1u) {
        world_pos = corners[(tri_idx + 1u) % 4u];
    } else {
        world_pos = corners[tri_idx];
    }

    let adjusted_pos = world_pos + vec4<f32>(ground.basis_z.xyz, 0.0) * ground.height * world_pos.w;
    out.position = camera.view_proj * adjusted_pos;

    return out;
}

// Fragment shader writes stencil only (no color output)
@fragment
fn fs_main(in: VertexOutput) {
    // Just mark stencil - actual value set in pipeline state
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/ground_stencil.wgsl
git commit -m "feat(render): add ground plane stencil shader"
```

---

## Task 4: Create Reflected Scene Shader Variant

**Files:**
- Create: `crates/polyscope-render/src/shaders/reflected_mesh.wgsl`

**Step 1: Create reflected mesh shader**

Create `crates/polyscope-render/src/shaders/reflected_mesh.wgsl`:

```wgsl
// Reflected mesh shader
// Renders mesh geometry with reflection matrix applied
// Also flips normals for correct lighting

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct MeshUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    edge_width: f32,
    edge_color: vec3<f32>,
    shade_style: u32,
    use_face_colors: u32,
    backface_policy: u32,
    _padding: vec2<f32>,
}

struct ReflectionUniforms {
    reflection_matrix: mat4x4<f32>,
    intensity: f32,
    ground_height: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> mesh_uniforms: MeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> barycentrics: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> colors: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read> edge_is_real: array<vec4<f32>>;
@group(1) @binding(0) var<uniform> reflection: ReflectionUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) edge_real: vec3<f32>,
    @location(4) world_pos: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let local_pos = positions[vertex_index].xyz;
    let world_pos = (mesh_uniforms.model * vec4<f32>(local_pos, 1.0)).xyz;

    // Apply reflection matrix
    let reflected_pos = (reflection.reflection_matrix * vec4<f32>(world_pos, 1.0)).xyz;

    out.position = camera.view_proj * vec4<f32>(reflected_pos, 1.0);
    out.world_pos = reflected_pos;

    // Flip normal for reflected geometry
    let local_normal = normals[vertex_index].xyz;
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);
    out.normal = -world_normal; // Flip for reflection

    out.color = colors[vertex_index].rgb;
    out.barycentric = barycentrics[vertex_index].xyz;
    out.edge_real = edge_is_real[vertex_index].xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clip pixels below ground plane (shouldn't be visible in reflection)
    if (in.world_pos.y < reflection.ground_height) {
        discard;
    }

    // Basic shading (simplified from main mesh shader)
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.normal, light_dir), 0.0) * 0.7;

    var color = in.color * (ambient + diffuse);

    // Apply reflection intensity (fade out reflection)
    color *= reflection.intensity;

    return vec4<f32>(color, reflection.intensity);
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/reflected_mesh.wgsl
git commit -m "feat(render): add reflected mesh shader"
```

---

## Task 5: Create Reflection Pass Structure

**Files:**
- Create: `crates/polyscope-render/src/reflection_pass.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create reflection pass structure**

Create `crates/polyscope-render/src/reflection_pass.rs`:

```rust
//! Planar reflection rendering pass.

use glam::Mat4;
use wgpu::util::DeviceExt;

/// GPU representation of reflection uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReflectionUniforms {
    pub reflection_matrix: [[f32; 4]; 4],
    pub intensity: f32,
    pub ground_height: f32,
    pub _padding: [f32; 2],
}

impl Default for ReflectionUniforms {
    fn default() -> Self {
        Self {
            reflection_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            intensity: 0.25,
            ground_height: 0.0,
            _padding: [0.0; 2],
        }
    }
}

/// Reflection pass render resources.
pub struct ReflectionPass {
    stencil_pipeline: wgpu::RenderPipeline,
    reflected_mesh_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl ReflectionPass {
    /// Creates a new reflection pass.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        mesh_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Create reflection uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Reflection Uniform Buffer"),
            contents: bytemuck::cast_slice(&[ReflectionUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout for reflection uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Reflection Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflection Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create stencil pipeline
        let stencil_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ground Stencil Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ground_stencil.wgsl").into()),
        });

        let stencil_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Stencil Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let stencil_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ground Stencil Pipeline"),
            layout: Some(&stencil_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &stencil_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &stencil_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(), // Don't write color
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        pass_op: wgpu::StencilOperation::Replace,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Always,
                        pass_op: wgpu::StencilOperation::Replace,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xFF,
                    write_mask: 0xFF,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create reflected mesh pipeline
        let reflected_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reflected Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/reflected_mesh.wgsl").into()),
        });

        let reflected_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Reflected Mesh Pipeline Layout"),
            bind_group_layouts: &[mesh_bind_group_layout, &bind_group_layout],
            push_constant_ranges: &[],
        });

        let reflected_mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Reflected Mesh Pipeline"),
            layout: Some(&reflected_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &reflected_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &reflected_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Front), // Flip culling for reflection
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal, // Only draw where stencil == 1
                        pass_op: wgpu::StencilOperation::Keep,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal,
                        pass_op: wgpu::StencilOperation::Keep,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xFF,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            stencil_pipeline,
            reflected_mesh_pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    /// Updates the reflection uniforms.
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        reflection_matrix: Mat4,
        intensity: f32,
        ground_height: f32,
    ) {
        let uniforms = ReflectionUniforms {
            reflection_matrix: reflection_matrix.to_cols_array_2d(),
            intensity,
            ground_height,
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the stencil pipeline.
    pub fn stencil_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.stencil_pipeline
    }

    /// Returns the reflected mesh pipeline.
    pub fn reflected_mesh_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.reflected_mesh_pipeline
    }

    /// Returns the reflection bind group.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-render/src/lib.rs`:

```rust
mod reflection_pass;
pub use reflection_pass::{ReflectionPass, ReflectionUniforms};
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/reflection_pass.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add ReflectionPass structure"
```

---

## Task 6: Add Reflection UI Controls

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Update ground plane UI with reflection mode**

```rust
/// Builds the ground plane settings section.
pub fn build_ground_plane_section(
    ui: &mut Ui,
    mode: &mut u32, // 0=None, 1=Tile, 2=ShadowOnly, 3=TileReflection
    height: &mut f32,
    height_is_relative: &mut bool,
    shadow_blur_iters: &mut u32,
    shadow_darkness: &mut f32,
    reflection_intensity: &mut f32,
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
                    3 => "Tile + Reflection",
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
                    if ui.selectable_value(mode, 3, "Tile + Reflection").changed() {
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

                // Shadow settings (for modes 2 and 3)
                if *mode >= 2 {
                    ui.separator();
                    ui.label("Shadow Settings:");

                    ui.horizontal(|ui| {
                        ui.label("Blur:");
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

                // Reflection settings (for mode 3)
                if *mode == 3 {
                    ui.separator();
                    ui.label("Reflection Settings:");

                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        if ui.add(Slider::new(reflection_intensity, 0.0..=1.0)).changed() {
                            changed = true;
                        }
                    });
                }
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
git commit -m "feat(ui): add TileReflection mode to ground plane UI"
```

---

## Task 7: Integrate Reflection Pass into Engine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add reflection pass to RenderEngine**

Add to RenderEngine struct:

```rust
    reflection_pass: Option<ReflectionPass>,
```

**Step 2: Initialize reflection pass**

```rust
    if self.reflection_pass.is_none() {
        self.reflection_pass = Some(ReflectionPass::new(
            &self.device,
            self.surface_config.format,
            wgpu::TextureFormat::Depth24PlusStencil8, // Need stencil support
            &self.camera_bind_group_layout,
            &self.mesh_bind_group_layout,
        ));
    }
```

**Step 3: Add reflection rendering in render loop**

For TileReflection mode:

```rust
    if ground_plane.mode == GroundPlaneMode::TileReflection {
        // 1. Clear depth/stencil
        // 2. Render ground plane to stencil (mark reflection area)
        // 3. Render reflected meshes (only where stencil is set)
        // 4. Clear depth, keep stencil
        // 5. Render normal scene
        // 6. Render ground plane with tile pattern blended over reflection
    }
```

**Step 4: Update depth texture to include stencil**

Change depth format from `Depth32Float` to `Depth24PlusStencil8`:

```rust
    format: wgpu::TextureFormat::Depth24PlusStencil8,
```

**Step 5: Run compilation check**

Run: `cargo check --workspace`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/engine.rs crates/polyscope/src/app.rs
git commit -m "feat: integrate reflection pass into render engine"
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
- Ground Plane mode dropdown includes "Tile + Reflection"
- When TileReflection is selected, scene reflects on ground
- Reflection intensity slider affects mirror-like appearance
- Shadow settings still work in this mode

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: finalize reflection mode implementation"
```

---

## Summary

This plan adds TileReflection ground plane mode with:

1. **TileReflection mode** in GroundPlaneMode enum
2. **Reflection matrix** computation utilities
3. **Stencil pass** to mark ground plane pixels
4. **Reflected mesh shader** with flipped geometry
5. **ReflectionPass** for orchestrating multi-pass rendering
6. **UI controls** for reflection intensity
7. **Engine integration** with stencil buffer support

The implementation uses:
- Stencil buffer to mask reflection region
- Reflection matrix to mirror geometry about ground plane
- Front-face culling (flipped from normal) for reflected meshes
- Alpha blending for reflection intensity control
- Combined with existing shadow support
