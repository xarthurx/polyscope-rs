# Phase 5: SurfaceMesh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement full SurfaceMesh support with polygon storage, multiple shading modes, wireframe, and all quantity types matching C++ Polyscope.

**Architecture:** SurfaceMesh stores variable-length polygon faces and computes triangulation for rendering. Uses instanced rendering for vectors, barycentric coordinates for wireframe, and per-element color encoding for picking. Quantities follow the same pattern as PointCloud.

**Tech Stack:** Rust, wgpu, egui, glam, bytemuck

---

## Task 1: Core Data Structure

Rewrite the SurfaceMesh struct to support polygons and computed data.

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`
- Test: `crates/polyscope-structures/src/surface_mesh/mod.rs` (inline tests)

**Step 1: Write the failing test**

Add this test to `crates/polyscope-structures/src/surface_mesh/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_mesh_creation_triangles() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = SurfaceMesh::new("test_mesh", vertices.clone(), faces);

        assert_eq!(mesh.num_vertices(), 3);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.vertices(), &vertices);
    }

    #[test]
    fn test_surface_mesh_creation_quad() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]]; // Quad
        let mesh = SurfaceMesh::new("quad_mesh", vertices, faces);

        assert_eq!(mesh.num_vertices(), 4);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.face(0), &[0, 1, 2, 3]);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures test_surface_mesh_creation -- --nocapture`
Expected: FAIL with compilation errors (old signature incompatible)

**Step 3: Write minimal implementation**

Replace the entire `mod.rs` with:

```rust
//! Surface mesh structure.

use std::ops::Range;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};

/// Shading style for surface mesh rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadeStyle {
    #[default]
    Smooth,
    Flat,
    TriFlat,
}

/// Backface rendering policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackfacePolicy {
    #[default]
    Identical,
    Different,
    Custom,
    Cull,
}

/// A surface mesh structure (triangular or polygonal).
pub struct SurfaceMesh {
    name: String,
    vertices: Vec<Vec3>,
    faces: Vec<Vec<u32>>,  // Variable-size polygons

    // Computed data (populated on demand)
    triangulation: Vec<[u32; 3]>,
    face_to_tri_range: Vec<Range<usize>>,
    vertex_normals: Vec<Vec3>,
    face_normals: Vec<Vec3>,
    corner_normals: Vec<Vec3>,
    edges: Vec<(u32, u32)>,

    // Render options
    shade_style: ShadeStyle,
    edge_width: f32,
    edge_color: Vec3,
    show_edges: bool,
    backface_policy: BackfacePolicy,
    backface_color: Vec3,
    surface_color: Vec3,
    transparency: f32,

    // State
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,
    needs_recompute: bool,
}

impl SurfaceMesh {
    /// Creates a new surface mesh from polygons.
    pub fn new(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        faces: Vec<Vec<u32>>,
    ) -> Self {
        let mut mesh = Self {
            name: name.into(),
            vertices,
            faces,
            triangulation: Vec::new(),
            face_to_tri_range: Vec::new(),
            vertex_normals: Vec::new(),
            face_normals: Vec::new(),
            corner_normals: Vec::new(),
            edges: Vec::new(),
            shade_style: ShadeStyle::default(),
            edge_width: 1.0,
            edge_color: Vec3::new(0.0, 0.0, 0.0),
            show_edges: false,
            backface_policy: BackfacePolicy::default(),
            backface_color: Vec3::new(0.3, 0.3, 0.3),
            surface_color: Vec3::new(0.7, 0.7, 0.7),
            transparency: 1.0,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            needs_recompute: true,
        };
        mesh.recompute();
        mesh
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of faces.
    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }

    /// Returns the vertices.
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Returns a face by index.
    pub fn face(&self, index: usize) -> &[u32] {
        &self.faces[index]
    }

    /// Returns all faces.
    pub fn faces(&self) -> &[Vec<u32>] {
        &self.faces
    }

    /// Returns the triangulation.
    pub fn triangulation(&self) -> &[[u32; 3]] {
        &self.triangulation
    }

    /// Recomputes derived data (triangulation, normals, edges).
    fn recompute(&mut self) {
        self.compute_triangulation();
        self.compute_face_normals();
        self.compute_vertex_normals();
        self.compute_edges();
        self.needs_recompute = false;
    }

    fn compute_triangulation(&mut self) {
        self.triangulation.clear();
        self.face_to_tri_range.clear();

        for face in &self.faces {
            let start = self.triangulation.len();
            // Fan triangulation: v0, v1, v2; v0, v2, v3; ...
            if face.len() >= 3 {
                for i in 1..(face.len() - 1) {
                    self.triangulation.push([face[0], face[i as u32] as u32, face[i as u32 + 1] as u32]);
                }
            }
            let end = self.triangulation.len();
            self.face_to_tri_range.push(start..end);
        }
    }

    fn compute_face_normals(&mut self) {
        self.face_normals.clear();

        for face in &self.faces {
            if face.len() >= 3 {
                let v0 = self.vertices[face[0] as usize];
                let v1 = self.vertices[face[1] as usize];
                let v2 = self.vertices[face[2] as usize];
                let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
                self.face_normals.push(normal);
            } else {
                self.face_normals.push(Vec3::Y);
            }
        }
    }

