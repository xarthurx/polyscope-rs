//! Volume mesh structure for tetrahedral and hexahedral meshes.
//!
//! # Overview
//!
//! `VolumeMesh` supports both tetrahedral (4 vertices) and hexahedral (8 vertices)
//! cells. Mixed meshes are supported by using 8-index cells where unused indices
//! are set to `u32::MAX`.
//!
//! # Interior/Exterior Faces
//!
//! Only exterior faces (not shared between cells) are rendered. This is determined
//! by hashing sorted face vertex indices and counting occurrences.
//!
//! # Quantities
//!
//! Supported quantities:
//! - `VolumeMeshVertexScalarQuantity` - scalar per vertex
//! - `VolumeMeshCellScalarQuantity` - scalar per cell
//! - `VolumeMeshVertexColorQuantity` - RGB color per vertex
//! - `VolumeMeshCellColorQuantity` - RGB color per cell
//! - `VolumeMeshVertexVectorQuantity` - vector per vertex
//! - `VolumeMeshCellVectorQuantity` - vector per cell
//!
//! # Example
//!
//! ```rust,ignore
//! use glam::Vec3;
//! use polyscope_structures::VolumeMesh;
//!
//! // Create a single tetrahedron
//! let vertices = vec![
//!     Vec3::new(0.0, 0.0, 0.0),
//!     Vec3::new(1.0, 0.0, 0.0),
//!     Vec3::new(0.5, 1.0, 0.0),
//!     Vec3::new(0.5, 0.5, 1.0),
//! ];
//! let tets = vec![[0, 1, 2, 3]];
//! let mut mesh = VolumeMesh::new_tet_mesh("my_tet", vertices, tets);
//!
//! // Add a scalar quantity
//! mesh.add_vertex_scalar_quantity("temperature", vec![0.0, 0.5, 1.0, 0.25]);
//! ```

mod scalar_quantity;
mod color_quantity;
mod vector_quantity;
pub mod slice_geometry;

pub use scalar_quantity::*;
pub use color_quantity::*;
pub use vector_quantity::*;
pub use slice_geometry::{slice_tet, slice_hex, CellSliceResult};

// Re-export SliceMeshData from this module

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{MeshUniforms, SliceMeshRenderData, SurfaceMeshRenderData};

/// Cell type for volume meshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeCellType {
    /// Tetrahedron (4 vertices)
    Tet,
    /// Hexahedron (8 vertices)
    Hex,
}

/// A volume mesh structure (tetrahedral or hexahedral).
///
/// Cells are stored as arrays of 8 vertex indices. For tetrahedra,
/// only the first 4 indices are used (indices 4-7 are set to u32::MAX).
pub struct VolumeMesh {
    name: String,

    // Geometry
    vertices: Vec<Vec3>,
    cells: Vec<[u32; 8]>, // 8 indices per cell, unused slots hold u32::MAX

    // Common structure fields
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Visualization parameters
    color: Vec3,
    interior_color: Vec3,
    edge_color: Vec3,
    edge_width: f32,

    // GPU resources (renders exterior faces)
    render_data: Option<SurfaceMeshRenderData>,

    // Slice mesh GPU resources (renders cross-section caps)
    slice_render_data: Option<SliceMeshRenderData>,
    /// Cached slice plane parameters for invalidation (origin, normal)
    slice_plane_cache: Option<(Vec3, Vec3)>,
    /// Cached cell culling plane parameters (origin, normal) for each enabled plane.
    /// When Some, indicates render_data shows culled geometry.
    culling_plane_cache: Option<Vec<(Vec3, Vec3)>>,
}

impl VolumeMesh {
    /// Creates a new volume mesh from vertices and cell indices.
    ///
    /// # Arguments
    /// * `name` - The name of the mesh
    /// * `vertices` - Vertex positions
    /// * `cells` - Cell indices, 8 per cell (unused indices should be u32::MAX)
    pub fn new(name: impl Into<String>, vertices: Vec<Vec3>, cells: Vec<[u32; 8]>) -> Self {
        let color = Vec3::new(0.25, 0.50, 0.75);
        // Interior color is a desaturated version
        let interior_color = Vec3::new(0.45, 0.50, 0.55);

        Self {
            name: name.into(),
            vertices,
            cells,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            color,
            interior_color,
            edge_color: Vec3::ZERO,
            edge_width: 0.0,
            render_data: None,
            slice_render_data: None,
            slice_plane_cache: None,
            culling_plane_cache: None,
        }
    }

