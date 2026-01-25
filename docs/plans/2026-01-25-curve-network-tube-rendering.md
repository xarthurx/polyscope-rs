# Curve Network Tube Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Render curve network edges as 3D cylinder impostors with proper depth and lighting, using compute shaders for geometry generation and ray-cylinder intersection in fragment shaders.

**Architecture:** A compute shader generates bounding box geometry (36 vertices per edge) that fully contains each cylinder. The fragment shader performs ray-cylinder intersection to find the exact hit point, computes the surface normal, writes correct depth, and applies lighting. This matches the approach used by the original C++ Polyscope.

**Tech Stack:** wgpu, WGSL shaders, bytemuck for GPU structs

---

## Task 1: Create Compute Shader for Bounding Box Generation

**Files:**
- Create: `crates/polyscope-render/src/shaders/curve_network_tube_compute.wgsl`

**Step 1: Create the compute shader file**

```wgsl
// Compute shader for generating cylinder bounding box geometry
// Each edge gets a bounding box (36 vertices = 12 triangles)

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

struct GeneratedVertex {
    position: vec4<f32>,
    edge_id_and_vertex_id: vec4<u32>,  // edge_id, vertex_id, padding, padding
}

@group(0) @binding(0) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read_write> output_vertices: array<GeneratedVertex>;
@group(0) @binding(3) var<uniform> num_edges: u32;

// Build orthonormal basis perpendicular to cylinder axis
fn build_basis(axis: vec3<f32>) -> mat3x3<f32> {
    // Choose a vector not parallel to axis
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(axis, up)) > 0.99) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }

    let perp1 = normalize(cross(axis, up));
    let perp2 = cross(axis, perp1);

    return mat3x3<f32>(perp1, perp2, axis);
}

// Box vertex offsets (8 corners)
// Indices: 0-3 at tail, 4-7 at tip
fn get_box_corner(corner_id: u32, basis: mat3x3<f32>, tail: vec3<f32>, tip: vec3<f32>, radius: f32) -> vec3<f32> {
    let r = radius * 1.1;  // Slight padding to ensure coverage

    // Determine which end (tail or tip) and which corner
    let at_tip = corner_id >= 4u;
    let local_id = corner_id % 4u;

    // Corner offsets in local space
    var offset: vec2<f32>;
    switch (local_id) {
        case 0u: { offset = vec2<f32>(-r, -r); }
        case 1u: { offset = vec2<f32>( r, -r); }
        case 2u: { offset = vec2<f32>( r,  r); }
        case 3u: { offset = vec2<f32>(-r,  r); }
        default: { offset = vec2<f32>(0.0, 0.0); }
    }

    let base_pos = select(tail, tip, at_tip);
    // Extend slightly beyond endpoints to cover caps
    let axis_extend = select(-0.1 * radius, 0.1 * radius, at_tip);
    let axis_dir = normalize(tip - tail);

    return base_pos + basis[0] * offset.x + basis[1] * offset.y + axis_dir * axis_extend;
}

// Triangle indices for a box (12 triangles = 36 vertices)
// Returns corner index for given triangle vertex
fn get_box_triangle_vertex(tri_id: u32, vert_id: u32) -> u32 {
    // 12 triangles, each with 3 vertices
    // Front face (at tail): 0,1,2, 0,2,3
    // Back face (at tip): 4,6,5, 4,7,6
    // Top face: 3,2,6, 3,6,7
    // Bottom face: 0,5,1, 0,4,5
    // Right face: 1,5,6, 1,6,2
    // Left face: 0,3,7, 0,7,4

    var indices = array<u32, 36>(
        // Front (tail end, facing -Z in local)
        0u, 2u, 1u,  0u, 3u, 2u,
        // Back (tip end, facing +Z in local)
        4u, 5u, 6u,  4u, 6u, 7u,
        // Top
        3u, 6u, 2u,  3u, 7u, 6u,
        // Bottom
        0u, 1u, 5u,  0u, 5u, 4u,
        // Right
        1u, 2u, 6u,  1u, 6u, 5u,
        // Left
        0u, 4u, 7u,  0u, 7u, 3u
    );

    return indices[tri_id * 3u + vert_id];
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let edge_id = global_id.x;

    if (edge_id >= num_edges) {
        return;
    }

    // Read edge endpoints
    let tail = edge_vertices[edge_id * 2u].xyz;
    let tip = edge_vertices[edge_id * 2u + 1u].xyz;

    // Handle degenerate edges
    let edge_length = length(tip - tail);
    if (edge_length < 0.0001) {
        // Write degenerate triangles (all at same point)
        for (var i = 0u; i < 36u; i++) {
            let out_idx = edge_id * 36u + i;
            output_vertices[out_idx].position = vec4<f32>(tail, 1.0);
            output_vertices[out_idx].edge_id_and_vertex_id = vec4<u32>(edge_id, i, 0u, 0u);
        }
        return;
    }

    // Build orthonormal basis
    let axis = normalize(tip - tail);
    let basis = build_basis(axis);

    // Generate 36 vertices (12 triangles)
    for (var tri = 0u; tri < 12u; tri++) {
        for (var v = 0u; v < 3u; v++) {
            let corner_id = get_box_triangle_vertex(tri, v);
            let position = get_box_corner(corner_id, basis, tail, tip, uniforms.radius);

            let out_idx = edge_id * 36u + tri * 3u + v;
            output_vertices[out_idx].position = vec4<f32>(position, 1.0);
            output_vertices[out_idx].edge_id_and_vertex_id = vec4<u32>(edge_id, tri * 3u + v, 0u, 0u);
        }
    }
}
```