    fn compute_vertex_normals(&mut self) {
        self.vertex_normals = vec![Vec3::ZERO; self.vertices.len()];

        for (face_idx, face) in self.faces.iter().enumerate() {
            let normal = self.face_normals[face_idx];
            // Area-weighted contribution (use face normal)
            for &vi in face {
                self.vertex_normals[vi as usize] += normal;
            }
        }

        for normal in &mut self.vertex_normals {
            *normal = normal.normalize_or_zero();
        }
    }

    fn compute_edges(&mut self) {
        use std::collections::HashSet;
        let mut edge_set = HashSet::new();

        for face in &self.faces {
            let n = face.len();
            for i in 0..n {
                let v0 = face[i];
                let v1 = face[(i + 1) % n];
                let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };
                edge_set.insert(edge);
            }
        }

        self.edges = edge_set.into_iter().collect();
        self.edges.sort();
    }

    /// Returns the unique edges.
    pub fn edges(&self) -> &[(u32, u32)] {
        &self.edges
    }

    /// Returns the number of edges.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Returns the vertex normals.
    pub fn vertex_normals(&self) -> &[Vec3] {
        &self.vertex_normals
    }

    /// Returns the face normals.
    pub fn face_normals(&self) -> &[Vec3] {
        &self.face_normals
    }

    // Getters/setters for render options
    pub fn shade_style(&self) -> ShadeStyle { self.shade_style }
    pub fn set_shade_style(&mut self, style: ShadeStyle) { self.shade_style = style; }
    pub fn surface_color(&self) -> Vec3 { self.surface_color }
    pub fn set_surface_color(&mut self, color: Vec3) { self.surface_color = color; }
    pub fn transparency(&self) -> f32 { self.transparency }
    pub fn set_transparency(&mut self, alpha: f32) { self.transparency = alpha; }
    pub fn show_edges(&self) -> bool { self.show_edges }
    pub fn set_show_edges(&mut self, show: bool) { self.show_edges = show; }
    pub fn edge_width(&self) -> f32 { self.edge_width }
    pub fn set_edge_width(&mut self, width: f32) { self.edge_width = width; }
    pub fn edge_color(&self) -> Vec3 { self.edge_color }
    pub fn set_edge_color(&mut self, color: Vec3) { self.edge_color = color; }
    pub fn backface_policy(&self) -> BackfacePolicy { self.backface_policy }
    pub fn set_backface_policy(&mut self, policy: BackfacePolicy) { self.backface_policy = policy; }
    pub fn backface_color(&self) -> Vec3 { self.backface_color }
    pub fn set_backface_color(&mut self, color: Vec3) { self.backface_color = color; }
}

impl Structure for SurfaceMesh {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn type_name(&self) -> &'static str { "SurfaceMesh" }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        if self.vertices.is_empty() { return None; }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for &v in &self.vertices {
            min = min.min(v);
            max = max.max(v);
        }

        let corners = [
            self.transform.transform_point3(Vec3::new(min.x, min.y, min.z)),
            self.transform.transform_point3(Vec3::new(max.x, min.y, min.z)),
            self.transform.transform_point3(Vec3::new(min.x, max.y, min.z)),
            self.transform.transform_point3(Vec3::new(max.x, max.y, min.z)),
            self.transform.transform_point3(Vec3::new(min.x, min.y, max.z)),
            self.transform.transform_point3(Vec3::new(max.x, min.y, max.z)),
            self.transform.transform_point3(Vec3::new(min.x, max.y, max.z)),
            self.transform.transform_point3(Vec3::new(max.x, max.y, max.z)),
        ];

        let mut world_min = Vec3::splat(f32::MAX);
        let mut world_max = Vec3::splat(f32::MIN);
        for corner in corners {
            world_min = world_min.min(corner);
            world_max = world_max.max(corner);
        }
        Some((world_min, world_max))
    }

    fn length_scale(&self) -> f32 {
        self.bounding_box()
            .map(|(min, max)| (max - min).length())
            .unwrap_or(1.0)
    }

    fn transform(&self) -> Mat4 { self.transform }
    fn set_transform(&mut self, transform: Mat4) { self.transform = transform; }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled { return; }
        // TODO: Implement mesh rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled { return; }
        // TODO: Implement mesh picking
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // TODO: Implement pick UI
    }

    fn refresh(&mut self) {
        self.recompute();
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for SurfaceMesh {
    fn add_quantity(&mut self, quantity: Box<dyn Quantity>) {
        self.quantities.push(quantity);
    }

    fn get_quantity(&self, name: &str) -> Option<&dyn Quantity> {
        self.quantities.iter().find(|q| q.name() == name).map(|q| q.as_ref())
    }

    fn get_quantity_mut(&mut self, name: &str) -> Option<&mut Box<dyn Quantity>> {
        self.quantities.iter_mut().find(|q| q.name() == name)
    }

    fn remove_quantity(&mut self, name: &str) -> Option<Box<dyn Quantity>> {
        let idx = self.quantities.iter().position(|q| q.name() == name)?;
        Some(self.quantities.remove(idx))
    }

    fn quantities(&self) -> &[Box<dyn Quantity>] {
        &self.quantities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_mesh_creation_triangles() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = SurfaceMesh::new("test_mesh", vertices.clone(), faces);

        assert_eq!(mesh.num_vertices(), 3);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.vertices(), &vertices);
    }

    #[test]
    fn test_surface_mesh_creation_quad() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = SurfaceMesh::new("quad_mesh", vertices, faces);

        assert_eq!(mesh.num_vertices(), 4);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.face(0), &[0, 1, 2, 3]);
    }

    #[test]
    fn test_quad_triangulation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = SurfaceMesh::new("quad_mesh", vertices, faces);

        // Quad should be triangulated into 2 triangles
        assert_eq!(mesh.triangulation().len(), 2);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);
        assert_eq!(mesh.triangulation()[1], [0, 2, 3]);
    }

    #[test]
    fn test_face_normals() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = SurfaceMesh::new("test", vertices, faces);

        // Normal should point in +Z direction
        let normal = mesh.face_normals()[0];
        assert!((normal - Vec3::Z).length() < 0.001);
    }

    #[test]
    fn test_edges() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = SurfaceMesh::new("test", vertices, faces);

        assert_eq!(mesh.num_edges(), 3);
        // Edges should be sorted pairs
        assert!(mesh.edges().contains(&(0, 1)));
        assert!(mesh.edges().contains(&(0, 2)));
        assert!(mesh.edges().contains(&(1, 2)));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures test_surface_mesh -- --nocapture`
Expected: PASS (all 5 tests)

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(surface_mesh): rewrite with polygon support and computed data

- Variable-length polygon faces with fan triangulation
- Computes vertex normals, face normals, edges
- ShadeStyle and BackfacePolicy enums
- Render option getters/setters
- Tests for triangulation, normals, edges"
```