    /// Creates a tetrahedral mesh.
    pub fn new_tet_mesh(name: impl Into<String>, vertices: Vec<Vec3>, tets: Vec<[u32; 4]>) -> Self {
        // Convert tets to 8-index cells
        let cells: Vec<[u32; 8]> = tets
            .into_iter()
            .map(|t| {
                [
                    t[0],
                    t[1],
                    t[2],
                    t[3],
                    u32::MAX,
                    u32::MAX,
                    u32::MAX,
                    u32::MAX,
                ]
            })
            .collect();
        Self::new(name, vertices, cells)
    }

    /// Creates a hexahedral mesh.
    pub fn new_hex_mesh(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        hexes: Vec<[u32; 8]>,
    ) -> Self {
        Self::new(name, vertices, hexes)
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of cells.
    pub fn num_cells(&self) -> usize {
        self.cells.len()
    }

    /// Returns the cell type of the given cell.
    pub fn cell_type(&self, cell_idx: usize) -> VolumeCellType {
        if self.cells[cell_idx][4] == u32::MAX {
            VolumeCellType::Tet
        } else {
            VolumeCellType::Hex
        }
    }

    /// Returns the vertices.
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Returns the cells.
    pub fn cells(&self) -> &[[u32; 8]] {
        &self.cells
    }

    /// Gets the base color.
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the base color.
    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        self
    }

    /// Gets the interior color.
    pub fn interior_color(&self) -> Vec3 {
        self.interior_color
    }

    /// Sets the interior color.
    pub fn set_interior_color(&mut self, color: Vec3) -> &mut Self {
        self.interior_color = color;
        self
    }

    /// Gets the edge color.
    pub fn edge_color(&self) -> Vec3 {
        self.edge_color
    }

    /// Sets the edge color.
    pub fn set_edge_color(&mut self, color: Vec3) -> &mut Self {
        self.edge_color = color;
        self
    }

    /// Gets the edge width.
    pub fn edge_width(&self) -> f32 {
        self.edge_width
    }

    /// Sets the edge width.
    pub fn set_edge_width(&mut self, width: f32) -> &mut Self {
        self.edge_width = width;
        self
    }