**Step 2: Verify shader compiles**

Run: `cargo build 2>&1 | head -50`
Expected: Build succeeds (shader not yet included in build)

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/curve_network_tube_compute.wgsl
git commit -m "feat(render): add compute shader for cylinder bounding box generation"
```

---

## Task 2: Create Tube Render Shader with Ray-Cylinder Intersection

**Files:**
- Create: `crates/polyscope-render/src/shaders/curve_network_tube.wgsl`

**Step 1: Create the tube render shader**

```wgsl
// Curve network tube shader - renders cylinders via ray casting
// Vertex shader passes through bounding box geometry
// Fragment shader performs ray-cylinder intersection

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> edge_colors: array<vec4<f32>>;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) edge_id_and_vertex_id: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) @interpolate(flat) edge_id: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.world_position = in.position.xyz;
    out.clip_position = camera.view_proj * in.position;
    out.edge_id = in.edge_id_and_vertex_id.x;

    return out;
}

// Ray-cylinder intersection
// Returns true if ray hits cylinder, outputs t_hit, hit_point, hit_normal
fn ray_cylinder_intersect(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    cyl_start: vec3<f32>,
    cyl_end: vec3<f32>,
    cyl_radius: f32,
    t_hit: ptr<function, f32>,
    hit_point: ptr<function, vec3<f32>>,
    hit_normal: ptr<function, vec3<f32>>
) -> bool {
    let cyl_axis = cyl_end - cyl_start;
    let cyl_length = length(cyl_axis);
    let cyl_dir = cyl_axis / cyl_length;

    // Vector from cylinder start to ray origin
    let delta = ray_origin - cyl_start;

    // Project ray direction and delta onto plane perpendicular to cylinder
    let ray_dir_perp = ray_dir - dot(ray_dir, cyl_dir) * cyl_dir;
    let delta_perp = delta - dot(delta, cyl_dir) * cyl_dir;

    // Quadratic coefficients for intersection with infinite cylinder
    let a = dot(ray_dir_perp, ray_dir_perp);
    let b = 2.0 * dot(ray_dir_perp, delta_perp);
    let c = dot(delta_perp, delta_perp) - cyl_radius * cyl_radius;

    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return false;
    }

    let sqrt_disc = sqrt(discriminant);
    var t = (-b - sqrt_disc) / (2.0 * a);

    // If t is negative, try the other intersection
    if (t < 0.001) {
        t = (-b + sqrt_disc) / (2.0 * a);
        if (t < 0.001) {
            return false;
        }
    }

    // Check if intersection is within cylinder bounds
    let p = ray_origin + t * ray_dir;
    let proj = dot(p - cyl_start, cyl_dir);

    if (proj < 0.0 || proj > cyl_length) {
        // Try cap intersection (simplified - just check if inside caps)
        // For now, just reject points outside the cylinder body
        return false;
    }

    // Compute normal (pointing outward from axis)
    let closest_on_axis = cyl_start + proj * cyl_dir;
    let normal = normalize(p - closest_on_axis);

    *t_hit = t;
    *hit_point = p;
    *hit_normal = normal;

    return true;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    // Get cylinder data
    let tail = edge_vertices[in.edge_id * 2u].xyz;
    let tip = edge_vertices[in.edge_id * 2u + 1u].xyz;
    let radius = uniforms.radius;

    // Setup ray from camera through this fragment
    let ray_origin = camera.camera_pos.xyz;
    let ray_dir = normalize(in.world_position - ray_origin);

    // Ray-cylinder intersection
    var t_hit: f32;
    var hit_point: vec3<f32>;
    var hit_normal: vec3<f32>;

    if (!ray_cylinder_intersect(ray_origin, ray_dir, tail, tip, radius,
                                 &t_hit, &hit_point, &hit_normal)) {
        discard;
    }

    // Compute depth
    let clip_pos = camera.view_proj * vec4<f32>(hit_point, 1.0);
    out.depth = clip_pos.z / clip_pos.w;

    // Get color
    let ec = edge_colors[in.edge_id];
    var base_color: vec3<f32>;
    if (ec.r + ec.g + ec.b > 0.001) {
        base_color = ec.rgb;
    } else {
        base_color = uniforms.color.rgb;
    }

    // Simple lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let view_dir = -ray_dir;

    // Ambient
    let ambient = 0.3;

    // Diffuse
    let n_dot_l = max(dot(hit_normal, light_dir), 0.0);
    let diffuse = 0.6 * n_dot_l;

    // Specular
    let half_vec = normalize(light_dir + view_dir);
    let n_dot_h = max(dot(hit_normal, half_vec), 0.0);
    let specular = 0.3 * pow(n_dot_h, 32.0);

    let lit_color = base_color * (ambient + diffuse) + vec3<f32>(1.0) * specular;

    out.color = vec4<f32>(lit_color, 1.0);

    return out;
}
```

**Step 2: Verify file created**

Run: `ls -la crates/polyscope-render/src/shaders/curve_network_tube.wgsl`
Expected: File exists

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/curve_network_tube.wgsl
git commit -m "feat(render): add tube render shader with ray-cylinder intersection"
```