---

## Task 2: GPU Buffer Management

Create the SurfaceMesh render data structure and buffer initialization.

**Files:**
- Create: `crates/polyscope-render/src/surface_mesh_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`
- Test: inline in `surface_mesh_render.rs`

**Step 1: Write the failing test**

Create `crates/polyscope-render/src/surface_mesh_render.rs`:

```rust
//! Surface mesh GPU rendering resources.

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_mesh_uniforms_default() {
        let uniforms = MeshUniforms::default();
        assert_eq!(uniforms.shade_style, 0); // Smooth
        assert_eq!(uniforms.show_edges, 0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-render test_mesh_uniforms -- --nocapture`
Expected: FAIL with "MeshUniforms not found"

**Step 3: Write minimal implementation**

Complete `crates/polyscope-render/src/surface_mesh_render.rs`:

```rust
//! Surface mesh GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// Uniforms for surface mesh rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    /// 0 = smooth, 1 = flat, 2 = tri-flat
    pub shade_style: u32,
    /// 0 = off, 1 = on
    pub show_edges: u32,
    pub edge_width: f32,
    pub transparency: f32,
    pub surface_color: [f32; 4],
    pub edge_color: [f32; 4],
    /// 0 = identical, 1 = different, 2 = custom, 3 = cull
    pub backface_policy: u32,
    pub _padding: [f32; 3],
    pub backface_color: [f32; 4],
}

impl Default for MeshUniforms {
    fn default() -> Self {
        Self {
            shade_style: 0,
            show_edges: 0,
            edge_width: 1.0,
            transparency: 1.0,
            surface_color: [0.7, 0.7, 0.7, 1.0],
            edge_color: [0.0, 0.0, 0.0, 1.0],
            backface_policy: 0,
            _padding: [0.0; 3],
            backface_color: [0.3, 0.3, 0.3, 1.0],
        }
    }
}

/// GPU resources for rendering a surface mesh.
pub struct SurfaceMeshRenderData {
    /// Vertex positions (vec4 for alignment).
    pub vertex_buffer: wgpu::Buffer,
    /// Triangle indices.
    pub index_buffer: wgpu::Buffer,
    /// Vertex normals (for smooth shading).
    pub normal_buffer: wgpu::Buffer,
    /// Barycentric coordinates (for wireframe).
    pub barycentric_buffer: wgpu::Buffer,
    /// Per-vertex colors (optional quantity).
    pub color_buffer: wgpu::Buffer,
    /// Uniform buffer.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group.
    pub bind_group: wgpu::BindGroup,
    /// Number of triangles.
    pub num_triangles: u32,
    /// Number of indices (num_triangles * 3).
    pub num_indices: u32,
}

impl SurfaceMeshRenderData {
    /// Creates new render data from mesh geometry.
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        vertex_normals: &[Vec3],
    ) -> Self {
        let num_triangles = triangles.len() as u32;
        let num_indices = num_triangles * 3;

        // Vertex buffer (vec4 for alignment)
        let vertex_data: Vec<f32> = vertices
            .iter()
            .flat_map(|v| [v.x, v.y, v.z, 0.0])
            .collect();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Index buffer
        let index_data: Vec<u32> = triangles.iter().flat_map(|t| [t[0], t[1], t[2]]).collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // Normal buffer (vec4 for alignment)
        let normal_data: Vec<f32> = vertex_normals
            .iter()
            .flat_map(|n| [n.x, n.y, n.z, 0.0])
            .collect();
        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh normals"),
            contents: bytemuck::cast_slice(&normal_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Barycentric coordinates buffer
        // Each triangle vertex gets [1,0,0], [0,1,0], [0,0,1]
        let bary_data: Vec<f32> = (0..num_triangles)
            .flat_map(|_| {
                [
                    1.0, 0.0, 0.0, 0.0, // First vertex
                    0.0, 1.0, 0.0, 0.0, // Second vertex
                    0.0, 0.0, 1.0, 0.0, // Third vertex
                ]
            })
            .collect();
        let barycentric_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh barycentric"),
            contents: bytemuck::cast_slice(&bary_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Color buffer (default white)
        let color_data: Vec<f32> = vec![1.0; vertices.len() * 4];
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh colors"),
            contents: bytemuck::cast_slice(&color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Uniform buffer
        let uniforms = MeshUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: normal_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: barycentric_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: color_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            vertex_buffer,
            index_buffer,
            normal_buffer,
            barycentric_buffer,
            color_buffer,
            uniform_buffer,
            bind_group,
            num_triangles,
            num_indices,
        }
    }

    /// Updates uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &MeshUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Updates vertex colors.
    pub fn update_colors(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect();
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&color_data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_uniforms_default() {
        let uniforms = MeshUniforms::default();
        assert_eq!(uniforms.shade_style, 0);
        assert_eq!(uniforms.show_edges, 0);
        assert_eq!(uniforms.transparency, 1.0);
    }

    #[test]
    fn test_mesh_uniforms_size() {
        // Ensure proper alignment for GPU
        assert_eq!(std::mem::size_of::<MeshUniforms>() % 16, 0);
    }
}
```

