//! Surface mesh structure.

mod quantities;
pub use quantities::*;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{MeshUniforms, SurfaceMeshRenderData};
use std::ops::Range;

/// Shading style for surface mesh rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadeStyle {
    /// Smooth shading using interpolated vertex normals.
    #[default]
    Smooth,
    /// Flat shading using face normals.
    Flat,
    /// Flat shading per triangle (after triangulation).
    TriFlat,
}

/// Policy for rendering backfaces of the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackfacePolicy {
    /// Backfaces rendered identically to front faces.
    #[default]
    Identical,
    /// Backfaces rendered with a different (darker) shade.
    Different,
    /// Backfaces rendered with a custom color.
    Custom,
    /// Backfaces are culled (not rendered).
    Cull,
}

/// A surface mesh structure (triangular or polygonal).
pub struct SurfaceMesh {
    // Core data
    name: String,
    vertices: Vec<Vec3>,
    faces: Vec<Vec<u32>>, // Variable-length polygons
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Computed data
    triangulation: Vec<[u32; 3]>,
    face_to_tri_range: Vec<Range<usize>>,
    vertex_normals: Vec<Vec3>,
    face_normals: Vec<Vec3>,
    corner_normals: Vec<Vec3>,
    edge_is_real: Vec<Vec3>,
    edges: Vec<(u32, u32)>,
    needs_recompute: bool,

    // Render options
    shade_style: ShadeStyle,
    edge_width: f32,
    edge_color: Vec3,
    show_edges: bool,
    backface_policy: BackfacePolicy,
    backface_color: Vec3,
    surface_color: Vec3,
    transparency: f32,

    // GPU resources
    render_data: Option<SurfaceMeshRenderData>,
}