---

## Task 3: Add Tube Pipeline and Compute Pipeline to Engine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add new pipeline fields to RenderEngine struct**

Find the struct definition and add after `curve_network_edge_pipeline`:

```rust
// Add these fields to RenderEngine struct:
pub curve_network_tube_pipeline: Option<wgpu::RenderPipeline>,
pub curve_network_tube_compute_pipeline: Option<wgpu::ComputePipeline>,
curve_network_tube_bind_group_layout: Option<wgpu::BindGroupLayout>,
curve_network_tube_compute_bind_group_layout: Option<wgpu::BindGroupLayout>,
```

**Step 2: Initialize fields in new() and new_headless()**

Add to both constructors:
```rust
curve_network_tube_pipeline: None,
curve_network_tube_compute_pipeline: None,
curve_network_tube_bind_group_layout: None,
curve_network_tube_compute_bind_group_layout: None,
```

**Step 3: Create method to build compute pipeline**

Add after `create_curve_network_edge_pipeline`:

```rust
fn create_curve_network_tube_pipelines(&mut self) {
    // Compute shader
    let compute_shader_source = include_str!("shaders/curve_network_tube_compute.wgsl");
    let compute_shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Curve Network Tube Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(compute_shader_source.into()),
    });

    // Compute bind group layout
    let compute_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Curve Network Tube Compute Bind Group Layout"),
        entries: &[
            // Edge vertices (input)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Output vertices
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Num edges
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let compute_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Curve Network Tube Compute Pipeline Layout"),
        bind_group_layouts: &[&compute_bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Curve Network Tube Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Render shader
    let render_shader_source = include_str!("shaders/curve_network_tube.wgsl");
    let render_shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Curve Network Tube Render Shader"),
        source: wgpu::ShaderSource::Wgsl(render_shader_source.into()),
    });

    // Render bind group layout
    let render_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Curve Network Tube Render Bind Group Layout"),
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
            // Curve network uniforms
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
            // Edge vertices (for raycast)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Edge colors
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let render_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Curve Network Tube Render Pipeline Layout"),
        bind_group_layouts: &[&render_bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Curve Network Tube Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &render_shader,
            entry_point: Some("vs_main"),
            buffers: &[
                // Generated vertex buffer layout
                wgpu::VertexBufferLayout {
                    array_stride: 32, // vec4<f32> position + vec4<u32> edge_id_and_vertex_id
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32x4,
                            offset: 16,
                            shader_location: 1,
                        },
                    ],
                },
            ],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Don't cull - we need to see box from inside too
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    self.curve_network_tube_pipeline = Some(render_pipeline);
    self.curve_network_tube_compute_pipeline = Some(compute_pipeline);
    self.curve_network_tube_bind_group_layout = Some(render_bind_group_layout);
    self.curve_network_tube_compute_bind_group_layout = Some(compute_bind_group_layout);
}
```