Update `crates/polyscope-render/src/lib.rs` to add:

```rust
pub mod surface_mesh_render;
pub use surface_mesh_render::{MeshUniforms, SurfaceMeshRenderData};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-render test_mesh_uniforms -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-render/src/surface_mesh_render.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add SurfaceMeshRenderData for GPU buffer management

- MeshUniforms struct with shade style, wireframe, backface options
- Buffer creation for vertices, indices, normals, barycentric, colors
- Bind group setup matching mesh shader layout"
```

---

## Task 3: Surface Mesh Shader

Create the WGSL shader for surface mesh rendering with shading modes and wireframe.

**Files:**
- Create: `crates/polyscope-render/src/shaders/surface_mesh.wgsl`

**Step 1: Create the shader file**

```wgsl
// Surface mesh shader with smooth/flat shading and wireframe support

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct MeshUniforms {
    shade_style: u32,      // 0 = smooth, 1 = flat, 2 = tri-flat
    show_edges: u32,       // 0 = off, 1 = on
    edge_width: f32,
    transparency: f32,
    surface_color: vec4<f32>,
    edge_color: vec4<f32>,
    backface_policy: u32,  // 0 = identical, 1 = different, 2 = custom, 3 = cull
    _padding: vec3<f32>,
    backface_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> mesh_uniforms: MeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> barycentrics: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> colors: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = positions[vertex_index].xyz;
    let normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index].rgb;

    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.world_position = pos;
    out.world_normal = normal;
    out.barycentric = bary;
    out.vertex_color = color;

    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    // Handle backface culling
    if (mesh_uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Determine base color based on backface policy
    var base_color = mesh_uniforms.surface_color.rgb;
    if (!front_facing) {
        switch (mesh_uniforms.backface_policy) {
            case 1u: { // Different - slightly darker
                base_color = base_color * 0.6;
            }
            case 2u: { // Custom
                base_color = mesh_uniforms.backface_color.rgb;
            }
            default: {} // Identical - use same color
        }
    }

    // Apply per-vertex color if available (for quantities)
    let use_vertex_color = in.vertex_color.r + in.vertex_color.g + in.vertex_color.b > 0.01;
    if (use_vertex_color) {
        base_color = in.vertex_color;
    }

    // Compute lighting
    let normal = normalize(in.world_normal);
    let effective_normal = select(-normal, normal, front_facing);

    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.3;
    let diffuse = max(dot(effective_normal, light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;

    var color = base_color * lighting;

    // Wireframe rendering using barycentric coordinates
    if (mesh_uniforms.show_edges == 1u) {
        let d = min(in.barycentric.x, min(in.barycentric.y, in.barycentric.z));
        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }

    return vec4<f32>(color, mesh_uniforms.transparency);
}
```

**Step 2: Run build to verify shader compiles**

