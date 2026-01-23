//! Volume mesh structure for tetrahedral and hexahedral meshes.

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
    cells: Vec<[u32; 8]>,  // 8 indices per cell, unused slots hold u32::MAX

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
            .map(|t| [t[0], t[1], t[2], t[3], u32::MAX, u32::MAX, u32::MAX, u32::MAX])
            .collect();
        Self::new(name, vertices, cells)
    }

    /// Creates a hexahedral mesh.
    pub fn new_hex_mesh(name: impl Into<String>, vertices: Vec<Vec3>, hexes: Vec<[u32; 8]>) -> Self {
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

    /// Generates triangulated exterior faces for rendering.
    fn generate_render_geometry(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        // For simplicity, render all faces (not detecting interior faces)
        // In a full implementation, we'd detect shared faces between cells
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Tetrahedron - 4 triangular faces
                // Face 0: 0,2,1
                // Face 1: 0,1,3
                // Face 2: 0,3,2
                // Face 3: 1,2,3
                let tet_faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
                for [a, b, c] in tet_faces {
                    let base_idx = positions.len() as u32;
                    positions.push(self.vertices[cell[a] as usize]);
                    positions.push(self.vertices[cell[b] as usize]);
                    positions.push(self.vertices[cell[c] as usize]);
                    faces.push([base_idx, base_idx + 1, base_idx + 2]);
                }
            } else {
                // Hexahedron - 6 quadrilateral faces (2 triangles each)
                // Using VTK ordering (6/7 may be swapped from standard)
                let hex_faces = [
                    // Bottom face (z=0): 0,1,2,3
                    [[2, 1, 0], [2, 0, 3]],
                    // Front face (y=0): 0,1,5,4
                    [[4, 0, 1], [4, 1, 5]],
                    // Right face (x=1): 1,2,6,5
                    [[5, 1, 2], [5, 2, 6]],
                    // Back face (y=1): 3,7,6,2
                    [[6, 2, 3], [6, 3, 7]],
                    // Left face (x=0): 0,4,7,3
                    [[7, 3, 0], [7, 0, 4]],
                    // Top face (z=1): 4,5,6,7
                    [[7, 4, 5], [7, 5, 6]],
                ];
                for quad in hex_faces {
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
                if ui.add(egui::DragValue::new(&mut width).speed(0.01).range(0.01..=5.0)).changed() {
                    self.set_edge_width(width);
                }
            }
        });
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