impl SurfaceMesh {
    /// Creates a new surface mesh from vertices and polygon faces.
    ///
    /// Each face is a variable-length list of vertex indices forming a polygon.
    /// Triangles have 3 indices, quads have 4, etc.
    pub fn new(name: impl Into<String>, vertices: Vec<Vec3>, faces: Vec<Vec<u32>>) -> Self {
        let mut mesh = Self {
            name: name.into(),
            vertices,
            faces,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),

            // Computed data (will be filled by recompute)
            triangulation: Vec::new(),
            face_to_tri_range: Vec::new(),
            vertex_normals: Vec::new(),
            face_normals: Vec::new(),
            corner_normals: Vec::new(),
            edge_is_real: Vec::new(),
            edges: Vec::new(),
            needs_recompute: true,

            // Default render options
            shade_style: ShadeStyle::default(),
            edge_width: 1.0,
            edge_color: Vec3::ZERO,
            show_edges: false,
            backface_policy: BackfacePolicy::default(),
            backface_color: Vec3::new(0.3, 0.3, 0.3),
            surface_color: Vec3::new(0.5, 0.5, 0.8),
            transparency: 0.0, // 0.0 = fully opaque, 1.0 = fully transparent

            render_data: None,
        };
        mesh.recompute();
        mesh
    }

    /// Creates a new surface mesh from triangles (convenience method).
    ///
    /// This is a convenience method for creating a mesh from triangle data.
    pub fn from_triangles(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
    ) -> Self {
        let faces: Vec<Vec<u32>> = triangles.into_iter().map(|t| t.to_vec()).collect();
        Self::new(name, vertices, faces)
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of faces.
    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }

    /// Returns the number of triangles in the triangulation.
    pub fn num_triangles(&self) -> usize {
        self.triangulation.len()
    }

    /// Returns the number of edges.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Returns the vertices.
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Returns the faces (polygon indices).
    pub fn faces(&self) -> &[Vec<u32>] {
        &self.faces
    }

    /// Returns the triangulation.
    pub fn triangulation(&self) -> &[[u32; 3]] {
        &self.triangulation
    }

    /// Returns the mapping from face index to triangle range.
    pub fn face_to_tri_range(&self) -> &[Range<usize>] {
        &self.face_to_tri_range
    }

    /// Returns the vertex normals.
    pub fn vertex_normals(&self) -> &[Vec3] {
        &self.vertex_normals
    }

    /// Returns the face normals.
    pub fn face_normals(&self) -> &[Vec3] {
        &self.face_normals
    }

    /// Returns the corner normals (per triangle vertex).
    pub fn corner_normals(&self) -> &[Vec3] {
        &self.corner_normals
    }

    /// Returns the `edge_is_real` flags for each triangle corner.
    /// For each triangle vertex, this is a Vec3 where:
    /// - x = 1.0 if edge from this vertex to next is real, 0.0 if internal
    /// - y = 1.0 if edge from next to prev is real (always real for middle edge)
    /// - z = 1.0 if edge from prev to this is real, 0.0 if internal
    pub fn edge_is_real(&self) -> &[Vec3] {
        &self.edge_is_real
    }

    /// Returns the unique edges as sorted pairs.
    pub fn edges(&self) -> &[(u32, u32)] {
        &self.edges
    }

    /// Updates the vertex positions.
    pub fn update_vertices(&mut self, vertices: Vec<Vec3>) {
        self.vertices = vertices;
        self.needs_recompute = true;
        self.refresh();
    }

    /// Updates the faces.
    pub fn update_faces(&mut self, faces: Vec<Vec<u32>>) {
        self.faces = faces;
        self.needs_recompute = true;
        self.refresh();
    }

    // === Render option getters and setters ===

    /// Gets the shade style.
    pub fn shade_style(&self) -> ShadeStyle {
        self.shade_style
    }

    /// Sets the shade style.
    pub fn set_shade_style(&mut self, style: ShadeStyle) {
        self.shade_style = style;
    }

    /// Gets the edge width.
    pub fn edge_width(&self) -> f32 {
        self.edge_width
    }

    /// Sets the edge width.
    pub fn set_edge_width(&mut self, width: f32) {
        self.edge_width = width;
    }

    /// Gets the edge color.
    pub fn edge_color(&self) -> Vec3 {
        self.edge_color
    }

    /// Sets the edge color.
    pub fn set_edge_color(&mut self, color: Vec3) {
        self.edge_color = color;
    }

    /// Gets whether edges are shown.
    pub fn show_edges(&self) -> bool {
        self.show_edges
    }

    /// Sets whether edges are shown.
    pub fn set_show_edges(&mut self, show: bool) {
        self.show_edges = show;
    }

    /// Gets the backface policy.
    pub fn backface_policy(&self) -> BackfacePolicy {
        self.backface_policy
    }

    /// Sets the backface policy.
    pub fn set_backface_policy(&mut self, policy: BackfacePolicy) {
        self.backface_policy = policy;
    }

    /// Gets the backface color.
    pub fn backface_color(&self) -> Vec3 {
        self.backface_color
    }

    /// Sets the backface color.
    pub fn set_backface_color(&mut self, color: Vec3) {
        self.backface_color = color;
    }

    /// Gets the surface color.
    pub fn surface_color(&self) -> Vec3 {
        self.surface_color
    }

    /// Sets the surface color.
    pub fn set_surface_color(&mut self, color: Vec3) {
        self.surface_color = color;
    }

    /// Gets the transparency (1.0 = opaque, 0.0 = fully transparent).
    pub fn transparency(&self) -> f32 {
        self.transparency
    }

    /// Sets the transparency.
    pub fn set_transparency(&mut self, transparency: f32) {
        self.transparency = transparency.clamp(0.0, 1.0);
    }

    // === Computation methods ===

    /// Recomputes all derived data (triangulation, normals, edges).
    fn recompute(&mut self) {
        if !self.needs_recompute {
            return;
        }

        self.compute_triangulation();
        self.compute_face_normals();
        self.compute_vertex_normals();
        self.compute_corner_normals();
        self.compute_edges();
        self.compute_edge_is_real();

        self.needs_recompute = false;
    }

    /// Computes triangulation using fan triangulation.
    ///
    /// For a polygon with vertices [v0, v1, v2, v3, ...], creates triangles:
    /// [v0, v1, v2], [v0, v2, v3], [v0, v3, v4], ...
    fn compute_triangulation(&mut self) {
        self.triangulation.clear();
        self.face_to_tri_range.clear();

        for face in &self.faces {
            let start_tri = self.triangulation.len();

            if face.len() >= 3 {
                let v0 = face[0];
                // Fan triangulation: create (n-2) triangles for n-gon
                for i in 1..(face.len() - 1) {
                    self.triangulation.push([v0, face[i], face[i + 1]]);
                }
            }

            let end_tri = self.triangulation.len();
            self.face_to_tri_range.push(start_tri..end_tri);
        }
    }

    /// Computes face normals using cross product of first two edges.
    fn compute_face_normals(&mut self) {
        self.face_normals.clear();
        self.face_normals.reserve(self.faces.len());

        for face in &self.faces {
            if face.len() >= 3 {
                let v0 = self.vertices[face[0] as usize];
                let v1 = self.vertices[face[1] as usize];
                let v2 = self.vertices[face[2] as usize];

                let e1 = v1 - v0;
                let e2 = v2 - v0;
                let normal = e1.cross(e2).normalize_or_zero();
                self.face_normals.push(normal);
            } else {
                self.face_normals.push(Vec3::ZERO);
            }
        }
    }

    /// Computes vertex normals as area-weighted average of incident face normals.
    fn compute_vertex_normals(&mut self) {
        self.vertex_normals.clear();
        self.vertex_normals.resize(self.vertices.len(), Vec3::ZERO);

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                continue;
            }

            let face_normal = self.face_normals[face_idx];

            // Compute face area using triangulation
            let v0 = self.vertices[face[0] as usize];
            let mut area = 0.0;
            for i in 1..(face.len() - 1) {
                let v1 = self.vertices[face[i] as usize];
                let v2 = self.vertices[face[i + 1] as usize];
                let e1 = v1 - v0;
                let e2 = v2 - v0;
                area += e1.cross(e2).length() * 0.5;
            }

            // Add weighted normal to each vertex of this face
            let weighted_normal = face_normal * area;
            for &vi in face {
                self.vertex_normals[vi as usize] += weighted_normal;
            }
        }

        // Normalize all vertex normals
        for normal in &mut self.vertex_normals {
            *normal = normal.normalize_or_zero();
        }
    }

    /// Computes corner normals (per-corner of each triangle).
    fn compute_corner_normals(&mut self) {
        self.corner_normals.clear();
        self.corner_normals.reserve(self.triangulation.len() * 3);

        for (face_idx, range) in self.face_to_tri_range.iter().enumerate() {
            let face_normal = self.face_normals[face_idx];

            for tri_idx in range.clone() {
                let tri = self.triangulation[tri_idx];
                for vi in tri {
                    // For tri-flat, we use face normals; for smooth, we use vertex normals
                    // Store both options - the shader will choose based on shade_style
                    match self.shade_style {
                        ShadeStyle::Smooth => {
                            self.corner_normals.push(self.vertex_normals[vi as usize]);
                        }
                        ShadeStyle::Flat | ShadeStyle::TriFlat => {
                            self.corner_normals.push(face_normal);
                        }
                    }
                }
            }
        }
    }

    /// Computes `edge_is_real` flags for wireframe rendering.
    /// Marks which edges in the triangulation are real polygon edges vs internal.
    fn compute_edge_is_real(&mut self) {
        self.edge_is_real.clear();
        self.edge_is_real.reserve(self.triangulation.len() * 3);

        for (_face_idx, range) in self.face_to_tri_range.iter().enumerate() {
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

    /// Computes unique edges as sorted pairs.
    fn compute_edges(&mut self) {
        use std::collections::HashSet;

        let mut edge_set: HashSet<(u32, u32)> = HashSet::new();

        for face in &self.faces {
            let n = face.len();
            for i in 0..n {
                let v0 = face[i];
                let v1 = face[(i + 1) % n];
                // Store as sorted pair to avoid duplicates
                let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };
                edge_set.insert(edge);
            }
        }

        self.edges = edge_set.into_iter().collect();
        self.edges.sort(); // Sort for deterministic ordering
    }

    /// Builds the egui UI for this surface mesh.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        let mut shade_style = self.shade_style as u32;
        let mut color = [
            self.surface_color.x,
            self.surface_color.y,
            self.surface_color.z,
        ];
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

        // Show quantities
        if !self.quantities.is_empty() {
            ui.separator();
            ui.label("Quantities:");
            for quantity in &mut self.quantities {
                // Try downcasting to each known quantity type
                if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshVertexScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshFaceScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(cq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshVertexColorQuantity>()
                {
                    cq.build_egui_ui(ui);
                } else if let Some(cq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshFaceColorQuantity>()
                {
                    cq.build_egui_ui(ui);
                } else if let Some(vq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshVertexVectorQuantity>()
                {
                    vq.build_egui_ui(ui);
                }
            }
        }
    }

    // === Quantity methods ===

    /// Adds a vertex scalar quantity to this mesh.
    pub fn add_vertex_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = MeshVertexScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Returns the currently active vertex scalar quantity, if any.
    pub fn active_vertex_scalar_quantity(&self) -> Option<&MeshVertexScalarQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<MeshVertexScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Adds a face scalar quantity to this mesh.
    pub fn add_face_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = MeshFaceScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex color quantity to this mesh.
    pub fn add_vertex_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshVertexColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face color quantity to this mesh.
    pub fn add_face_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshFaceColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex vector quantity to this mesh.
    pub fn add_vertex_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshVertexVectorQuantity::new(name, self.name.clone(), vectors);
        self.add_quantity(Box::new(quantity));
        self
    }

    // === GPU resource methods ===

    /// Initializes GPU resources for rendering.
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

    /// Returns the render data if initialized.
    pub fn render_data(&self) -> Option<&SurfaceMeshRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU buffers with current mesh settings.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
        let Some(render_data) = &self.render_data else {
            return;
        };

        let uniforms = MeshUniforms {
            shade_style: self.shade_style as u32,
            show_edges: u32::from(self.show_edges),
            edge_width: self.edge_width,
            transparency: self.transparency,
            surface_color: [
                self.surface_color.x,
                self.surface_color.y,
                self.surface_color.z,
                1.0,
            ],
            edge_color: [self.edge_color.x, self.edge_color.y, self.edge_color.z, 1.0],
            backface_policy: self.backface_policy as u32,
            _pad1: [0.0; 3],
            _pad2: [0.0; 3],
            _pad3: 0.0,
            backface_color: [
                self.backface_color.x,
                self.backface_color.y,
                self.backface_color.z,
                1.0,
            ],
        };
        render_data.update_uniforms(queue, &uniforms);

        // Apply vertex scalar quantity colors if enabled
        // TODO: Need ColorMapRegistry to compute colors - this will be passed in from app.rs
        // if let Some(sq) = self.active_vertex_scalar_quantity() {
        //     if let Some(colormap) = color_maps.get(sq.colormap_name()) {
        //         let colors = sq.compute_colors(colormap);
        //         render_data.update_vertex_colors(queue, &colors);
        //     }
        // }
    }
}