Run: `cargo build -p polyscope-render`
Expected: PASS (shader file doesn't need compilation at build time)

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/surface_mesh.wgsl
git commit -m "feat(render): add surface mesh shader with shading and wireframe

- Smooth shading with interpolated vertex normals
- Lighting with ambient + diffuse
- Barycentric wireframe rendering
- Backface policy handling (identical/different/custom/cull)
- Per-vertex color support for quantities"
```

---

## Task 4: Pipeline Creation

Add mesh pipeline creation to RenderEngine.

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Read existing engine code**

Run: Read the engine.rs file to understand the pipeline creation pattern.

**Step 2: Add mesh pipeline**

Add to `RenderEngine`:

```rust
// In struct RenderEngine
pub mesh_pipeline: Option<wgpu::RenderPipeline>,
mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,

// In new_windowed() or init methods
fn create_mesh_pipeline(&mut self) {
    let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mesh bind group layout"),
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
            // Positions (storage)
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
            // Normals (storage)
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
            // Barycentrics (storage)
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
            // Colors (storage)
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
        ],
    });

    let shader_source = include_str!("shaders/surface_mesh.wgsl");
    let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mesh shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mesh pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh pipeline"),
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
                format: self.surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Handle culling in shader for backface policy
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    self.mesh_bind_group_layout = Some(bind_group_layout);
    self.mesh_pipeline = Some(pipeline);
}

pub fn mesh_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    self.mesh_bind_group_layout.as_ref().expect("mesh pipeline not initialized")
}
```

**Step 3: Run build to verify**

Run: `cargo build -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add mesh rendering pipeline to RenderEngine

- Bind group layout with camera, mesh uniforms, buffers
- Shader compilation from surface_mesh.wgsl
- Pipeline with alpha blending, depth test, no culling"
```

---

## Task 5: Integrate SurfaceMesh into App

Wire up SurfaceMesh rendering in the main application.

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs` (add GPU init/update)
- Modify: `crates/polyscope/src/app.rs` (render meshes)

**Step 1: Add GPU resource management to SurfaceMesh**

Add to `SurfaceMesh`:

```rust
use polyscope_render::{SurfaceMeshRenderData, MeshUniforms};

// In struct
render_data: Option<SurfaceMeshRenderData>,

// Methods
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
    ));
}

pub fn render_data(&self) -> Option<&SurfaceMeshRenderData> {
    self.render_data.as_ref()
}

pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
    let Some(render_data) = &self.render_data else { return; };

    let uniforms = MeshUniforms {
        shade_style: self.shade_style as u32,
        show_edges: if self.show_edges { 1 } else { 0 },
        edge_width: self.edge_width,
        transparency: self.transparency,
        surface_color: [self.surface_color.x, self.surface_color.y, self.surface_color.z, 1.0],
        edge_color: [self.edge_color.x, self.edge_color.y, self.edge_color.z, 1.0],
        backface_policy: self.backface_policy as u32,
        _padding: [0.0; 3],
        backface_color: [self.backface_color.x, self.backface_color.y, self.backface_color.z, 1.0],
    };
    render_data.update_uniforms(queue, &uniforms);
}
```

**Step 2: Add mesh rendering to App::render()**

In `app.rs`, add to the render loop:

```rust
use polyscope_structures::SurfaceMesh;

// In render(), after point cloud init:
if structure.type_name() == "SurfaceMesh" {
    if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
        if mesh.render_data().is_none() {
            mesh.init_gpu_resources(
                &engine.device,
                engine.mesh_bind_group_layout(),
                engine.camera_buffer(),
            );
        }
    }
}

// In render(), after point cloud GPU update:
if structure.type_name() == "SurfaceMesh" {
    if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
        mesh.update_gpu_buffers(&engine.queue);
    }
}

// In render pass, after point cloud drawing:
if let Some(pipeline) = &engine.mesh_pipeline {
    render_pass.set_pipeline(pipeline);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !structure.is_enabled() { continue; }
            if structure.type_name() == "SurfaceMesh" {
                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                    if let Some(render_data) = mesh.render_data() {
                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                    }
                }
            }
        }
    });
}
```

**Step 3: Run build and test**

Run: `cargo build -p polyscope`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/mod.rs crates/polyscope/src/app.rs
git commit -m "feat: integrate SurfaceMesh rendering into app

