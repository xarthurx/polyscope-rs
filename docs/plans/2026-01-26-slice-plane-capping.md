# Slice Plane Capping Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement slice planes that cut through geometry and optionally show cross-section "capping" for volume meshes.

**Architecture:** Three-phase approach: (1) Basic fragment-level slicing via shader discard, (2) Slice plane visualization, (3) Volume mesh cross-section capping using geometry generation. The existing `SlicePlane` data structure in `polyscope-core` provides the foundation.

**Tech Stack:** wgpu, WGSL shaders, glam math library, egui for UI

---

## Background: C++ Polyscope Implementation

The C++ Polyscope uses two mechanisms:
1. **Fragment-level slicing**: Shader rule injects `if(dot(cullPos, normal) < dot(center, normal)) { discard; }` into fragment shaders
2. **Volume mesh inspection (SLICE_TETS)**: Geometry shader computes edge-plane intersections for each tetrahedron and emits the cross-section polygon

Since WGSL/wgpu doesn't support geometry shaders, we'll implement capping via CPU-side geometry generation that updates when the slice plane moves.

---

## Phase 1: Fragment-Level Slicing (Core)

This phase adds slice plane support to all existing shaders, discarding fragments on the negative side of the plane.

### Task 1.1: Add Slice Plane Uniforms to Camera Bind Group

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add SlicePlaneUniforms array to RenderEngine**

Add imports and a new buffer for slice planes:

```rust
// In imports
use polyscope_core::slice_plane::{SlicePlane, SlicePlaneUniforms, MAX_SLICE_PLANES};

// In RenderEngine struct
pub slice_plane_buffer: wgpu::Buffer,
pub slice_planes: Vec<SlicePlane>,
```

**Step 2: Create slice plane buffer in RenderEngine::new()**

```rust
// After camera_buffer creation
let slice_planes_data = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
let slice_plane_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Slice Plane Buffer"),
    contents: bytemuck::cast_slice(&slice_planes_data),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
});
```

**Step 3: Run build to verify compilation**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add slice plane uniform buffer to RenderEngine"
```

---

### Task 1.2: Update Surface Mesh Shader for Slice Planes

**Files:**
- Modify: `crates/polyscope-render/src/shaders/surface_mesh.wgsl`

**Step 1: Add slice plane uniform struct to shader**

Add after CameraUniforms:

```wgsl
struct SlicePlaneUniforms {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
}

struct SlicePlanesArray {
    planes: array<SlicePlaneUniforms, 4>,
}

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;
```

**Step 2: Add slice plane discard logic to fragment shader**

Add at the start of fs_main, after backface culling:

```wgsl
// Slice plane culling
for (var i = 0u; i < 4u; i = i + 1u) {
    let plane = slice_planes.planes[i];
    if (plane.enabled > 0.5) {
        let dist = dot(in.world_position - plane.origin, plane.normal);
        if (dist < 0.0) {
            discard;
        }
    }
}
```

**Step 3: Run build to verify shader compiles**

Run: `cargo build -p polyscope-render`
Expected: PASS (shader validation)

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shaders/surface_mesh.wgsl
git commit -m "feat(shaders): add slice plane discard to surface mesh shader"
```

---

### Task 1.3: Update Surface Mesh Render Pipeline for Slice Planes

**Files:**
- Modify: `crates/polyscope-render/src/surface_mesh_render.rs`

**Step 1: Create slice plane bind group layout**

In `create_mesh_pipeline()` or a new helper, add:

```rust
let slice_plane_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("Slice Plane Bind Group Layout"),
    entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }],
});
```

**Step 2: Update pipeline layout to include slice planes bind group**

Update the pipeline layout to include group(1):

```rust
let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Surface Mesh Pipeline Layout"),
    bind_group_layouts: &[&mesh_bind_group_layout, &slice_plane_bind_group_layout],
    push_constant_ranges: &[],
});
```

**Step 3: Store slice plane bind group in SurfaceMeshRenderData**

Add field and create bind group in `SurfaceMeshRenderData::new()`.

