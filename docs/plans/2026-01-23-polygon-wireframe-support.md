# Polygon Wireframe Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `edge_is_real` buffer support so wireframe rendering only shows original polygon edges, not internal triangulation edges.

**Architecture:** Add a new GPU buffer `edge_is_real` that stores per-triangle-vertex flags indicating which edges are real polygon edges vs internal fan-triangulation edges. The shader uses this to selectively draw only real edges.

**Tech Stack:** Rust, wgpu, WGSL shaders

---

## Task 1: Compute edge_is_real in SurfaceMesh Triangulation

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Context:** The C++ Polyscope computes `edgeIsReal` during triangulation. For each triangle vertex, it stores a vec3 where each component indicates if that edge is a real polygon edge (1.0) or internal triangulation edge (0.0). For fan triangulation of polygon [v0,v1,v2,v3,...], the middle edges are always internal, and only the first and last edges of the fan are real.

**Step 1: Add edge_is_real field to SurfaceMesh struct**

Add after line 55 (after `corner_normals`):
```rust
    edge_is_real: Vec<Vec3>,
```

**Step 2: Add getter method**

Add after `corner_normals()` method (around line 176):
```rust
    /// Returns the edge_is_real flags for each triangle corner.
    /// For each triangle vertex, this is a Vec3 where:
    /// - x = 1.0 if edge from this vertex to next is real, 0.0 if internal
    /// - y = 1.0 if edge from next to prev is real (always real for middle edge)
    /// - z = 1.0 if edge from prev to this is real, 0.0 if internal
    pub fn edge_is_real(&self) -> &[Vec3] {
        &self.edge_is_real
    }
```

**Step 3: Initialize in constructor**

Add after `corner_normals: Vec::new(),` (around line 94):
```rust
            edge_is_real: Vec::new(),
```

**Step 4: Add compute_edge_is_real method**

Add after `compute_corner_normals` method:
```rust
    /// Computes edge_is_real flags for wireframe rendering.
    /// Marks which edges in the triangulation are real polygon edges vs internal.
    fn compute_edge_is_real(&mut self) {
        self.edge_is_real.clear();
        self.edge_is_real.reserve(self.triangulation.len() * 3);

        for (face_idx, range) in self.face_to_tri_range.iter().enumerate() {
            let face = &self.faces[face_idx];
            let d = face.len(); // degree of polygon
            let num_tris = range.end - range.start;

            for (j, _tri_idx) in range.clone().enumerate() {
                // For fan triangulation from v0:
                // Triangle j has vertices [v0, v_{j+1}, v_{j+2}]
                // Edge 0 (v0 -> v_{j+1}): real only if j == 0
                // Edge 1 (v_{j+1} -> v_{j+2}): always real (it's a polygon edge)
                // Edge 2 (v_{j+2} -> v0): real only if j == num_tris - 1

                let edge0_real = if j == 0 { 1.0 } else { 0.0 };
                let edge1_real = 1.0; // middle edge always real
                let edge2_real = if j == num_tris - 1 { 1.0 } else { 0.0 };

                // Each triangle corner gets the edge_is_real for all three edges
                // This matches C++ Polyscope's approach
                let edge_real = Vec3::new(edge0_real, edge1_real, edge2_real);
                self.edge_is_real.push(edge_real);
                self.edge_is_real.push(edge_real);
                self.edge_is_real.push(edge_real);
            }
        }
    }
```

**Step 5: Call compute_edge_is_real in recompute()**

Add after `self.compute_edges();` in the `recompute` method:
```rust
        self.compute_edge_is_real();
```

**Step 6: Run tests**

```bash
cargo test -p polyscope-structures
```

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(surface_mesh): compute edge_is_real for polygon wireframe support

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Add edge_is_real Buffer to SurfaceMeshRenderData

**Files:**
- Modify: `crates/polyscope-render/src/surface_mesh_render.rs`

**Step 1: Add edge_is_real_buffer field**

Add after `color_buffer` field (around line 66):
```rust
    /// Edge is real buffer - marks which edges are real polygon edges vs triangulation internal.
    pub edge_is_real_buffer: wgpu::Buffer,
```

**Step 2: Update new() signature**

Change the function signature to accept edge_is_real data:
```rust
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        vertex_normals: &[Vec3],
        edge_is_real: &[Vec3],
    ) -> Self {
```