- GPU resource init and buffer update methods on SurfaceMesh
- Render pass draws indexed triangles with mesh pipeline
- Uniform update syncs shade style, wireframe, colors"
```

---

## Task 6: SurfaceMesh UI

Add UI controls for surface mesh in the left panel.

**Files:**
- Modify: `crates/polyscope-ui/src/structure_ui.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs` (add `build_egui_ui`)

**Step 1: Add UI builder function**

Add to `structure_ui.rs`:

```rust
/// Builds UI for a surface mesh.
pub fn build_surface_mesh_ui(
    ui: &mut Ui,
    num_vertices: usize,
    num_faces: usize,
    num_edges: usize,
    shade_style: &mut u32,
    surface_color: &mut [f32; 3],
    transparency: &mut f32,
    show_edges: &mut bool,
    edge_width: &mut f32,
    edge_color: &mut [f32; 3],
    backface_policy: &mut u32,
) -> bool {
    let mut changed = false;

    ui.label(format!("Vertices: {num_vertices}"));
    ui.label(format!("Faces: {num_faces}"));
    ui.label(format!("Edges: {num_edges}"));

    ui.separator();

    // Shade style
    egui::ComboBox::from_label("Shading")
        .selected_text(match *shade_style {
            0 => "Smooth",
            1 => "Flat",
            _ => "Tri-Flat",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(shade_style, 0, "Smooth").changed() { changed = true; }
            if ui.selectable_value(shade_style, 1, "Flat").changed() { changed = true; }
            if ui.selectable_value(shade_style, 2, "Tri-Flat").changed() { changed = true; }
        });

    // Surface color
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(surface_color).changed() { changed = true; }
    });

    // Transparency
    ui.horizontal(|ui| {
        ui.label("Opacity:");
        if ui.add(egui::Slider::new(transparency, 0.0..=1.0)).changed() { changed = true; }
    });

    ui.separator();

    // Wireframe
    ui.horizontal(|ui| {
        if ui.checkbox(show_edges, "Show edges").changed() { changed = true; }
    });

    if *show_edges {
        ui.indent("edges", |ui| {
            ui.horizontal(|ui| {
                ui.label("Width:");
                if ui.add(egui::DragValue::new(edge_width).speed(0.1).range(0.1..=5.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_rgb(edge_color).changed() { changed = true; }
            });
        });
    }

    ui.separator();

    // Backface policy
    egui::ComboBox::from_label("Backface")
        .selected_text(match *backface_policy {
            0 => "Identical",
            1 => "Different",
            2 => "Custom",
            _ => "Cull",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(backface_policy, 0, "Identical").changed() { changed = true; }
            if ui.selectable_value(backface_policy, 1, "Different").changed() { changed = true; }
            if ui.selectable_value(backface_policy, 2, "Custom").changed() { changed = true; }
            if ui.selectable_value(backface_policy, 3, "Cull").changed() { changed = true; }
        });

    changed
}
```

**Step 2: Add `build_egui_ui` to SurfaceMesh**

```rust
pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
    let mut shade_style = self.shade_style as u32;
    let mut color = [self.surface_color.x, self.surface_color.y, self.surface_color.z];
    let mut transparency = self.transparency;
    let mut show_edges = self.show_edges;
    let mut edge_width = self.edge_width;
    let mut edge_color = [self.edge_color.x, self.edge_color.y, self.edge_color.z];
    let mut backface_policy = self.backface_policy as u32;

    if polyscope_ui::build_surface_mesh_ui(
        ui,
        self.num_vertices(),
        self.num_faces(),
        self.num_edges(),
        &mut shade_style,
        &mut color,
        &mut transparency,
        &mut show_edges,
        &mut edge_width,
        &mut edge_color,
        &mut backface_policy,
    ) {
        self.shade_style = match shade_style {
            0 => ShadeStyle::Smooth,
            1 => ShadeStyle::Flat,
            _ => ShadeStyle::TriFlat,
        };
        self.surface_color = Vec3::new(color[0], color[1], color[2]);
        self.transparency = transparency;
        self.show_edges = show_edges;
        self.edge_width = edge_width;
        self.edge_color = Vec3::new(edge_color[0], edge_color[1], edge_color[2]);
        self.backface_policy = match backface_policy {
            0 => BackfacePolicy::Identical,
            1 => BackfacePolicy::Different,
            2 => BackfacePolicy::Custom,
            _ => BackfacePolicy::Cull,
        };
    }
}
```

**Step 3: Wire up in App**

In `app.rs`, in the structure tree callback:

```rust
if type_name == "SurfaceMesh" {
    if let Some(mesh) = s.as_any_mut().downcast_mut::<SurfaceMesh>() {
        mesh.build_egui_ui(ui);
    }
}
```

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/structure_ui.rs crates/polyscope-structures/src/surface_mesh/mod.rs crates/polyscope/src/app.rs
git commit -m "feat(ui): add SurfaceMesh UI controls

- Shade style dropdown (smooth/flat/tri-flat)
- Color, opacity sliders
- Wireframe toggle with width/color
- Backface policy dropdown"
```

---

## Task 7: Vertex Scalar Quantity

Implement scalar quantities on mesh vertices.

**Files:**
- Create: `crates/polyscope-structures/src/surface_mesh/quantities.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_vertex_scalar_quantity() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = SurfaceMesh::new("test", vertices, faces);

    mesh.add_vertex_scalar_quantity("height", vec![0.0, 0.5, 1.0]);

    let q = mesh.get_quantity("height").expect("quantity not found");
    assert_eq!(q.data_size(), 3);
}
```

**Step 2: Implement vertex scalar quantity**

Create `quantities.rs` following the PointCloud pattern:

```rust
//! Surface mesh quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind, VertexQuantity, FaceQuantity};
use polyscope_render::ColorMap;

/// A vertex scalar quantity on a surface mesh.
pub struct MeshVertexScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl MeshVertexScalarQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            colormap_name: "viridis".to_string(),
            range_min: min,
            range_max: max,
        }
    }

    pub fn values(&self) -> &[f32] { &self.values }
    pub fn colormap_name(&self) -> &str { &self.colormap_name }
    pub fn set_colormap(&mut self, name: impl Into<String>) { self.colormap_name = name.into(); }
    pub fn range_min(&self) -> f32 { self.range_min }
    pub fn range_max(&self) -> f32 { self.range_max }
    pub fn set_range(&mut self, min: f32, max: f32) { self.range_min = min; self.range_max = max; }

    pub fn compute_colors(&self, colormap: &ColorMap) -> Vec<Vec3> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        self.values
            .iter()
            .map(|&v| {
                let t = (v - self.range_min) / range;
                colormap.sample(t)
            })
            .collect()
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let colormaps = ["viridis", "blues", "reds", "coolwarm", "rainbow"];
        polyscope_ui::build_scalar_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.colormap_name,
            &mut self.range_min,
            &mut self.range_max,
            &colormaps,
        )
    }
}

impl Quantity for MeshVertexScalarQuantity {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Scalar }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.values.len() }
}

impl VertexQuantity for MeshVertexScalarQuantity {}
```

Add convenience method to SurfaceMesh:

```rust
pub fn add_vertex_scalar_quantity(&mut self, name: impl Into<String>, values: Vec<f32>) -> &mut Self {
    let quantity = MeshVertexScalarQuantity::new(name, self.name.clone(), values);
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/quantities.rs crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(surface_mesh): add MeshVertexScalarQuantity

- Scalar values per vertex with colormap
- Range auto-detection and manual setting
- egui UI integration
- Convenience add_vertex_scalar_quantity method"
```

---

## Task 8: Face Scalar Quantity

Implement scalar quantities on mesh faces.

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/quantities.rs`

**Step 1: Add FaceScalarQuantity**

```rust
/// A face scalar quantity on a surface mesh.
pub struct MeshFaceScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl MeshFaceScalarQuantity {
    // Similar to vertex scalar, but:
    // - `compute_vertex_colors()` expands face values to all vertices of each face
    // - Requires face_to_tri mapping and face data

    pub fn compute_vertex_colors(
        &self,
        faces: &[Vec<u32>],
        num_vertices: usize,
        colormap: &ColorMap,
    ) -> Vec<Vec3> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        // Accumulate colors for each vertex (last write wins, or average)
        let mut colors = vec![Vec3::ZERO; num_vertices];

        for (face_idx, face) in faces.iter().enumerate() {
            let t = (self.values[face_idx] - self.range_min) / range;
            let color = colormap.sample(t);
            for &vi in face {
                colors[vi as usize] = color;
            }
        }

        colors
    }
}

impl FaceQuantity for MeshFaceScalarQuantity {}
```

**Step 2: Add convenience method**

```rust
pub fn add_face_scalar_quantity(&mut self, name: impl Into<String>, values: Vec<f32>) -> &mut Self {
    let quantity = MeshFaceScalarQuantity::new(name, self.name.clone(), values);
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/quantities.rs crates/polyscope-structures/src/surface_mesh/mod.rs
git commit -m "feat(surface_mesh): add MeshFaceScalarQuantity

- Scalar values per face with colormap
- Expands to vertex colors for rendering"
```

---

## Task 9: Color Quantities

Implement vertex and face color quantities.

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/quantities.rs`

**Step 1: Add color quantities**

```rust
/// Vertex color quantity.
pub struct MeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

/// Face color quantity.
pub struct MeshFaceColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/quantities.rs
git commit -m "feat(surface_mesh): add color quantities (vertex and face)"
```

---

## Task 10: Vector Quantities

Implement vertex and face vector quantities using the existing VectorRenderData.

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/quantities.rs`

**Step 1: Add vector quantities**

```rust
use polyscope_render::{VectorRenderData, VectorUniforms};

/// Vertex vector quantity.
pub struct MeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
    render_data: Option<VectorRenderData>,
}

/// Face vector quantity (vectors at face centroids).
pub struct MeshFaceVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
    render_data: Option<VectorRenderData>,
}

impl MeshFaceVectorQuantity {
    /// Computes face centroids as base positions for vectors.
    pub fn compute_base_positions(vertices: &[Vec3], faces: &[Vec<u32>]) -> Vec<Vec3> {
        faces.iter().map(|face| {
            let sum: Vec3 = face.iter().map(|&i| vertices[i as usize]).sum();
            sum / face.len() as f32
        }).collect()
    }
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/quantities.rs
git commit -m "feat(surface_mesh): add vector quantities (vertex and face)

- Reuses VectorRenderData for arrow rendering
- Face vectors positioned at centroids"
```

---

## Task 11: Mesh Registration API

Add convenience function to register surface meshes.

**Files:**
- Modify: `crates/polyscope/src/lib.rs`

**Step 1: Add registration function**

```rust
pub use polyscope_structures::SurfaceMesh;

/// Registers a surface mesh structure.
pub fn register_surface_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    faces: Vec<Vec<u32>>,
) -> &'static mut SurfaceMesh {
    let name = name.into();
    let mesh = SurfaceMesh::new(name.clone(), vertices, faces);

    with_context_mut(|ctx| {
        ctx.registry.add(Box::new(mesh));
    });

    // Return mutable reference
    with_context_mut(|ctx| {
        let structure = ctx.registry.get_mut("SurfaceMesh", &name).unwrap();
        let ptr = structure.as_any_mut().downcast_mut::<SurfaceMesh>().unwrap() as *mut _;
        unsafe { &mut *ptr }
    })
}
```

**Step 2: Add integration test**

Create `tests/surface_mesh_test.rs`:

```rust
use glam::Vec3;

