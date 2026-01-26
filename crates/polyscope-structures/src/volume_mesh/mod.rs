//! Volume mesh structure for tetrahedral and hexahedral meshes.

mod scalar_quantity;
mod color_quantity;
mod vector_quantity;
pub use scalar_quantity::*;
pub use color_quantity::*;
pub use vector_quantity::*;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{MeshUniforms, SurfaceMeshRenderData};

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

    /// Returns the render data if available.
    pub fn render_data(&self) -> Option<&SurfaceMeshRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU buffers.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
        if let Some(ref rd) = self.render_data {
            let uniforms = MeshUniforms {
                shade_style: 0, // smooth
                show_edges: if self.edge_width > 0.0 { 1 } else { 0 },
                edge_width: self.edge_width,
                transparency: 0.0,
                surface_color: [self.color.x, self.color.y, self.color.z, 1.0],
                edge_color: [self.edge_color.x, self.edge_color.y, self.edge_color.z, 1.0],
                backface_policy: 0,
                ..Default::default()
            };
            rd.update_uniforms(queue, &uniforms);
        }
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
}