impl Structure for SurfaceMesh {
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
        "SurfaceMesh"
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

        // Apply transform (same logic as PointCloud)
        let transform = self.transform;
        let corners = [
            transform.transform_point3(Vec3::new(min.x, min.y, min.z)),
            transform.transform_point3(Vec3::new(max.x, min.y, min.z)),
            transform.transform_point3(Vec3::new(min.x, max.y, min.z)),
            transform.transform_point3(Vec3::new(max.x, max.y, min.z)),
            transform.transform_point3(Vec3::new(min.x, min.y, max.z)),
            transform.transform_point3(Vec3::new(max.x, min.y, max.z)),
            transform.transform_point3(Vec3::new(min.x, max.y, max.z)),
            transform.transform_point3(Vec3::new(max.x, max.y, max.z)),
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
        if !self.enabled {
            return;
        }
        // TODO: Implement mesh rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {
            return;
        }
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
        // TODO: Refresh GPU buffers
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test basic triangle mesh creation.
    #[test]
    fn test_surface_mesh_creation_triangles() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_tri", vertices, faces);

        assert_eq!(mesh.num_vertices(), 3);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_triangles(), 1);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);
    }

    /// Test quad face creation.
    #[test]
    fn test_surface_mesh_creation_quad() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];

        let mesh = SurfaceMesh::new("test_quad", vertices, faces);

        assert_eq!(mesh.num_vertices(), 4);
        assert_eq!(mesh.num_faces(), 1);
        // A quad should produce 2 triangles
        assert_eq!(mesh.num_triangles(), 2);
    }

    /// Test quad triangulation produces correct triangles.
    #[test]
    fn test_quad_triangulation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];

        let mesh = SurfaceMesh::new("test_quad", vertices, faces);

        // Fan triangulation: [0,1,2], [0,2,3]
        assert_eq!(mesh.triangulation().len(), 2);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);
        assert_eq!(mesh.triangulation()[1], [0, 2, 3]);

        // Check face to triangle range
        assert_eq!(mesh.face_to_tri_range().len(), 1);
        assert_eq!(mesh.face_to_tri_range()[0], 0..2);
    }

    /// Test face normal computation.
    #[test]
    fn test_face_normals() {
        // Triangle in XY plane, CCW winding -> normal should point in +Z
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_normal", vertices, faces);

        assert_eq!(mesh.face_normals().len(), 1);
        let normal = mesh.face_normals()[0];
        // Should point in +Z direction
        assert!(
            (normal.z - 1.0).abs() < 1e-6,
            "Normal Z should be 1.0, got {}",
            normal.z
        );
        assert!(
            normal.x.abs() < 1e-6,
            "Normal X should be 0.0, got {}",
            normal.x
        );
        assert!(
            normal.y.abs() < 1e-6,
            "Normal Y should be 0.0, got {}",
            normal.y
        );
    }

    /// Test edge computation.
    #[test]
    fn test_edges() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_edges", vertices, faces);

        // Triangle has 3 unique edges
        assert_eq!(mesh.num_edges(), 3);

        // Edges should be sorted pairs
        let edges = mesh.edges();
        assert!(edges.contains(&(0, 1)));
        assert!(edges.contains(&(1, 2)));
        assert!(edges.contains(&(0, 2)));
    }

    /// Test vertex normals computation.
    #[test]
    fn test_vertex_normals() {
        // Single triangle in XY plane
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_vnormals", vertices, faces);

        // All vertex normals should point in +Z for a single flat triangle
        assert_eq!(mesh.vertex_normals().len(), 3);
        for normal in mesh.vertex_normals() {
            assert!(
                (normal.z - 1.0).abs() < 1e-6,
                "Vertex normal Z should be 1.0"
            );
        }
    }

    /// Test from_triangles convenience constructor.
    #[test]
    fn test_from_triangles() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2]];

        let mesh = SurfaceMesh::from_triangles("test_from_tri", vertices, triangles);

        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_triangles(), 1);
    }

    /// Test pentagon triangulation (5 vertices -> 3 triangles).
    #[test]
    fn test_pentagon_triangulation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.5, 0.5, 0.0),
            Vec3::new(0.75, 1.0, 0.0),
            Vec3::new(-0.25, 0.5, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3, 4]];

        let mesh = SurfaceMesh::new("test_pentagon", vertices, faces);

        // Pentagon -> 3 triangles via fan triangulation
        assert_eq!(mesh.num_triangles(), 3);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);
        assert_eq!(mesh.triangulation()[1], [0, 2, 3]);
        assert_eq!(mesh.triangulation()[2], [0, 3, 4]);

        // Pentagon has 5 edges
        assert_eq!(mesh.num_edges(), 5);
    }

    /// Test shared edge between two triangles.
    #[test]
    fn test_shared_edges() {
        // Two triangles sharing an edge (0-1)
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, -1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 3, 1]];

        let mesh = SurfaceMesh::new("test_shared", vertices, faces);

        // 2 triangles, but only 5 unique edges (edge 0-1 is shared)
        assert_eq!(mesh.num_triangles(), 2);
        assert_eq!(mesh.num_edges(), 5);
    }

    /// Test render options getters and setters.
    #[test]
    fn test_render_options() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mut mesh = SurfaceMesh::new("test_options", vertices, faces);

        // Test defaults
        assert_eq!(mesh.shade_style(), ShadeStyle::Smooth);
        assert_eq!(mesh.backface_policy(), BackfacePolicy::Identical);
        assert!(!mesh.show_edges());
        assert_eq!(mesh.transparency(), 0.0); // 0.0 = fully opaque

        // Test setters
        mesh.set_shade_style(ShadeStyle::Flat);
        assert_eq!(mesh.shade_style(), ShadeStyle::Flat);

        mesh.set_backface_policy(BackfacePolicy::Cull);
        assert_eq!(mesh.backface_policy(), BackfacePolicy::Cull);

        mesh.set_show_edges(true);
        assert!(mesh.show_edges());

        mesh.set_edge_width(2.0);
        assert_eq!(mesh.edge_width(), 2.0);

        mesh.set_edge_color(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(mesh.edge_color(), Vec3::new(1.0, 0.0, 0.0));

        mesh.set_surface_color(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.surface_color(), Vec3::new(0.0, 1.0, 0.0));

        mesh.set_backface_color(Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(mesh.backface_color(), Vec3::new(0.0, 0.0, 1.0));

        mesh.set_transparency(0.5);
        assert_eq!(mesh.transparency(), 0.5);

        // Test transparency clamping
        mesh.set_transparency(1.5);
        assert_eq!(mesh.transparency(), 1.0);
        mesh.set_transparency(-0.5);
        assert_eq!(mesh.transparency(), 0.0);
    }

    /// Test vertex scalar quantity.
    #[test]
    fn test_vertex_scalar_quantity() {
        use polyscope_core::quantity::QuantityKind;

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
        assert_eq!(q.kind(), QuantityKind::Scalar);
    }

    /// Test face scalar quantity.
    #[test]
    fn test_face_scalar_quantity() {
        use polyscope_core::quantity::QuantityKind;

        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = SurfaceMesh::new("test", vertices, faces);

        mesh.add_face_scalar_quantity("area", vec![1.0]);

        let q = mesh.get_quantity("area").expect("quantity not found");
        assert_eq!(q.data_size(), 1);
        assert_eq!(q.kind(), QuantityKind::Scalar);
    }

    /// Test vertex color quantity.
    #[test]
    fn test_vertex_color_quantity() {
        use polyscope_core::quantity::QuantityKind;

        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = SurfaceMesh::new("test", vertices, faces);

        mesh.add_vertex_color_quantity("colors", vec![Vec3::X, Vec3::Y, Vec3::Z]);

        let q = mesh.get_quantity("colors").expect("quantity not found");
        assert_eq!(q.data_size(), 3);
        assert_eq!(q.kind(), QuantityKind::Color);
    }

    /// Test face color quantity.
    #[test]
    fn test_face_color_quantity() {
        use polyscope_core::quantity::QuantityKind;

        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = SurfaceMesh::new("test", vertices, faces);

        mesh.add_face_color_quantity("face_colors", vec![Vec3::new(1.0, 0.0, 0.0)]);

        let q = mesh
            .get_quantity("face_colors")
            .expect("quantity not found");
        assert_eq!(q.data_size(), 1);
        assert_eq!(q.kind(), QuantityKind::Color);
    }

    /// Test vertex vector quantity.
    #[test]
    fn test_vertex_vector_quantity() {
        use polyscope_core::quantity::QuantityKind;

        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = SurfaceMesh::new("test", vertices, faces);

        mesh.add_vertex_vector_quantity("normals", vec![Vec3::Z, Vec3::Z, Vec3::Z]);

        let q = mesh.get_quantity("normals").expect("quantity not found");
        assert_eq!(q.data_size(), 3);
        assert_eq!(q.kind(), QuantityKind::Vector);
    }

    /// Test vector quantity properties.
    #[test]
    fn test_vector_quantity_properties() {
        let mut vq =
            MeshVertexVectorQuantity::new("test_vectors", "mesh", vec![Vec3::X, Vec3::Y, Vec3::Z]);

        // Test default values
        assert_eq!(vq.length_scale(), 1.0);
        assert_eq!(vq.radius(), 0.005);
        assert_eq!(vq.color(), Vec3::new(0.8, 0.2, 0.2));

        // Test setters
        vq.set_length_scale(2.0);
        assert_eq!(vq.length_scale(), 2.0);

        vq.set_radius(0.01);
        assert_eq!(vq.radius(), 0.01);

        vq.set_color(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(vq.color(), Vec3::new(0.0, 1.0, 0.0));
    }

    /// Test face scalar quantity compute_vertex_colors.
    #[test]
    fn test_face_scalar_compute_vertex_colors() {
        let fsq = MeshFaceScalarQuantity::new("test", "mesh", vec![0.0, 1.0]);
        let faces = vec![vec![0, 1, 2], vec![2, 1, 3]];
        let colormap = polyscope_render::ColorMap::new("test", vec![Vec3::ZERO, Vec3::ONE]);

        let colors = fsq.compute_vertex_colors(&faces, 4, &colormap);

        // Each vertex gets color from the last face it belongs to
        assert_eq!(colors.len(), 4);
        // Vertex 0 is only in face 0 (value 0.0) -> should be Vec3::ZERO
        assert!((colors[0] - Vec3::ZERO).length() < 1e-5);
        // Vertex 3 is only in face 1 (value 1.0) -> should be Vec3::ONE
        assert!((colors[3] - Vec3::ONE).length() < 1e-5);
    }

    /// Test face color quantity compute_vertex_colors.
    #[test]
    fn test_face_color_compute_vertex_colors() {
        let fcq = MeshFaceColorQuantity::new(
            "test",
            "mesh",
            vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
        );
        let faces = vec![vec![0, 1, 2], vec![2, 1, 3]];

        let colors = fcq.compute_vertex_colors(&faces, 4);

        assert_eq!(colors.len(), 4);
        // Vertex 0 is only in face 0 -> red
        assert_eq!(colors[0], Vec3::new(1.0, 0.0, 0.0));
        // Vertex 3 is only in face 1 -> green
        assert_eq!(colors[3], Vec3::new(0.0, 1.0, 0.0));
    }

    /// Test edge_is_real for triangle (all edges should be real).
    #[test]
    fn test_edge_is_real_triangle() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];

        let mesh = SurfaceMesh::new("test_tri", vertices, faces);

        // Triangle has 1 triangle, so 3 edge_is_real entries (one per corner)
        assert_eq!(mesh.edge_is_real().len(), 3);

        // For a single triangle (1 tri from fan), all edges are real
        // j=0, num_tris=1: edge0_real=1.0, edge1_real=1.0, edge2_real=1.0
        let expected = Vec3::new(1.0, 1.0, 1.0);
        for edge_real in mesh.edge_is_real() {
            assert_eq!(*edge_real, expected);
        }
    }

    /// Test edge_is_real for quad (internal edge should not be real).
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

        // Quad has 2 triangles, so 6 edge_is_real entries
        assert_eq!(mesh.edge_is_real().len(), 6);

        // Fan triangulation: [0,1,2], [0,2,3]
        // Triangle 0 (j=0, num_tris=2):
        //   edge0 (0->1): real (j==0)
        //   edge1 (1->2): always real
        //   edge2 (2->0): NOT real (j != num_tris-1)
        let tri0_expected = Vec3::new(1.0, 1.0, 0.0);

        // Triangle 1 (j=1, num_tris=2):
        //   edge0 (0->2): NOT real (j!=0)
        //   edge1 (2->3): always real
        //   edge2 (3->0): real (j == num_tris-1)
        let tri1_expected = Vec3::new(0.0, 1.0, 1.0);

        // First 3 entries are for triangle 0
        assert_eq!(mesh.edge_is_real()[0], tri0_expected);
        assert_eq!(mesh.edge_is_real()[1], tri0_expected);
        assert_eq!(mesh.edge_is_real()[2], tri0_expected);

        // Next 3 entries are for triangle 1
        assert_eq!(mesh.edge_is_real()[3], tri1_expected);
        assert_eq!(mesh.edge_is_real()[4], tri1_expected);
        assert_eq!(mesh.edge_is_real()[5], tri1_expected);
    }

    /// Test edge_is_real for pentagon (multiple internal edges).
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

        // Pentagon has 3 triangles, so 9 edge_is_real entries
        assert_eq!(mesh.edge_is_real().len(), 9);

        // Fan triangulation: [0,1,2], [0,2,3], [0,3,4]
        // Triangle 0 (j=0, num_tris=3): (1.0, 1.0, 0.0)
        // Triangle 1 (j=1, num_tris=3): (0.0, 1.0, 0.0) - middle triangle, only edge1 real
        // Triangle 2 (j=2, num_tris=3): (0.0, 1.0, 1.0)

        let tri0_expected = Vec3::new(1.0, 1.0, 0.0);
        let tri1_expected = Vec3::new(0.0, 1.0, 0.0);
        let tri2_expected = Vec3::new(0.0, 1.0, 1.0);

        // Check triangle 0 corners
        assert_eq!(mesh.edge_is_real()[0], tri0_expected);
        assert_eq!(mesh.edge_is_real()[1], tri0_expected);
        assert_eq!(mesh.edge_is_real()[2], tri0_expected);

        // Check triangle 1 corners
        assert_eq!(mesh.edge_is_real()[3], tri1_expected);
        assert_eq!(mesh.edge_is_real()[4], tri1_expected);
        assert_eq!(mesh.edge_is_real()[5], tri1_expected);

        // Check triangle 2 corners
        assert_eq!(mesh.edge_is_real()[6], tri2_expected);
        assert_eq!(mesh.edge_is_real()[7], tri2_expected);
        assert_eq!(mesh.edge_is_real()[8], tri2_expected);
    }
}