**Step 4: Bind slice planes in render pass**

In the draw call, set bind group at index 1.

**Step 5: Run tests**

Run: `cargo test -p polyscope-render`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/polyscope-render/src/surface_mesh_render.rs
git commit -m "feat(render): integrate slice plane uniforms into surface mesh pipeline"
```

---

### Task 1.4: Update Point Cloud Shader for Slice Planes

**Files:**
- Modify: `crates/polyscope-render/src/shaders/point_cloud.wgsl`
- Modify: `crates/polyscope-render/src/point_cloud_render.rs`

**Step 1: Add slice plane uniforms and discard to point cloud shader**

Same pattern as surface mesh shader.

**Step 2: Update point cloud pipeline**

Add slice plane bind group to point cloud render pipeline.

**Step 3: Run tests**

Run: `cargo test -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shaders/point_cloud.wgsl
git add crates/polyscope-render/src/point_cloud_render.rs
git commit -m "feat(render): add slice plane support to point cloud shader"
```

---

### Task 1.5: Update Curve Network Shader for Slice Planes

**Files:**
- Modify: `crates/polyscope-render/src/shaders/curve_network*.wgsl`
- Modify: `crates/polyscope-render/src/curve_network_render.rs`

**Step 1: Add slice plane uniforms and discard to curve network shaders**

Apply to both edge and tube shaders.

**Step 2: Update curve network pipeline**

Add slice plane bind group.

**Step 3: Run tests**

Run: `cargo test -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shaders/curve_network*.wgsl
git add crates/polyscope-render/src/curve_network_render.rs
git commit -m "feat(render): add slice plane support to curve network shader"
```

---

### Task 1.6: Update Vector Shader for Slice Planes

**Files:**
- Modify: `crates/polyscope-render/src/shaders/vector.wgsl`
- Modify: `crates/polyscope-render/src/vector_render.rs`

**Step 1: Add slice plane uniforms and discard to vector shader**

**Step 2: Update vector pipeline**

**Step 3: Run tests**

Run: `cargo test -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/shaders/vector.wgsl
git add crates/polyscope-render/src/vector_render.rs
git commit -m "feat(render): add slice plane support to vector shader"
```

---

### Task 1.7: Add Slice Plane API to Main polyscope Crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add slice plane management functions**

```rust
/// Adds a new slice plane.
pub fn add_slice_plane(name: &str) -> SlicePlane {
    with_context_mut(|ctx| {
        let plane = SlicePlane::new(name);
        ctx.slice_planes.push(plane.clone());
        plane
    })
}

/// Gets a mutable reference to a slice plane by name.
pub fn get_slice_plane_mut(name: &str) -> Option<&mut SlicePlane> {
    with_context_mut(|ctx| {
        ctx.slice_planes.iter_mut().find(|p| p.name() == name)
    })
}

/// Removes a slice plane by name.
pub fn remove_slice_plane(name: &str) {
    with_context_mut(|ctx| {
        ctx.slice_planes.retain(|p| p.name() != name);
    })
}
```

**Step 2: Update render loop to pass slice plane uniforms**

In the render function, update the slice plane buffer before rendering:

```rust
let uniforms: Vec<SlicePlaneUniforms> = ctx.slice_planes
    .iter()
    .take(MAX_SLICE_PLANES)
    .map(SlicePlaneUniforms::from)
    .collect();
// Pad to MAX_SLICE_PLANES
let mut padded = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
for (i, u) in uniforms.iter().enumerate() {
    padded[i] = *u;
}
queue.write_buffer(&engine.slice_plane_buffer, 0, bytemuck::cast_slice(&padded));
```

**Step 3: Run integration tests**

Run: `cargo test -p polyscope`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope/src/lib.rs
git add crates/polyscope/src/app.rs
git commit -m "feat: add slice plane API to main polyscope crate"
```

---

## Phase 2: Slice Plane Visualization

This phase renders the slice plane itself as a semi-transparent grid.