**Step 4: Call the method in constructors**

Add after `engine.create_curve_network_edge_pipeline();`:
```rust
engine.create_curve_network_tube_pipelines();
```

**Step 5: Add accessor methods**

```rust
pub fn curve_network_tube_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    self.curve_network_tube_bind_group_layout
        .as_ref()
        .expect("Tube bind group layout not initialized")
}

pub fn curve_network_tube_compute_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    self.curve_network_tube_compute_bind_group_layout
        .as_ref()
        .expect("Tube compute bind group layout not initialized")
}

pub fn curve_network_tube_compute_pipeline(&self) -> &wgpu::ComputePipeline {
    self.curve_network_tube_compute_pipeline
        .as_ref()
        .expect("Tube compute pipeline not initialized")
}
```

**Step 6: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add tube compute and render pipelines to engine"
```

---

## Task 4: Extend CurveNetworkRenderData for Tube Rendering

**Files:**
- Modify: `crates/polyscope-render/src/curve_network_render.rs`

**Step 1: Add tube-specific fields to CurveNetworkRenderData**

```rust
/// GPU resources for rendering a curve network.
pub struct CurveNetworkRenderData {
    // ... existing fields ...

    // Tube rendering resources
    pub generated_vertex_buffer: Option<wgpu::Buffer>,
    pub num_edges_buffer: Option<wgpu::Buffer>,
    pub compute_bind_group: Option<wgpu::BindGroup>,
    pub tube_render_bind_group: Option<wgpu::BindGroup>,
}
```

**Step 2: Initialize new fields in constructor**

Add to the `Self { ... }` return in `new()`:
```rust
generated_vertex_buffer: None,
num_edges_buffer: None,
compute_bind_group: None,
tube_render_bind_group: None,
```

**Step 3: Add method to initialize tube resources**

```rust
/// Initializes tube rendering resources.
pub fn init_tube_resources(
    &mut self,
    device: &wgpu::Device,
    compute_bind_group_layout: &wgpu::BindGroupLayout,
    render_bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
) {
    // Create generated vertex buffer (36 vertices per edge)
    let vertex_buffer_size = (self.num_edges as usize * 36 * 32) as u64; // 32 bytes per vertex
    let generated_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Curve Network Generated Vertices"),
        size: vertex_buffer_size.max(32), // Minimum size
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });

    // Create num_edges uniform buffer
    let num_edges_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Curve Network Num Edges"),
        contents: bytemuck::cast_slice(&[self.num_edges]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // Create compute bind group
    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Curve Network Tube Compute Bind Group"),
        layout: compute_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.edge_vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: generated_vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: num_edges_buffer.as_entire_binding(),
            },
        ],
    });

    // Create render bind group
    let tube_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Curve Network Tube Render Bind Group"),
        layout: render_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: self.edge_vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: self.edge_color_buffer.as_entire_binding(),
            },
        ],
    });

    self.generated_vertex_buffer = Some(generated_vertex_buffer);
    self.num_edges_buffer = Some(num_edges_buffer);
    self.compute_bind_group = Some(compute_bind_group);
    self.tube_render_bind_group = Some(tube_render_bind_group);
}

