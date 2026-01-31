# Mesh GPU Picking Implementation Plan

## Goal

Add GPU-based face picking for surface meshes, matching C++ Polyscope's approach where all structures participate in GPU pick rendering. This enables clicking on individual mesh faces (polygon-aware, not just triangles).

## Current State

- **Point clouds**: Full GPU pick — sphere impostor rendering in pick buffer, `PickUniforms` with structure_id, 12+12 bit encoding (struct_id 12 bits + elem_id 12 bits)
- **Curve networks**: Full GPU pick — line mode and tube mode (ray-cylinder intersection)
- **Surface meshes**: No GPU pick. Structure-level selection uses CPU ray-triangle intersection (`pick_structure_at_ray`), which returns the structure but NOT the face index
- **Pick encoding**: `encode_pick_id(structure_id: u16, element_id: u16)` — 12-bit struct + 12-bit element. Max 4096 elements per structure — **insufficient for meshes with >4096 faces**

## Design: Flat 24-bit Global Index Encoding

Replace the current 12+12 split encoding with a flat 24-bit global index scheme:

- Each structure gets a **contiguous index range** `[global_start, global_start + num_elements)`
- Each element encodes its **absolute global index** into RGB using existing `index_to_color()`/`color_to_index()` functions
- On readback, decode the global index and binary-search to find which structure owns it
- **16.7M total pickable elements** across all structures (vs 4096 per structure today)
- **Index 0 reserved** as background (no hit), so all structures start from index >= 1

### Per-Structure Element Layout

For surface meshes, the element range maps to polygon face indices (not triangle indices). The shader uses `face_to_tri_range` mapping: each GPU triangle knows which original polygon face it belongs to.

## Files to Modify

### 1. `crates/polyscope-render/src/pick.rs` — Encoding redesign

- Deprecate/remove `encode_pick_id()` / `decode_pick_id()` (12+12 split)
- Keep `index_to_color()` / `color_to_index()` as the primary encoding
- Update `PickUniforms` to store `global_start: u32` instead of `structure_id: u32`
- Add `MeshPickUniforms` struct with `global_start: u32` and `model: mat4x4<f32>`
- Update `TubePickUniforms` to use `global_start: u32`

### 2. `crates/polyscope-render/src/engine/pick.rs` — ID management redesign

- Replace `structure_id_map: HashMap<(String, String), u16>` with range-based allocation:
  ```rust
  struct PickRange {
      global_start: u32,
      count: u32,
      type_name: String,
      name: String,
  }
  ```
- `assign_pick_range(type_name, name, num_elements) -> u32` — returns global_start
- `remove_pick_range(type_name, name)` — frees range (no compaction needed)
- `lookup_global_index(index: u32) -> Option<(&str, &str, u32)>` — binary search to find structure + local element index
- Keep `next_global_index: u32` as a monotonically increasing counter
- Update `pick_at()` to use `color_to_index()` instead of `decode_pick_id()`

### 3. `crates/polyscope-render/src/engine/pick.rs` — Mesh pick pipeline

- Add `init_mesh_pick_pipeline()` — creates pipeline for surface mesh face picking
- Uses existing `pick_bind_group_layout` (camera + pick_uniforms + positions storage buffer)
- TriangleList topology with Depth24Plus depth, Rgba8Unorm output, no blending
- Add `mesh_pick_pipeline: Option<wgpu::RenderPipeline>` field

### 4. `crates/polyscope-render/src/shaders/pick_mesh.wgsl` — Shader rewrite

Rewrite to use flat global index encoding with a **face index storage buffer**:

```wgsl
struct PickUniforms {
    global_start: u32,  // Start of this structure's pick range
    _padding0: f32,
    _padding1: f32,
    _padding2: f32,
    model: mat4x4<f32>, // Model transform for the mesh
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: PickUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> face_indices: array<u32>;  // tri_index -> face_index

fn index_to_color(index: u32) -> vec3<f32> {
    let r = f32((index >> 16u) & 0xFFu) / 255.0;
    let g = f32((index >> 8u) & 0xFFu) / 255.0;
    let b = f32(index & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let world_pos = (pick.model * positions[vertex_index]).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.tri_index = vertex_index / 3u;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let face_index = face_indices[in.tri_index];
    let global_index = pick.global_start + face_index;
    let color = index_to_color(global_index);
    return vec4<f32>(color, 1.0);
}
```

Key: The `face_indices` buffer maps each GPU triangle to its original polygon face index (built from `face_to_tri_range`).

### 5. `crates/polyscope-render/src/shaders/pick.wgsl` — Point cloud shader update

- Update `encode_pick_id()` to `index_to_color()` using `global_start` instead of `structure_id`
- Rename uniform field: `structure_id` -> `global_start`
- Each point encodes `global_start + instance_index`

### 6. `crates/polyscope-render/src/shaders/pick_curve.wgsl` — Curve network shader update

- Same encoding change as point cloud: `global_start + edge_index`

### 7. `crates/polyscope-render/src/shaders/pick_curve_tube.wgsl` — Tube pick shader update

- Same encoding change: `global_start + edge_index`

### 8. `crates/polyscope-structures/src/surface_mesh/mod.rs` — Pick resources

Add to `SurfaceMesh` struct:
```rust
pick_uniform_buffer: Option<wgpu::Buffer>,
pick_bind_group: Option<wgpu::BindGroup>,
pick_face_index_buffer: Option<wgpu::Buffer>,  // tri_index -> face_index mapping
```