### Task 2.1: Create Slice Plane Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/slice_plane.wgsl`

**Step 1: Write slice plane visualization shader**

```wgsl
// Slice plane visualization shader
// Renders an infinite plane with a grid pattern

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PlaneUniforms {
    transform: mat4x4<f32>,  // Plane's object transform
    color: vec4<f32>,
    grid_color: vec4<f32>,
    transparency: f32,
    length_scale: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> plane: PlaneUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) plane_uv: vec2<f32>,
}

// Use points at infinity technique like C++ polyscope
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Plane geometry using homogeneous coordinates
    // 4 triangles forming a quad with vertices at infinity
    var positions = array<vec4<f32>, 12>(
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 1.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 0.0, -1.0, 0.0), vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, -1.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, -1.0, 0.0, 0.0),
    );

    let pos = positions[vertex_index];
    let world_pos = plane.transform * pos;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz / world_pos.w;
    out.plane_uv = vec2<f32>(dot(world_pos.xyz, (plane.transform * vec4<f32>(0.0, 1.0, 0.0, 0.0)).xyz),
                             dot(world_pos.xyz, (plane.transform * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Grid pattern
    let grid_size = plane.length_scale * 0.1;
    let uv = in.plane_uv / grid_size;
    let grid = abs(fract(uv - 0.5) - 0.5) / fwidth(uv);
    let line = min(grid.x, grid.y);
    let grid_factor = 1.0 - min(line, 1.0);

    let color = mix(plane.color.rgb, plane.grid_color.rgb, grid_factor * 0.5);
    return vec4<f32>(color, plane.transparency);
}
```

**Step 2: Run build to verify shader compiles**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/slice_plane.wgsl
git commit -m "feat(shaders): add slice plane visualization shader"
```

---

### Task 2.2: Create Slice Plane Render Module

**Files:**
- Create: `crates/polyscope-render/src/slice_plane_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Implement SlicePlaneRenderData struct**

```rust
use polyscope_core::slice_plane::SlicePlane;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlaneRenderUniforms {
    pub transform: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub grid_color: [f32; 4],
    pub transparency: f32,
    pub length_scale: f32,
    pub _padding: [f32; 2],
}

pub struct SlicePlaneRenderData {
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl SlicePlaneRenderData {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        plane: &SlicePlane,
        length_scale: f32,
    ) -> Self {
        // Create uniform buffer and bind group
        // ...
    }

    pub fn update(&self, queue: &wgpu::Queue, plane: &SlicePlane, length_scale: f32) {
        // Update uniforms
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        // Draw 12 vertices (4 triangles)
    }
}
```

**Step 2: Add pipeline creation function**

Create the render pipeline for slice plane visualization.

**Step 3: Add to lib.rs exports**

**Step 4: Run build**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/slice_plane_render.rs
git add crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add slice plane visualization rendering"
```

---

### Task 2.3: Integrate Slice Plane Rendering into Main Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add slice plane rendering after structures**

In the main render function, after rendering all structures:

```rust
// Render slice plane visualizations
for plane in &ctx.slice_planes {
    if plane.is_enabled() && plane.draw_plane() {
        // Render the plane with transparency
        slice_plane_render_data.draw(&mut render_pass);
    }
}
```

**Step 2: Enable blending for transparency**

Ensure the render pass has proper blend state.

**Step 3: Run example to verify**

Run: `cargo run --example basic_demo`
Expected: Slice planes visible when enabled

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: integrate slice plane visualization into render loop"
```

---

## Phase 3: UI Controls

### Task 3.1: Add Slice Plane Panel to UI

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Add slice plane controls**