    /// Decomposes all cells into tetrahedra.
    /// Tets pass through unchanged, hexes are decomposed into 5 tets.
    pub fn decompose_to_tets(&self) -> Vec<[u32; 4]> {
        let mut tets = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Already a tet
                tets.push([cell[0], cell[1], cell[2], cell[3]]);
            } else {
                // Hex - decompose using diagonal pattern (5 tets)
                for tet_local in HEX_TO_TET_PATTERN.iter() {
                    let tet = [
                        cell[tet_local[0]],
                        cell[tet_local[1]],
                        cell[tet_local[2]],
                        cell[tet_local[3]],
                    ];
                    tets.push(tet);
                }
            }
        }

        tets
    }

    /// Returns the number of tetrahedra (including decomposed hexes).
    pub fn num_tets(&self) -> usize {
        self.decompose_to_tets().len()
    }

    /// Computes face counts for interior/exterior detection.
    fn compute_face_counts(&self) -> HashMap<[u32; 4], usize> {
        let mut face_counts: HashMap<[u32; 4], usize> = HashMap::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            } else {
                // Hexahedron - each quad face uses same 4 vertices
                for quad in HEX_FACE_STENCIL {
                    // Get the 4 unique vertices of this quad face
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]]; // The fourth vertex
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            }
        }

        face_counts
    }

    /// Computes the centroid of a cell.
    fn cell_centroid(&self, cell: &[u32; 8]) -> Vec3 {
        if cell[4] == u32::MAX {
            // Tetrahedron: average of 4 vertices
            let sum = self.vertices[cell[0] as usize]
                + self.vertices[cell[1] as usize]
                + self.vertices[cell[2] as usize]
                + self.vertices[cell[3] as usize];
            sum / 4.0
        } else {
            // Hexahedron: average of 8 vertices
            let sum = (0..8)
                .map(|i| self.vertices[cell[i] as usize])
                .fold(Vec3::ZERO, |a, b| a + b);
            sum / 8.0
        }
    }

    /// Tests if a cell should be visible based on slice planes.
    /// Returns true if the cell's centroid is on the "kept" side of all planes.
    fn is_cell_visible(&self, cell: &[u32; 8], planes: &[(Vec3, Vec3)]) -> bool {
        if planes.is_empty() {
            return true;
        }
        let centroid = self.cell_centroid(cell);
        let centroid_world = (self.transform * centroid.extend(1.0)).truncate();
        for (plane_origin, plane_normal) in planes {
            let signed_dist = (centroid_world - *plane_origin).dot(*plane_normal);
            // Keep cells on the positive side of the plane (same side as normal points)
            if signed_dist < 0.0 {
                return false;
            }
        }
        true
    }

    /// Computes face counts for interior/exterior detection, only for visible cells.
    fn compute_face_counts_with_culling(
        &self,
        planes: &[(Vec3, Vec3)],
    ) -> HashMap<[u32; 4], usize> {
        let mut face_counts: HashMap<[u32; 4], usize> = HashMap::new();

        for cell in &self.cells {
            // Skip cells culled by slice planes
            if !self.is_cell_visible(cell, planes) {
                continue;
            }

            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            }
        }

        face_counts
    }

    /// Generates triangulated exterior faces for rendering.
    fn generate_render_geometry(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let face_counts = self.compute_face_counts();
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    if face_counts[&key] == 1 {
                        // Exterior face
                        let base_idx = positions.len() as u32;
                        positions.push(self.vertices[cell[a] as usize]);
                        positions.push(self.vertices[cell[b] as usize]);
                        positions.push(self.vertices[cell[c] as usize]);
                        faces.push([base_idx, base_idx + 1, base_idx + 2]);
                    }
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    if face_counts[&key] == 1 {
                        // Exterior face - emit both triangles
                        for [a, b, c] in quad {
                            let base_idx = positions.len() as u32;
                            positions.push(self.vertices[cell[a] as usize]);
                            positions.push(self.vertices[cell[b] as usize]);
                            positions.push(self.vertices[cell[c] as usize]);
                            faces.push([base_idx, base_idx + 1, base_idx + 2]);
                        }
                    }
                }
            }
        }

        (positions, faces)
    }

    /// Generates triangulated exterior faces with cell culling based on slice planes.
    /// Only cells whose centroid is on the positive side of all planes are rendered.
    fn generate_render_geometry_with_culling(
        &self,
        planes: &[(Vec3, Vec3)],
    ) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        // Compute face counts only for visible cells
        let face_counts = self.compute_face_counts_with_culling(planes);
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for cell in &self.cells {
            // Skip cells culled by slice planes
            if !self.is_cell_visible(cell, planes) {
                continue;
            }

            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    // Render face if it's exterior among visible cells
                    if face_counts.get(&key) == Some(&1) {
                        let base_idx = positions.len() as u32;
                        positions.push(self.vertices[cell[a] as usize]);
                        positions.push(self.vertices[cell[b] as usize]);
                        positions.push(self.vertices[cell[c] as usize]);
                        faces.push([base_idx, base_idx + 1, base_idx + 2]);
                    }
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    if face_counts.get(&key) == Some(&1) {
                        for [a, b, c] in quad {
                            let base_idx = positions.len() as u32;
                            positions.push(self.vertices[cell[a] as usize]);
                            positions.push(self.vertices[cell[b] as usize]);
                            positions.push(self.vertices[cell[c] as usize]);
                            faces.push([base_idx, base_idx + 1, base_idx + 2]);
                        }
                    }
                }
            }
        }

        (positions, faces)
    }

    /// Generates render geometry including any enabled quantity data.
    pub fn generate_render_geometry_with_quantities(&self) -> VolumeMeshRenderGeometry {
        let face_counts = self.compute_face_counts();
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        let mut vertex_indices = Vec::new(); // Track original vertex indices
        let mut cell_indices = Vec::new();   // Track which cell each face belongs to

        // First pass: generate geometry and track indices
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    if face_counts[&key] == 1 {
                        let base_idx = positions.len() as u32;
                        positions.push(self.vertices[cell[a] as usize]);
                        positions.push(self.vertices[cell[b] as usize]);
                        positions.push(self.vertices[cell[c] as usize]);
                        vertex_indices.push(cell[a] as usize);
                        vertex_indices.push(cell[b] as usize);
                        vertex_indices.push(cell[c] as usize);
                        cell_indices.push(cell_idx);
                        cell_indices.push(cell_idx);
                        cell_indices.push(cell_idx);
                        faces.push([base_idx, base_idx + 1, base_idx + 2]);
                    }
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    if face_counts[&key] == 1 {
                        for [a, b, c] in quad {
                            let base_idx = positions.len() as u32;
                            positions.push(self.vertices[cell[a] as usize]);
                            positions.push(self.vertices[cell[b] as usize]);
                            positions.push(self.vertices[cell[c] as usize]);
                            vertex_indices.push(cell[a] as usize);
                            vertex_indices.push(cell[b] as usize);
                            vertex_indices.push(cell[c] as usize);
                            cell_indices.push(cell_idx);
                            cell_indices.push(cell_idx);
                            cell_indices.push(cell_idx);
                            faces.push([base_idx, base_idx + 1, base_idx + 2]);
                        }
                    }
                }
            }
        }

        // Compute normals
        let mut normals = vec![Vec3::ZERO; positions.len()];
        for [a, b, c] in &faces {
            let p0 = positions[*a as usize];
            let p1 = positions[*b as usize];
            let p2 = positions[*c as usize];
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            normals[*a as usize] = normal;
            normals[*b as usize] = normal;
            normals[*c as usize] = normal;
        }

        // Find enabled scalar quantity
        let mut vertex_values = None;
        let mut vertex_colors = None;

        for q in &self.quantities {
            if q.is_enabled() {
                if let Some(scalar) = q.as_any().downcast_ref::<VolumeMeshVertexScalarQuantity>() {
                    let values: Vec<f32> = vertex_indices.iter()
                        .map(|&idx| scalar.values().get(idx).copied().unwrap_or(0.0))
                        .collect();
                    vertex_values = Some(values);
                    break;
                }
                if let Some(color) = q.as_any().downcast_ref::<VolumeMeshVertexColorQuantity>() {
                    let colors: Vec<Vec3> = vertex_indices.iter()
                        .map(|&idx| color.colors().get(idx).copied().unwrap_or(Vec3::ONE))
                        .collect();
                    vertex_colors = Some(colors);
                    break;
                }
                if let Some(scalar) = q.as_any().downcast_ref::<VolumeMeshCellScalarQuantity>() {
                    let values: Vec<f32> = cell_indices.iter()
                        .map(|&idx| scalar.values().get(idx).copied().unwrap_or(0.0))
                        .collect();
                    vertex_values = Some(values);
                    break;
                }
                if let Some(color) = q.as_any().downcast_ref::<VolumeMeshCellColorQuantity>() {
                    let colors: Vec<Vec3> = cell_indices.iter()
                        .map(|&idx| color.colors().get(idx).copied().unwrap_or(Vec3::ONE))
                        .collect();
                    vertex_colors = Some(colors);
                    break;
                }
            }
        }

        VolumeMeshRenderGeometry {
            positions,
            faces,
            normals,
            vertex_values,
            vertex_colors,
        }
    }

    /// Initializes GPU render data.
    pub fn init_render_data(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        let (positions, triangles) = self.generate_render_geometry();

        if triangles.is_empty() {
            return;
        }

        // Compute per-vertex normals (for flat shading, each triangle vertex gets the face normal)
        let mut normals = vec![Vec3::ZERO; positions.len()];
        for [a, b, c] in &triangles {
            let p0 = positions[*a as usize];
            let p1 = positions[*b as usize];
            let p2 = positions[*c as usize];
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            normals[*a as usize] = normal;
            normals[*b as usize] = normal;
            normals[*c as usize] = normal;
        }

        // For volume meshes, all edges are "real" (not internal triangulation edges)
        // edge_is_real is per-triangle-vertex, 3 values per triangle
        let edge_is_real: Vec<Vec3> = vec![Vec3::ONE; triangles.len() * 3];

        let render_data = SurfaceMeshRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &positions,
            &triangles,
            &normals,
            &edge_is_real,
        );

        self.render_data = Some(render_data);
    }

    /// Reinitializes render data with cell culling based on slice planes.
    /// Cells whose centroid is on the negative side of any plane are hidden.
    /// Uses caching to avoid regenerating geometry every frame.
    pub fn update_render_data_with_culling(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        planes: &[(Vec3, Vec3)],
    ) {
        // Check if cache is still valid (plane hasn't moved)
        let cache_valid = self.culling_plane_cache.as_ref().map_or(false, |cache| {
            if cache.len() != planes.len() {
                return false;
            }
            cache.iter().zip(planes.iter()).all(|((o, n), (po, pn))| {
                (*o - *po).length_squared() < 1e-10 && (*n - *pn).length_squared() < 1e-10
            })
        });

        if cache_valid && self.render_data.is_some() {
            // Cached geometry is still valid
            return;
        }

        let (positions, triangles) = self.generate_render_geometry_with_culling(planes);

        if triangles.is_empty() {
            self.render_data = None;
            self.culling_plane_cache = Some(planes.to_vec());
            return;
        }

        // Compute per-vertex normals
        let mut normals = vec![Vec3::ZERO; positions.len()];
        for [a, b, c] in &triangles {
            let p0 = positions[*a as usize];
            let p1 = positions[*b as usize];
            let p2 = positions[*c as usize];
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            normals[*a as usize] = normal;
            normals[*b as usize] = normal;
            normals[*c as usize] = normal;
        }

        let edge_is_real: Vec<Vec3> = vec![Vec3::ONE; triangles.len() * 3];

        let render_data = SurfaceMeshRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &positions,
            &triangles,
            &normals,
            &edge_is_real,
        );

        self.render_data = Some(render_data);
        self.culling_plane_cache = Some(planes.to_vec());
    }

    /// Resets render data to show all cells (no culling).
    pub fn reset_render_data(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        self.culling_plane_cache = None;
        self.init_render_data(device, bind_group_layout, camera_buffer);
    }

    /// Returns true if the mesh is currently showing culled geometry.
    pub fn is_culled(&self) -> bool {
        self.culling_plane_cache.is_some()
    }

    /// Returns the render data if available.
    pub fn render_data(&self) -> Option<&SurfaceMeshRenderData> {
        self.render_data.as_ref()
    }

    /// Generates triangulated exterior faces for picking.
    /// Uses current slice plane culling if provided.
    pub fn pick_triangles(&self, planes: &[(Vec3, Vec3)]) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        if planes.is_empty() {
            self.generate_render_geometry()
        } else {
            self.generate_render_geometry_with_culling(planes)
        }
    }

    /// Updates GPU buffers.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
        if let Some(ref rd) = self.render_data {
            // Convert transform to array format for GPU
            let model_matrix = self.transform.to_cols_array_2d();

            let uniforms = MeshUniforms {
                model_matrix,
                shade_style: 0, // smooth
                show_edges: if self.edge_width > 0.0 { 1 } else { 0 },
                edge_width: self.edge_width,
                transparency: 0.0,
                surface_color: [self.color.x, self.color.y, self.color.z, 1.0],
                edge_color: [self.edge_color.x, self.edge_color.y, self.edge_color.z, 1.0],
                backface_policy: 0,
                slice_planes_enabled: 0,
                ..Default::default()
            };
            rd.update_uniforms(queue, &uniforms);
        }
    }

    /// Updates or creates slice mesh render data for a given slice plane.
    ///
    /// Returns `true` if the slice intersects this volume mesh.
    pub fn update_slice_render_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        plane_origin: Vec3,
        plane_normal: Vec3,
    ) -> bool {
        // Check if cache is still valid
        let cache_valid = self.slice_plane_cache.map_or(false, |(o, n)| {
            (o - plane_origin).length_squared() < 1e-10 && (n - plane_normal).length_squared() < 1e-10
        });

        if cache_valid && self.slice_render_data.is_some() {
            return !self.slice_render_data.as_ref().unwrap().is_empty();
        }

        // Generate new slice geometry
        if let Some(slice_data) = self.generate_slice_geometry(plane_origin, plane_normal) {
            if let Some(ref mut rd) = self.slice_render_data {
                // Update existing render data
                rd.update(
                    device,
                    queue,
                    bind_group_layout,
                    camera_buffer,
                    &slice_data.vertices,
                    &slice_data.normals,
                    &slice_data.colors,
                );
            } else {
                // Create new render data
                self.slice_render_data = Some(SliceMeshRenderData::new(
                    device,
                    bind_group_layout,
                    camera_buffer,
                    &slice_data.vertices,
                    &slice_data.normals,
                    &slice_data.colors,
                ));
            }

            // Update uniforms with interior color
            if let Some(ref rd) = self.slice_render_data {
                rd.update_uniforms(queue, self.interior_color);
            }

            self.slice_plane_cache = Some((plane_origin, plane_normal));
            true
        } else {
            // No intersection
            self.slice_render_data = None;
            self.slice_plane_cache = None;
            false
        }
    }

    /// Returns the slice render data if available.
    pub fn slice_render_data(&self) -> Option<&SliceMeshRenderData> {
        self.slice_render_data.as_ref()
    }

    /// Clears the slice render data cache.
    pub fn clear_slice_render_data(&mut self) {
        self.slice_render_data = None;
        self.slice_plane_cache = None;
    }

    /// Builds the egui UI for this volume mesh.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        // Info
        let num_tets = self.cells.iter().filter(|c| c[4] == u32::MAX).count();
        let num_hexes = self.num_cells() - num_tets;
        ui.label(format!(
            "Vertices: {}  Cells: {} ({} tets, {} hexes)",
            self.num_vertices(),
            self.num_cells(),
            num_tets,
            num_hexes
        ));

        // Color
        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut color = [self.color.x, self.color.y, self.color.z];
            if ui.color_edit_button_rgb(&mut color).changed() {
                self.set_color(Vec3::new(color[0], color[1], color[2]));
            }
        });

        // Edge width
        ui.horizontal(|ui| {
            let mut show_edges = self.edge_width > 0.0;
            if ui.checkbox(&mut show_edges, "Edges").changed() {
                self.set_edge_width(if show_edges { 1.0 } else { 0.0 });
            }
            if show_edges {
                let mut width = self.edge_width;
                if ui
                    .add(
                        egui::DragValue::new(&mut width)
                            .speed(0.01)
                            .range(0.01..=5.0),
                    )
                    .changed()
                {
                    self.set_edge_width(width);
                }
            }
        });
    }

    /// Adds a vertex scalar quantity.
    pub fn add_vertex_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshVertexScalarQuantity::new(
            name.clone(),
            self.name.clone(),
            values,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshCellScalarQuantity::new(
            name.clone(),
            self.name.clone(),
            values,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex color quantity.
    pub fn add_vertex_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshVertexColorQuantity::new(
            name.clone(),
            self.name.clone(),
            colors,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a cell color quantity.
    pub fn add_cell_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshCellColorQuantity::new(
            name.clone(),
            self.name.clone(),
            colors,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex vector quantity.
    pub fn add_vertex_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshVertexVectorQuantity::new(
            name.clone(),
            self.name.clone(),
            vectors,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a cell vector quantity.
    pub fn add_cell_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshCellVectorQuantity::new(
            name.clone(),
            self.name.clone(),
            vectors,
        );
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Returns the active vertex color quantity, if any.
    fn active_vertex_color_quantity(&self) -> Option<&VolumeMeshVertexColorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() {
                if let Some(vcq) = q.as_any().downcast_ref::<VolumeMeshVertexColorQuantity>() {
                    return Some(vcq);
                }
            }
        }
        None
    }

    /// Generates mesh geometry for the cross-section created by a slice plane.
    ///
    /// This computes the intersection of all cells with the plane and triangulates
    /// the resulting polygons for rendering. If a vertex color quantity is enabled,
    /// colors are interpolated at slice points.
    ///
    /// # Arguments
    /// * `plane_origin` - A point on the slice plane
    /// * `plane_normal` - The plane normal (points toward kept geometry)
    ///
    /// # Returns
    /// `Some(SliceMeshData)` if the plane intersects the mesh, `None` otherwise.
    pub fn generate_slice_geometry(
        &self,
        plane_origin: Vec3,
        plane_normal: Vec3,
    ) -> Option<SliceMeshData> {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();

        // Get active vertex color quantity for interpolation (if any)
        let vertex_colors = self.active_vertex_color_quantity().map(|q| q.colors());

        for (cell_idx, cell) in self.cells.iter().enumerate() {
            let cell_type = self.cell_type(cell_idx);

            let slice = match cell_type {
                VolumeCellType::Tet => {
                    slice_tet(
                        self.vertices[cell[0] as usize],
                        self.vertices[cell[1] as usize],
                        self.vertices[cell[2] as usize],
                        self.vertices[cell[3] as usize],
                        plane_origin,
                        plane_normal,
                    )
                }
                VolumeCellType::Hex => {
                    let hex_verts: [Vec3; 8] =
                        std::array::from_fn(|i| self.vertices[cell[i] as usize]);
                    slice_hex(hex_verts, plane_origin, plane_normal)
                }
            };

            if slice.has_intersection() {
                // Compute interpolated colors for each slice vertex
                let slice_colors: Vec<Vec3> = if let Some(vc) = vertex_colors {
                    slice
                        .interpolation
                        .iter()
                        .map(|&(a, b, t)| {
                            // Map local cell indices to global vertex indices
                            let va_idx = cell[a as usize] as usize;
                            let vb_idx = cell[b as usize] as usize;
                            // Interpolate colors
                            vc[va_idx].lerp(vc[vb_idx], t)
                        })
                        .collect()
                } else {
                    vec![self.interior_color; slice.vertices.len()]
                };

                // Triangulate the polygon (fan from first vertex)
                for i in 1..slice.vertices.len() - 1 {
                    vertices.push(slice.vertices[0]);
                    vertices.push(slice.vertices[i]);
                    vertices.push(slice.vertices[i + 1]);

                    // Normal is the slice plane normal
                    normals.push(plane_normal);
                    normals.push(plane_normal);
                    normals.push(plane_normal);

                    // Interpolated colors (or interior_color if no quantity)
                    colors.push(slice_colors[0]);
                    colors.push(slice_colors[i]);
                    colors.push(slice_colors[i + 1]);
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

/// Data representing a slice mesh cross-section.
///
/// Contains triangulated geometry for rendering the cross-section
/// created by a slice plane intersecting a volume mesh.
#[derive(Debug, Clone)]
pub struct SliceMeshData {
    /// Vertex positions (3 per triangle)
    pub vertices: Vec<Vec3>,
    /// Vertex normals (3 per triangle, all pointing along plane normal)
    pub normals: Vec<Vec3>,
    /// Vertex colors (3 per triangle, from interior color or interpolated quantity)
    pub colors: Vec<Vec3>,
}

impl SliceMeshData {
    /// Returns the number of triangles in the slice mesh.
    pub fn num_triangles(&self) -> usize {
        self.vertices.len() / 3
    }

    /// Returns true if the slice mesh is empty.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

impl Structure for VolumeMesh {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn type_name(&self) -> &'static str {
        "VolumeMesh"
    }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        if self.vertices.is_empty() {
            return None;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for &v in &self.vertices {
            min = min.min(v);
            max = max.max(v);
        }

        Some((min, max))
    }

    fn length_scale(&self) -> f32 {
        self.bounding_box()
            .map(|(min, max)| (max - min).length())
            .unwrap_or(1.0)
    }

    fn transform(&self) -> Mat4 {
        self.transform
    }

    fn set_transform(&mut self, transform: Mat4) {
        self.transform = transform;
        self.culling_plane_cache = None;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        // Drawing is handled externally
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        // Picking not implemented
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is built via build_egui_ui
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // Pick UI not implemented
    }

    fn refresh(&mut self) {
        self.render_data = None;
        self.slice_render_data = None;
        self.slice_plane_cache = None;
        self.culling_plane_cache = None;
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for VolumeMesh {
    fn add_quantity(&mut self, quantity: Box<dyn Quantity>) {
        self.quantities.push(quantity);
    }

    fn get_quantity(&self, name: &str) -> Option<&dyn Quantity> {
        self.quantities
            .iter()
            .find(|q| q.name() == name)
            .map(|q| q.as_ref())
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

use std::collections::HashMap;

/// Generates a canonical (sorted) face key for hashing.
/// For triangular faces, the fourth element is u32::MAX.
fn canonical_face_key(v0: u32, v1: u32, v2: u32, v3: Option<u32>) -> [u32; 4] {
    let mut key = [v0, v1, v2, v3.unwrap_or(u32::MAX)];
    key.sort();
    key
}

/// Face stencil for tetrahedra: 4 triangular faces
const TET_FACE_STENCIL: [[usize; 3]; 4] = [
    [0, 2, 1],
    [0, 1, 3],
    [0, 3, 2],
    [1, 2, 3],
];

/// Face stencil for hexahedra: 6 quad faces (each as 2 triangles sharing diagonal)
const HEX_FACE_STENCIL: [[[usize; 3]; 2]; 6] = [
    [[2, 1, 0], [2, 0, 3]], // Bottom
    [[4, 0, 1], [4, 1, 5]], // Front
    [[5, 1, 2], [5, 2, 6]], // Right
    [[7, 3, 0], [7, 0, 4]], // Left
    [[6, 2, 3], [6, 3, 7]], // Back
    [[7, 4, 5], [7, 5, 6]], // Top
];

/// Render geometry data with optional quantity values.
pub struct VolumeMeshRenderGeometry {
    pub positions: Vec<Vec3>,
    pub faces: Vec<[u32; 3]>,
    pub normals: Vec<Vec3>,
    /// Per-vertex scalar values for color mapping (from enabled vertex scalar quantity).
    pub vertex_values: Option<Vec<f32>>,
    /// Per-vertex colors (from enabled vertex color quantity).
    pub vertex_colors: Option<Vec<Vec3>>,
}

/// Rotation map for hex vertices to place vertex 0 in canonical position.
#[allow(dead_code)]
const HEX_ROTATION_MAP: [[usize; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [1, 0, 4, 5, 2, 3, 7, 6],
    [2, 1, 5, 6, 3, 0, 4, 7],
    [3, 0, 1, 2, 7, 4, 5, 6],
    [4, 0, 3, 7, 5, 1, 2, 6],
    [5, 1, 0, 4, 6, 2, 3, 7],
    [6, 2, 1, 5, 7, 3, 0, 4],
    [7, 3, 2, 6, 4, 0, 1, 5],
];

/// Diagonal decomposition patterns (5 tets).
const HEX_TO_TET_PATTERN: [[usize; 4]; 5] = [
    [0, 1, 2, 5],
    [0, 2, 7, 5],
    [0, 2, 3, 7],
    [0, 5, 7, 4],
    [2, 7, 5, 6],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interior_face_detection() {
        // Two tets sharing a face
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),  // 0
            Vec3::new(1.0, 0.0, 0.0),  // 1
            Vec3::new(0.5, 1.0, 0.0),  // 2
            Vec3::new(0.5, 0.5, 1.0),  // 3 - apex of first tet
            Vec3::new(0.5, 0.5, -1.0), // 4 - apex of second tet
        ];
        // Two tets sharing face [0,1,2]
        let tets = vec![[0, 1, 2, 3], [0, 2, 1, 4]];
        let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

        // Should have 6 exterior faces (4 per tet - 1 shared = 3 per tet * 2 = 6)
        // Not 8 faces (4 per tet * 2 = 8)
        let (_, faces) = mesh.generate_render_geometry();
        assert_eq!(faces.len(), 6, "Should only render exterior faces");
    }

    #[test]
    fn test_single_tet_all_exterior() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

        let (_, faces) = mesh.generate_render_geometry();
        assert_eq!(faces.len(), 4, "Single tet should have 4 exterior faces");
    }

    #[test]
    fn test_single_hex_all_exterior() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
        let mesh = VolumeMesh::new_hex_mesh("test", vertices, hexes);

        let (_, faces) = mesh.generate_render_geometry();
        // 6 quad faces * 2 triangles each = 12 triangles
        assert_eq!(faces.len(), 12, "Single hex should have 12 triangles (6 quads)");
    }

    #[test]
    fn test_tet_mesh_creation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

        assert_eq!(mesh.num_vertices(), 4);
        assert_eq!(mesh.num_cells(), 1);
        assert_eq!(mesh.cell_type(0), VolumeCellType::Tet);
    }

    #[test]
    fn test_hex_mesh_creation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
        let mesh = VolumeMesh::new_hex_mesh("test", vertices, hexes);

        assert_eq!(mesh.num_vertices(), 8);
        assert_eq!(mesh.num_cells(), 1);
        assert_eq!(mesh.cell_type(0), VolumeCellType::Hex);
    }

    #[test]
    fn test_hex_to_tet_decomposition() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
        let mesh = VolumeMesh::new_hex_mesh("test", vertices, hexes);

        let tets = mesh.decompose_to_tets();
        // A hex is decomposed into 5 tets
        assert_eq!(tets.len(), 5);

        // Each tet should have 4 vertices
        for tet in &tets {
            assert!(tet[0] < 8);
            assert!(tet[1] < 8);
            assert!(tet[2] < 8);
            assert!(tet[3] < 8);
        }
    }

    #[test]
    fn test_tet_mesh_decomposition() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

        // Tet mesh should decompose to itself
        let decomposed = mesh.decompose_to_tets();
        assert_eq!(decomposed.len(), 1);
        assert_eq!(decomposed[0], [0, 1, 2, 3]);
    }

    #[test]
    fn test_quantity_aware_geometry_generation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        let mut mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

        // Add vertex scalar quantity
        mesh.add_vertex_scalar_quantity("temp", vec![0.0, 0.5, 1.0, 0.25]);

        // Get the quantity and enable it
        if let Some(q) = mesh.get_quantity_mut("temp") {
            q.set_enabled(true);
        }

        // Generate geometry should include scalar values for color mapping
        let render_data = mesh.generate_render_geometry_with_quantities();
        assert!(render_data.vertex_values.is_some());
        assert_eq!(render_data.vertex_values.as_ref().unwrap().len(),
                   render_data.positions.len());
    }
}