#[test]
fn test_register_surface_mesh() {
    polyscope::init();

    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];

    let mesh = polyscope::register_surface_mesh("test_quad", vertices, faces);

    assert_eq!(mesh.num_vertices(), 4);
    assert_eq!(mesh.num_faces(), 1);
    assert_eq!(mesh.triangulation().len(), 2); // Quad -> 2 triangles
}
```

**Step 3: Commit**

```bash
git add crates/polyscope/src/lib.rs tests/surface_mesh_test.rs
git commit -m "feat: add register_surface_mesh API function

- Top-level convenience for registering meshes
- Returns mutable reference for chaining quantity adds
- Integration test for quad mesh registration"
```

---

## Task 12: Picking for Mesh Elements

Extend picking to support face/vertex/edge selection.

**Files:**
- Modify: `crates/polyscope-render/src/pick.rs`
- Create: `crates/polyscope-render/src/shaders/surface_mesh_pick.wgsl`

**Step 1: Extend PickResult**

```rust
/// Element type for pick results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickElementType {
    #[default]
    None,
    Vertex,
    Face,
    Edge,
}

// Add to PickResult:
pub element_type: PickElementType,
```

**Step 2: Create pick shader**

```wgsl
// surface_mesh_pick.wgsl
// Renders each face with a unique color encoding face index

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Encode face index as color
    let face_id = in.face_id;
    let r = f32((face_id >> 16u) & 0xFFu) / 255.0;
    let g = f32((face_id >> 8u) & 0xFFu) / 255.0;
    let b = f32(face_id & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, 1.0);
}
```

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/pick.rs crates/polyscope-render/src/shaders/surface_mesh_pick.wgsl
git commit -m "feat(pick): extend picking for mesh elements

- PickElementType enum (vertex/face/edge)
- Pick shader encodes face index as color"
```

---

## Task 13: Example Application

Create an example demonstrating SurfaceMesh features.

**Files:**
- Create: `examples/surface_mesh_demo.rs`

**Step 1: Create example**

```rust
//! Surface mesh demonstration.

use glam::Vec3;

fn main() {
    env_logger::init();
    polyscope::init();

    // Create a simple box mesh
    let vertices = vec![
        // Front face
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        // Back face
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
    ];

    let faces = vec![
        vec![0, 1, 2, 3], // Front
        vec![5, 4, 7, 6], // Back
        vec![4, 0, 3, 7], // Left
        vec![1, 5, 6, 2], // Right
        vec![3, 2, 6, 7], // Top
        vec![4, 5, 1, 0], // Bottom
    ];

    let mesh = polyscope::register_surface_mesh("box", vertices, faces);

    // Add a face scalar quantity (face index)
    let face_scalars: Vec<f32> = (0..6).map(|i| i as f32).collect();
    mesh.add_face_scalar_quantity("face_id", face_scalars);

    // Add vertex scalar quantity (height)
    let vertex_scalars: Vec<f32> = mesh.vertices().iter().map(|v| v.y).collect();
    mesh.add_vertex_scalar_quantity("height", vertex_scalars);

    // Enable wireframe
    mesh.set_show_edges(true);
    mesh.set_edge_width(2.0);

    polyscope::show();
}
```

**Step 2: Test example runs**

Run: `cargo run --example surface_mesh_demo`
Expected: Window opens showing a colored box with wireframe

**Step 3: Commit**

```bash
git add examples/surface_mesh_demo.rs
git commit -m "docs: add surface_mesh_demo example

- Box mesh with quad faces
- Face and vertex scalar quantities
- Wireframe enabled"
```

---

## Task 14: Final Integration Test

Run full test suite and fix any issues.

**Step 1: Run all tests**

```bash
cargo test --workspace
```

**Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

**Step 3: Format code**

```bash
cargo fmt --all
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

This plan implements SurfaceMesh with:

1. **Core data structure** - Polygon storage, triangulation, normals, edges
2. **GPU rendering** - Buffers, shaders, pipeline integration
3. **Shading modes** - Smooth, flat, tri-flat
4. **Wireframe** - Barycentric coordinate rendering
5. **Backface handling** - Policy-based coloring/culling
6. **Quantities** - Vertex/face scalar, color, vector
7. **UI** - Full control panel with egui
8. **Picking** - Face selection support
9. **API** - Convenient `register_surface_mesh` function

Total: 14 tasks, each independently testable and commitable.