**Step 3: Build edge_is_real_data in the triangle loop**

After `let mut barycentric_data` declaration, add:
```rust
        let mut edge_is_real_data: Vec<f32> = Vec::with_capacity(triangles.len() * 3 * 4);
```

Inside the triangle loop, after the barycentric line, add:
```rust
                // Edge is real flags (same for all vertices of triangle)
                let eir = edge_is_real[tri_vertex_idx];
                edge_is_real_data.extend_from_slice(&[eir.x, eir.y, eir.z, 0.0]);
```

where `tri_vertex_idx` is the global vertex index. Update the loop to track this:
```rust
        let mut tri_vertex_idx = 0;
        for tri in triangles {
            // ... existing code ...
            for (i, &vi) in tri.iter().enumerate() {
                // ... existing position, normal, color, barycentric code ...

                // Edge is real flags
                let eir = edge_is_real[tri_vertex_idx];
                edge_is_real_data.extend_from_slice(&[eir.x, eir.y, eir.z, 0.0]);

                tri_vertex_idx += 1;
            }
        }
```

**Step 4: Create edge_is_real_buffer**

After creating color_buffer:
```rust
        // Create edge_is_real buffer
        let edge_is_real_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh edge_is_real"),
            contents: bytemuck::cast_slice(&edge_is_real_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
```

**Step 5: Add binding 6 to bind_group**

Add after the colors binding entry:
```rust
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: edge_is_real_buffer.as_entire_binding(),
                },
```

**Step 6: Add to Self return**

```rust
            edge_is_real_buffer,
```

**Step 7: Commit**

```bash
git add crates/polyscope-render/src/surface_mesh_render.rs
git commit -m "feat(polyscope-render): add edge_is_real buffer to SurfaceMeshRenderData

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Update Bind Group Layout in RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add binding 6 entry**

After the colors storage buffer entry (binding 5), add:
```rust
                        // Edge is real storage buffer
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
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(polyscope-render): add edge_is_real binding to mesh bind group layout

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Update WGSL Shader to Use edge_is_real

**Files:**
- Modify: `crates/polyscope-render/src/shaders/surface_mesh.wgsl`

**Step 1: Add edge_is_real storage buffer binding**

After line 36 (colors binding):
```wgsl
@group(0) @binding(6) var<storage, read> edge_is_real: array<vec4<f32>>;
```

**Step 2: Add edge_is_real to VertexOutput**

After `barycentric` location:
```wgsl
    @location(4) edge_real: vec3<f32>,
```

**Step 3: Pass edge_is_real in vertex shader**

After `out.barycentric = bary;`:
```wgsl
    out.edge_real = edge_is_real[vertex_index].xyz;
```

**Step 4: Update wireframe logic in fragment shader**

Replace the wireframe section (lines 127-132):
```wgsl
    // Wireframe: if show_edges, mix edge_color based on barycentric distance
    // Only draw edges marked as real (not internal triangulation edges)
    if (mesh_uniforms.show_edges == 1u) {
        let bary = in.barycentric;
        let edge_real = in.edge_real;

        // Compute distance to each edge, but only consider real edges
        // Edge 0: opposite to vertex 0 (barycentric.x), between vertices 1-2
        // Edge 1: opposite to vertex 1 (barycentric.y), between vertices 2-0
        // Edge 2: opposite to vertex 2 (barycentric.z), between vertices 0-1
        var d = 1.0; // start with max distance
        if (edge_real.z > 0.5) { // edge from v0 to v1 (opposite to v2)
            d = min(d, bary.z);
        }
        if (edge_real.x > 0.5) { // edge from v1 to v2 (opposite to v0)
            d = min(d, bary.x);
        }
        if (edge_real.y > 0.5) { // edge from v2 to v0 (opposite to v1)
            d = min(d, bary.y);
        }

        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }
```

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/shaders/surface_mesh.wgsl
git commit -m "feat(shader): use edge_is_real for polygon wireframe rendering

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Update SurfaceMesh GPU Initialization

**Files:**
- Modify: `crates/polyscope/src/app.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Step 1: Update init_gpu_resources in SurfaceMesh**

Update the `init_gpu_resources` method to pass edge_is_real:
```rust
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        self.render_data = Some(SurfaceMeshRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &self.vertices,
            &self.triangulation,
            &self.vertex_normals,
            &self.edge_is_real,
        ));
    }