Add methods:
- `init_pick_resources(device, layout, camera_buf, global_start)` — creates pick uniform buffer, face index buffer (from `face_to_tri_range`), bind group
- `pick_bind_group() -> Option<&BindGroup>`
- `update_pick_uniforms(queue)` — writes model transform to pick uniforms

The face index buffer is built by iterating `face_to_tri_range`: for each face `f`, for each triangle in `face_to_tri_range[f]`, store `f` as `u32`.

### 9. `crates/polyscope-structures/src/point_cloud/mod.rs` — Update pick encoding

- Update `init_pick_resources` to accept `global_start: u32` instead of `structure_id: u16`
- Update `PickUniforms` usage to store `global_start`

### 10. `crates/polyscope-structures/src/curve_network/mod.rs` — Update pick encoding

- Same changes as point cloud: `global_start: u32`

### 11. `crates/polyscope/src/app/render.rs` — Pick pass integration

In the init phase (~line 93), after SurfaceMesh render data init:
```rust
if mesh.pick_bind_group().is_none() && mesh.render_data().is_some() {
    let num_faces = mesh.num_faces();
    let global_start = engine.assign_pick_range("SurfaceMesh", &name, num_faces as u32);
    mesh.init_pick_resources(&engine.device, engine.mesh_pick_bind_group_layout(), engine.camera_buffer(), global_start);
}
```

In the pick pass (~line 656), add mesh pick rendering:
```rust
// Draw surface meshes to pick buffer
if engine.has_mesh_pick_pipeline() {
    pick_pass.set_pipeline(engine.mesh_pick_pipeline());
    for structure in ctx.registry.iter() {
        if structure.type_name() == "SurfaceMesh" {
            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                if let Some(pick_bind_group) = mesh.pick_bind_group() {
                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                    pick_pass.draw(0..mesh.num_triangulation_vertices(), 0..1);
                }
            }
        }
    }
}
```

### 12. `crates/polyscope/src/app/picking.rs` — Update GPU pick decoding

Update `gpu_pick_at()`:
```rust
pub(super) fn gpu_pick_at(&self, x: u32, y: u32) -> Option<(String, String, u32)> {
    let engine = self.engine.as_ref()?;
    let global_index = engine.pick_at(x, y)?;  // Now returns u32
    if global_index == 0 { return None; }  // Background
    let (type_name, name, local_index) = engine.lookup_global_index(global_index)?;
    Some((type_name.to_string(), name.to_string(), local_index))
}
```

### 13. `crates/polyscope/src/app/input.rs` — Handle mesh GPU picks

In the click handling, add `SurfaceMesh` to the GPU pick handling (currently only handles PointCloud and CurveNetwork):
```rust
if let Some((type_name, name, idx)) = gpu_picked {
    match type_name.as_str() {
        "PointCloud" => { /* existing */ }
        "CurveNetwork" => { /* existing */ }
        "SurfaceMesh" => {
            // GPU pick gives us face index directly — validate with ray depth
            mesh_hit = Some((name, idx, ray_depth));
        }
        _ => {}
    }
}
```

## Implementation Order

1. **Pick encoding migration** (pick.rs, engine/pick.rs) — flat 24-bit encoding + range management
2. **Update existing shaders** (pick.wgsl, pick_curve.wgsl, pick_curve_tube.wgsl) — use `global_start`
3. **Update existing structures** (point_cloud, curve_network) — pass `global_start` instead of `structure_id`
4. **Update render.rs init** — use `assign_pick_range()` instead of `assign_structure_id()`
5. **Add mesh pick shader** (pick_mesh.wgsl) — rewrite with face_indices buffer
6. **Add mesh pick pipeline** (engine/pick.rs) — new pipeline with 4-binding layout
7. **Add mesh pick resources** (surface_mesh/mod.rs) — uniform buffer, face index buffer, bind group
8. **Wire up render.rs** — init mesh pick resources + draw in pick pass
9. **Update pick decoding** (picking.rs, input.rs) — handle mesh faces in GPU pick results
10. **Build + test + clippy**

## Key Design Decisions

1. **Flat 24-bit vs 12+12 split**: Flat encoding supports 16.7M total elements (vs 4096 per structure). This is the same approach C++ Polyscope uses (though C++ uses 66 bits across 3 channels × 22 bits).

2. **Face index buffer**: Rather than encoding face IDs in vertex attributes, we use a separate `storage<read>` buffer mapping `tri_index -> face_index`. This leverages the existing `face_to_tri_range` data and keeps the vertex buffer format unchanged.

3. **Model transform in pick uniforms**: The mesh pick shader needs the model transform to render in world space (positions in storage buffer are in object space). Point cloud pick already handles this via the billboard expansion, but meshes need explicit transform.

4. **Monotonic range allocation**: `next_global_index` only increases. When a structure is removed, its range is "freed" but not reused until overflow (at 16.7M this is fine). This avoids fragmentation complexity.

5. **No vertex/edge/halfedge picking (yet)**: C++ Polyscope uses barycentric radius tests in the fragment shader to distinguish vertex/face/edge/halfedge/corner picks. We start with face-only picking and can add vertex/edge selection later by subdividing the face's pick range or using barycentric tests.