```rust
pub fn build_slice_planes_panel(ui: &mut egui::Ui, ctx: &mut Context) {
    egui::CollapsingHeader::new("Slice Planes")
        .default_open(false)
        .show(ui, |ui| {
            if ui.button("Add Plane").clicked() {
                let name = format!("Slice Plane {}", ctx.slice_planes.len());
                ctx.slice_planes.push(SlicePlane::new(&name));
            }

            // List existing planes with controls
            for plane in &mut ctx.slice_planes {
                ui.horizontal(|ui| {
                    let mut enabled = plane.is_enabled();
                    if ui.checkbox(&mut enabled, plane.name()).changed() {
                        plane.set_enabled(enabled);
                    }

                    // Color picker
                    let mut color = plane.color().to_array();
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        plane.set_color(glam::Vec3::from(color));
                    }
                });

                // Origin/normal controls (collapsible)
                egui::CollapsingHeader::new("Transform")
                    .id_salt(plane.name())
                    .show(ui, |ui| {
                        let mut origin = plane.origin().to_array();
                        ui.horizontal(|ui| {
                            ui.label("Origin:");
                            ui.add(egui::DragValue::new(&mut origin[0]).speed(0.1));
                            ui.add(egui::DragValue::new(&mut origin[1]).speed(0.1));
                            ui.add(egui::DragValue::new(&mut origin[2]).speed(0.1));
                        });
                        plane.set_origin(glam::Vec3::from(origin));

                        let mut normal = plane.normal().to_array();
                        ui.horizontal(|ui| {
                            ui.label("Normal:");
                            ui.add(egui::DragValue::new(&mut normal[0]).speed(0.01));
                            ui.add(egui::DragValue::new(&mut normal[1]).speed(0.01));
                            ui.add(egui::DragValue::new(&mut normal[2]).speed(0.01));
                        });
                        plane.set_normal(glam::Vec3::from(normal));
                    });
            }
        });
}
```

**Step 2: Call from main UI**

Add call to `build_slice_planes_panel()` in the options panel area.

**Step 3: Run and test UI**

Run: `cargo run --example basic_demo`
Expected: Slice plane controls appear in UI

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add slice plane control panel"
```

---

## Phase 4: Volume Mesh Cross-Section Capping

This phase generates actual geometry for the cross-section when slicing volume meshes.

### Task 4.1: Implement Tet-Plane Intersection Algorithm

**Files:**
- Create: `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs`

**Step 1: Write the tet slicing algorithm**

```rust
//! Geometry generation for slicing tetrahedra with a plane.

use glam::Vec3;

/// Result of slicing a tetrahedron with a plane.
/// Can produce 0, 3, or 4 intersection points (triangle or quad).
pub struct TetSliceResult {
    /// Intersection points (0, 3, or 4 vertices)
    pub vertices: Vec<Vec3>,
    /// Interpolation weights for vertex attributes (per-intersection point)
    /// Each entry is [(vert_a, vert_b, t), ...] where result = lerp(a, b, t)
    pub interpolation: Vec<(u32, u32, f32)>,
}