/// Returns whether tube resources are initialized.
pub fn has_tube_resources(&self) -> bool {
    self.generated_vertex_buffer.is_some()
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/curve_network_render.rs
git commit -m "feat(render): add tube rendering resources to CurveNetworkRenderData"
```

---

## Task 5: Update CurveNetwork Structure for Render Mode

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network/mod.rs`

**Step 1: Add render_mode field**

Add to the CurveNetwork struct:
```rust
/// Render mode: 0 = line, 1 = tube
render_mode: u32,
```

**Step 2: Initialize in constructor**

Add to `Self { ... }` in `new()`:
```rust
render_mode: 0, // Default to line rendering
```

**Step 3: Add getter/setter methods**

```rust
/// Gets the render mode (0 = line, 1 = tube).
pub fn render_mode(&self) -> u32 {
    self.render_mode
}

/// Sets the render mode (0 = line, 1 = tube).
pub fn set_render_mode(&mut self, mode: u32) -> &mut Self {
    self.render_mode = mode.min(1); // Clamp to valid values
    self
}
```

**Step 4: Update update_gpu_buffers to include render_mode**

Modify the uniforms creation in `update_gpu_buffers`:
```rust
let uniforms = CurveNetworkUniforms {
    color: [self.color.x, self.color.y, self.color.z, 1.0],
    radius: self.radius,
    radius_is_relative: if self.radius_is_relative { 1 } else { 0 },
    render_mode: self.render_mode,
    _padding: 0.0,
};
```

**Step 5: Add method to initialize tube resources**

```rust
/// Initializes tube rendering resources.
pub fn init_tube_resources(
    &mut self,
    device: &wgpu::Device,
    compute_bind_group_layout: &wgpu::BindGroupLayout,
    render_bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
) {
    if let Some(render_data) = &mut self.render_data {
        render_data.init_tube_resources(
            device,
            compute_bind_group_layout,
            render_bind_group_layout,
            camera_buffer,
        );
    }
}
```

**Step 6: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/curve_network/mod.rs
git commit -m "feat(structures): add render_mode to CurveNetwork"
```

---

## Task 6: Update UI for Render Mode Selection

**Files:**
- Modify: `crates/polyscope-ui/src/structure_ui.rs`

**Step 1: Add render_mode parameter to build_curve_network_ui**

Update function signature:
```rust
pub fn build_curve_network_ui(
    ui: &mut Ui,
    num_nodes: usize,
    num_edges: usize,
    radius: &mut f32,
    radius_is_relative: &mut bool,
    color: &mut [f32; 3],
    render_mode: &mut u32,
) -> bool {
```

**Step 2: Add render mode dropdown**

Add after the color section:
```rust
// Render mode
ui.separator();
egui::ComboBox::from_label("Render")
    .selected_text(match *render_mode {
        0 => "Lines",
        _ => "Tubes",
    })
    .show_ui(ui, |ui| {
        if ui.selectable_value(render_mode, 0, "Lines").changed() {
            changed = true;
        }
        if ui.selectable_value(render_mode, 1, "Tubes").changed() {
            changed = true;
        }
    });
```

**Step 3: Build and verify**

Run: `cargo build 2>&1`
Expected: Build fails (caller not updated yet - expected)

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/structure_ui.rs
git commit -m "feat(ui): add render mode dropdown for curve networks"
```

---

## Task 7: Update CurveNetwork UI Integration

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network/mod.rs`

**Step 1: Update build_egui_ui to pass render_mode**

```rust
pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
    let mut color = [self.color.x, self.color.y, self.color.z];
    let mut radius = self.radius;
    let mut radius_is_relative = self.radius_is_relative;
    let mut render_mode = self.render_mode;

    if polyscope_ui::build_curve_network_ui(
        ui,
        self.node_positions.len(),
        self.edge_tail_inds.len(),
        &mut radius,
        &mut radius_is_relative,
        &mut color,
        &mut render_mode,
    ) {
        self.color = Vec3::new(color[0], color[1], color[2]);
        self.radius = radius;
        self.radius_is_relative = radius_is_relative;
        self.render_mode = render_mode;
    }
    // ... rest of function unchanged ...
}
```

**Step 2: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/curve_network/mod.rs
git commit -m "feat(structures): integrate render mode UI for curve networks"
```

---

## Task 8: Integrate Tube Rendering in App Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Initialize tube resources when needed**

In the GPU initialization section (around line 310), after `cn.init_gpu_resources()`:
```rust
if structure.type_name() == "CurveNetwork" {
    if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
        if cn.render_data().is_none() {
            cn.init_gpu_resources(
                &engine.device,
                engine.curve_network_edge_bind_group_layout(),
                engine.camera_buffer(),
            );
        }
        // Initialize tube resources if not already done
        if let Some(rd) = cn.render_data() {
            if !rd.has_tube_resources() {
                cn.init_tube_resources(
                    &engine.device,
                    engine.curve_network_tube_compute_bind_group_layout(),
                    engine.curve_network_tube_bind_group_layout(),
                    engine.camera_buffer(),
                );
            }
        }
    }
}
```

**Step 2: Add compute pass before render pass**

Before the main render pass, add a compute pass for tube geometry generation:
```rust
// Compute pass for curve network tubes
{
    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Curve Network Tube Compute Pass"),
        timestamp_writes: None,
    });

    compute_pass.set_pipeline(engine.curve_network_tube_compute_pipeline());

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !structure.is_enabled() {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    if cn.render_mode() == 1 {  // Tube mode
                        if let Some(render_data) = cn.render_data() {
                            if let Some(compute_bg) = &render_data.compute_bind_group {
                                compute_pass.set_bind_group(0, compute_bg, &[]);
                                let num_workgroups = (render_data.num_edges + 63) / 64;
                                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                            }
                        }
                    }
                }
            }
        }
    });
}
```

**Step 3: Update render pass to use tube pipeline when appropriate**

Replace the existing curve network rendering section with:
```rust
// Draw curve network edges (lines) and camera views
if let Some(line_pipeline) = &engine.curve_network_edge_pipeline {
    render_pass.set_pipeline(line_pipeline);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !structure.is_enabled() {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    if cn.render_mode() == 0 {  // Line mode
                        if let Some(render_data) = cn.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..render_data.num_edges * 2, 0..1);
                        }
                    }
                }
            }
            // ... camera view rendering unchanged ...
        }
    });
}

// Draw curve network tubes
if let Some(tube_pipeline) = &engine.curve_network_tube_pipeline {
    render_pass.set_pipeline(tube_pipeline);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !structure.is_enabled() {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    if cn.render_mode() == 1 {  // Tube mode
                        if let Some(render_data) = cn.render_data() {
                            if let (Some(tube_bg), Some(gen_buf)) =
                                (&render_data.tube_render_bind_group, &render_data.generated_vertex_buffer)
                            {
                                render_pass.set_bind_group(0, tube_bg, &[]);
                                render_pass.set_vertex_buffer(0, gen_buf.slice(..));
                                render_pass.draw(0..render_data.num_edges * 36, 0..1);
                            }
                        }
                    }
                }
            }
        }
    });
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: Build succeeds

**Step 5: Run curve network demo and test**

Run: `cargo run --example curve_network_demo`
Expected: Demo runs, can switch between Lines and Tubes in UI

**Step 6: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat(app): integrate tube rendering pipeline for curve networks"
```

---

## Task 9: Test and Debug

**Step 1: Run demo and verify line mode works**

Run: `cargo run --example curve_network_demo`
Expected: Lines render as before (thin 1-pixel lines)

**Step 2: Switch to tube mode and verify**

In the demo, select a curve network and change "Render" dropdown to "Tubes".
Expected: Curves render as 3D cylinders with proper depth and lighting

**Step 3: Test radius adjustment**

Adjust radius slider while in tube mode.
Expected: Cylinder thickness changes accordingly

**Step 4: Test color**

Change color while in tube mode.
Expected: Cylinder color updates

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete curve network tube rendering implementation"
```

---

## Summary

This implementation adds cylinder impostor rendering to curve networks:

1. **Compute shader** generates bounding box geometry (36 vertices per edge)
2. **Fragment shader** performs ray-cylinder intersection for pixel-perfect cylinders
3. **UI dropdown** allows switching between line and tube render modes
4. **Backward compatible** - default remains line rendering

The approach matches the original C++ Polyscope implementation, adapted for WebGPU's lack of geometry shaders by using compute shaders instead.