```

**Step 2: Run build**

```bash
cargo build
```

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(surface_mesh): pass edge_is_real to GPU render data

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Add Tests for edge_is_real

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Step 1: Add test for triangle edge_is_real**

```rust
    /// Test edge_is_real for a single triangle (all edges should be real).
    #[test]
    fn test_edge_is_real_triangle() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_tri", vertices, faces);

        // Single triangle - all 3 edges should be real
        assert_eq!(mesh.edge_is_real().len(), 3);
        // All vertices should have the same edge_is_real value
        for eir in mesh.edge_is_real() {
            assert_eq!(*eir, Vec3::new(1.0, 1.0, 1.0), "All edges in a triangle should be real");
        }
    }
```

**Step 2: Add test for quad edge_is_real**

```rust
    /// Test edge_is_real for a quad (internal triangulation edge should not be real).
    #[test]
    fn test_edge_is_real_quad() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];

        let mesh = SurfaceMesh::new("test_quad", vertices, faces);

        // Quad produces 2 triangles, so 6 corner entries
        assert_eq!(mesh.edge_is_real().len(), 6);

        // First triangle [0,1,2]: edge 0 (0->1) real, edge 1 (1->2) real, edge 2 (2->0) NOT real
        let eir0 = mesh.edge_is_real()[0];
        assert_eq!(eir0, Vec3::new(1.0, 1.0, 0.0), "First tri: edges 0,1 real, edge 2 internal");

        // Second triangle [0,2,3]: edge 0 (0->2) NOT real, edge 1 (2->3) real, edge 2 (3->0) real
        let eir1 = mesh.edge_is_real()[3];
        assert_eq!(eir1, Vec3::new(0.0, 1.0, 1.0), "Second tri: edge 0 internal, edges 1,2 real");
    }
```

**Step 3: Add test for pentagon edge_is_real**

```rust
    /// Test edge_is_real for a pentagon (multiple internal edges).
    #[test]
    fn test_edge_is_real_pentagon() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.5, 0.5, 0.0),
            Vec3::new(0.75, 1.0, 0.0),
            Vec3::new(-0.25, 0.5, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3, 4]];

        let mesh = SurfaceMesh::new("test_pentagon", vertices, faces);

        // Pentagon produces 3 triangles, so 9 corner entries
        assert_eq!(mesh.edge_is_real().len(), 9);

        // First triangle [0,1,2]: edge 0 real, edge 1 real, edge 2 NOT real
        let eir0 = mesh.edge_is_real()[0];
        assert_eq!(eir0, Vec3::new(1.0, 1.0, 0.0));

        // Middle triangle [0,2,3]: edge 0 NOT real, edge 1 real, edge 2 NOT real
        let eir1 = mesh.edge_is_real()[3];
        assert_eq!(eir1, Vec3::new(0.0, 1.0, 0.0));

        // Last triangle [0,3,4]: edge 0 NOT real, edge 1 real, edge 2 real
        let eir2 = mesh.edge_is_real()[6];
        assert_eq!(eir2, Vec3::new(0.0, 1.0, 1.0));
    }
```

**Step 4: Run tests**

```bash
cargo test -p polyscope-structures test_edge_is_real
```

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "test(surface_mesh): add edge_is_real tests for triangle, quad, pentagon

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Full Build and Integration Test

**Step 1: Build everything**

```bash
cargo build
```

**Step 2: Run all tests**

```bash
cargo test
```

**Step 3: Test with surface_mesh_demo**

```bash
cargo run --example surface_mesh_demo
```

Enable wireframe in the UI and verify:
- Triangle meshes show all edges
- Quad/polygon meshes only show original polygon edges, not internal triangulation lines

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "feat(polyscope): complete polygon wireframe support

- edge_is_real computed during triangulation
- GPU buffer for edge_is_real flags
- Shader uses edge_is_real for selective wireframe rendering
- Only real polygon edges shown, internal triangulation hidden

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Compute edge_is_real in SurfaceMesh | surface_mesh/mod.rs |
| 2 | Add edge_is_real buffer to render data | surface_mesh_render.rs |
| 3 | Update bind group layout | engine.rs |
| 4 | Update WGSL shader | surface_mesh.wgsl |
| 5 | Update GPU initialization | surface_mesh/mod.rs |
| 6 | Add tests | surface_mesh/mod.rs |
| 7 | Integration test | - |