/// Computes the intersection of a tetrahedron with a slice plane.
///
/// Returns the intersection polygon vertices and interpolation data
/// for computing attribute values at intersection points.
pub fn slice_tet(
    v0: Vec3, v1: Vec3, v2: Vec3, v3: Vec3,
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> TetSliceResult {
    let verts = [v0, v1, v2, v3];

    // Compute signed distances
    let d: [f32; 4] = std::array::from_fn(|i| {
        (verts[i] - plane_origin).dot(plane_normal)
    });

    // Find edge intersections where sign changes
    let mut intersections = Vec::new();
    let mut interp_data = Vec::new();

    let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for &(i, j) in &edges {
        if d[i] * d[j] < 0.0 {
            let t = d[i] / (d[i] - d[j]);
            let point = verts[i].lerp(verts[j], t);
            intersections.push(point);
            interp_data.push((i as u32, j as u32, t));
        }
    }

    // Order vertices to form valid polygon (convex hull)
    if intersections.len() >= 3 {
        order_polygon_vertices(&mut intersections, &mut interp_data, plane_normal);
    }

    TetSliceResult {
        vertices: intersections,
        interpolation: interp_data,
    }
}

fn order_polygon_vertices(
    vertices: &mut [Vec3],
    interp: &mut [(u32, u32, f32)],
    normal: Vec3,
) {
    if vertices.len() < 3 { return; }

    // Compute centroid
    let centroid: Vec3 = vertices.iter().copied().sum::<Vec3>() / vertices.len() as f32;

    // Sort by angle around centroid
    let ref_dir = (vertices[0] - centroid).normalize();
    let mut indices: Vec<usize> = (0..vertices.len()).collect();

    indices.sort_by(|&a, &b| {
        let va = (vertices[a] - centroid).normalize();
        let vb = (vertices[b] - centroid).normalize();
        let angle_a = ref_dir.dot(va).acos() * normal.cross(ref_dir).dot(va).signum();
        let angle_b = ref_dir.dot(vb).acos() * normal.cross(ref_dir).dot(vb).signum();
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    // Reorder
    let sorted_verts: Vec<Vec3> = indices.iter().map(|&i| vertices[i]).collect();
    let sorted_interp: Vec<_> = indices.iter().map(|&i| interp[i]).collect();
    vertices.copy_from_slice(&sorted_verts);
    interp.copy_from_slice(&sorted_interp);
}
```

**Step 2: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_tet_no_intersection() {
        // Tet entirely on one side
        let result = slice_tet(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.5, 1.0, 1.0),
            Vec3::new(0.5, 2.0, 0.5),
            Vec3::ZERO,
            Vec3::Y,
        );
        assert!(result.vertices.is_empty());
    }

    #[test]
    fn test_slice_tet_triangle() {
        // Plane cuts one vertex off
        let result = slice_tet(
            Vec3::new(0.0, -1.0, 0.0),  // Below plane
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::ZERO,
            Vec3::Y,
        );
        assert_eq!(result.vertices.len(), 3);
    }

    #[test]
    fn test_slice_tet_quad() {
        // Plane cuts through middle
        let result = slice_tet(
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 1.0, 1.0),
            Vec3::ZERO,
            Vec3::Y,
        );
        assert_eq!(result.vertices.len(), 4);
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p polyscope-structures slice`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/slice_geometry.rs
git commit -m "feat(structures): implement tet-plane intersection algorithm"
```

---

### Task 4.2: Add Hex Slicing Support

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs`

**Step 1: Implement hex slicing**

Hexahedra are sliced by treating them as 5 or 6 tetrahedra, then merging the resulting polygons.

```rust
/// Slice a hexahedron by decomposing into 5 tetrahedra.
pub fn slice_hex(
    vertices: [Vec3; 8],
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> TetSliceResult {
    // Decompose hex into 5 tets (standard decomposition)
    let tet_indices = [
        [0, 1, 3, 4],
        [1, 2, 3, 6],
        [1, 4, 5, 6],
        [3, 4, 6, 7],
        [1, 3, 4, 6],
    ];

    let mut all_vertices = Vec::new();
    let mut all_interp = Vec::new();

    for tet in &tet_indices {
        let result = slice_tet(
            vertices[tet[0]],
            vertices[tet[1]],
            vertices[tet[2]],
            vertices[tet[3]],
            plane_origin,
            plane_normal,
        );
        all_vertices.extend(result.vertices);
        all_interp.extend(result.interpolation);
    }

    // Merge and deduplicate vertices
    merge_slice_vertices(&mut all_vertices, &mut all_interp);
    order_polygon_vertices(&mut all_vertices, &mut all_interp, plane_normal);

    TetSliceResult {
        vertices: all_vertices,
        interpolation: all_interp,
    }
}
```

**Step 2: Write tests**

**Step 3: Run tests**

Run: `cargo test -p polyscope-structures slice_hex`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/slice_geometry.rs
git commit -m "feat(structures): add hex slicing support"
```

---

### Task 4.3: Generate Slice Mesh for VolumeMesh

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`

**Step 1: Add slice geometry generation method**

```rust
impl VolumeMesh {
    /// Generates mesh geometry for the cross-section created by a slice plane.
    pub fn generate_slice_geometry(
        &self,
        plane: &SlicePlane,
    ) -> Option<SliceMeshData> {
        if !plane.is_enabled() {
            return None;
        }

        let plane_origin = plane.origin();
        let plane_normal = plane.normal();

        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();

        for cell in &self.cells {
            let cell_type = self.cell_type(cell);

            let slice = match cell_type {
                VolumeCellType::Tet => {
                    let [i0, i1, i2, i3, ..] = *cell;
                    slice_tet(
                        self.vertices[i0 as usize],
                        self.vertices[i1 as usize],
                        self.vertices[i2 as usize],
                        self.vertices[i3 as usize],
                        plane_origin,
                        plane_normal,
                    )
                }
                VolumeCellType::Hex => {
                    let hex_verts = std::array::from_fn(|i| {
                        self.vertices[cell[i] as usize]
                    });
                    slice_hex(hex_verts, plane_origin, plane_normal)
                }
            };

            if slice.vertices.len() >= 3 {
                // Triangulate (fan from first vertex)
                for i in 1..slice.vertices.len() - 1 {
                    vertices.push(slice.vertices[0]);
                    vertices.push(slice.vertices[i]);
                    vertices.push(slice.vertices[i + 1]);

                    // Normal is the slice plane normal (or its negation for consistent facing)
                    let n = plane_normal;
                    normals.push(n);
                    normals.push(n);
                    normals.push(n);

                    // Color from interior_color or interpolated quantity
                    colors.push(self.interior_color);
                    colors.push(self.interior_color);
                    colors.push(self.interior_color);
                }
            }
        }

        if vertices.is_empty() {
            return None;
        }

        Some(SliceMeshData {
            vertices,
            normals,
            colors,
        })
    }
}

pub struct SliceMeshData {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colors: Vec<Vec3>,
}
```

**Step 2: Run build**

Run: `cargo build -p polyscope-structures`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs
git commit -m "feat(structures): add slice geometry generation for volume mesh"
```

---

### Task 4.4: Create Slice Mesh Render Data

**Files:**
- Create: `crates/polyscope-render/src/slice_mesh_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Implement SliceMeshRenderData**

Uses the surface mesh shader but with pre-computed slice geometry.

```rust
pub struct SliceMeshRenderData {
    vertex_buffer: wgpu::Buffer,
    normal_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    vertex_count: u32,
    bind_group: wgpu::BindGroup,
}

impl SliceMeshRenderData {
    pub fn new(
        device: &wgpu::Device,
        slice_data: &SliceMeshData,
        // ... other params
    ) -> Self {
        // Create buffers from slice geometry
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slice_data: &SliceMeshData,
    ) {
        // Update buffers when slice plane moves
    }
}
```

**Step 2: Export from lib.rs**

**Step 3: Run build**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/slice_mesh_render.rs
git add crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add slice mesh rendering support"
```

---

### Task 4.5: Integrate Slice Mesh into Volume Mesh Rendering

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add slice mesh render data to VolumeMesh**

```rust
// In VolumeMesh struct
slice_render_data: Option<SliceMeshRenderData>,
slice_plane_cache: Option<(Vec3, Vec3)>, // (origin, normal) for invalidation
```

**Step 2: Update slice geometry in render**

```rust
impl Structure for VolumeMesh {
    fn render(&mut self, ctx: &mut RenderContext) {
        // ... existing exterior face rendering ...

        // Render slice geometry if a slice plane intersects this mesh
        for plane in &ctx.slice_planes {
            if plane.is_enabled() {
                // Check if cache is valid
                let cache_valid = self.slice_plane_cache.map_or(false, |(o, n)| {
                    o == plane.origin() && n == plane.normal()
                });

                if !cache_valid {
                    if let Some(slice_data) = self.generate_slice_geometry(plane) {
                        // Update or create render data
                        self.slice_render_data = Some(SliceMeshRenderData::new(
                            &ctx.device,
                            &slice_data,
                            // ...
                        ));
                        self.slice_plane_cache = Some((plane.origin(), plane.normal()));
                    } else {
                        self.slice_render_data = None;
                        self.slice_plane_cache = None;
                    }
                }

                if let Some(ref render_data) = self.slice_render_data {
                    render_data.draw(&mut ctx.render_pass);
                }
            }
        }
    }
}
```

**Step 3: Run integration test**

Run: `cargo test -p polyscope`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs
git add crates/polyscope/src/app.rs
git commit -m "feat: integrate slice mesh rendering for volume meshes"
```

---

### Task 4.6: Add Quantity Interpolation for Slice Capping

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs`
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`

**Step 1: Extend slice generation to interpolate vertex quantities**

When a vertex scalar/color quantity is active, interpolate values at slice points.

**Step 2: Update slice render data to include quantity colors**

**Step 3: Run test with quantities**

Run: `cargo run --example volume_mesh_demo`
Expected: Slice capping shows interpolated colors

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(structures): add quantity interpolation for volume mesh slice capping"
```

---

## Phase 5: Create Demo

### Task 5.1: Create Slice Plane Demo

**Files:**
- Create: `examples/slice_plane_demo.rs`

**Step 1: Write comprehensive demo**

```rust
//! Slice Plane Demo
//!
//! Demonstrates slice plane functionality with different structure types.

use glam::Vec3;
use polyscope::*;

fn main() {
    init();

    // Create a surface mesh (torus)
    let (vertices, indices) = generate_torus(1.0, 0.3, 32, 16);
    register_surface_mesh("torus", vertices, indices);

    // Create a point cloud
    let points: Vec<Vec3> = (0..1000)
        .map(|_| Vec3::new(
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
        ))
        .collect();
    register_point_cloud("random_points", points);

    // Create a volume mesh (cube of tets)
    let (verts, tets) = generate_tet_cube(1.0, 4);
    let mut vol = register_volume_mesh_tet("tet_cube", verts, tets);

    // Add a scalar quantity for visualization
    let scalars: Vec<f32> = (0..vol.num_vertices())
        .map(|i| (i as f32 / vol.num_vertices() as f32))
        .collect();
    vol.add_vertex_scalar_quantity("height", scalars);

    // Add a slice plane
    let mut plane = add_slice_plane("main_slicer");
    plane.set_origin(Vec3::ZERO);
    plane.set_normal(Vec3::new(1.0, 0.5, 0.0).normalize());

    show();
}
```

**Step 2: Run demo**

Run: `cargo run --example slice_plane_demo`
Expected: All structures sliced correctly, volume mesh shows capping

**Step 3: Commit**

```bash
git add examples/slice_plane_demo.rs
git commit -m "example: add slice plane demo"
```

---

## Testing Checklist

After each phase, verify:

1. **Phase 1 (Fragment Slicing)**:
   - [ ] Surface meshes are sliced correctly
   - [ ] Point clouds are sliced correctly
   - [ ] Curve networks are sliced correctly
   - [ ] Vectors are sliced correctly
   - [ ] Multiple slice planes work

2. **Phase 2 (Visualization)**:
   - [ ] Slice plane renders as semi-transparent grid
   - [ ] Plane position/orientation matches settings
   - [ ] Transparency and color controls work

3. **Phase 3 (UI)**:
   - [ ] Can add/remove slice planes from UI
   - [ ] Can toggle plane enabled/disabled
   - [ ] Can adjust origin, normal, color, transparency

4. **Phase 4 (Volume Mesh Capping)**:
   - [ ] Tet meshes show correct cross-section geometry
   - [ ] Hex meshes show correct cross-section geometry
   - [ ] Quantity colors interpolate correctly
   - [ ] Performance acceptable (caching works)

---

## Summary

This plan implements slice planes in four phases:

1. **Fragment Slicing**: Add shader-level discard for all structure types
2. **Visualization**: Render the slice plane itself with grid pattern
3. **UI Controls**: Add user interface for managing slice planes
4. **Volume Mesh Capping**: Generate cross-section geometry for interior visualization

The approach matches C++ Polyscope's functionality while adapting to wgpu/WGSL constraints (no geometry shaders, so capping is done CPU-side with caching).
