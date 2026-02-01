//! Surface mesh structure.

mod intrinsic_vector_quantity;
mod one_form_quantity;
mod parameterization_quantity;
mod quantities;
pub use intrinsic_vector_quantity::*;
pub use one_form_quantity::*;
pub use parameterization_quantity::*;
pub use quantities::*;

use glam::{Mat4, Vec2, Vec3, Vec4};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{ColorMapRegistry, MeshPickUniforms, MeshUniforms, SurfaceMeshRenderData};
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
    material: String,
    shade_style: ShadeStyle,
    edge_width: f32,
    edge_color: Vec4,
    show_edges: bool,
    backface_policy: BackfacePolicy,
    backface_color: Vec4,
    surface_color: Vec4,
    transparency: f32,

    // GPU resources
    render_data: Option<SurfaceMeshRenderData>,

    // GPU picking resources
    pick_uniform_buffer: Option<wgpu::Buffer>,
    pick_bind_group: Option<wgpu::BindGroup>,
    pick_face_index_buffer: Option<wgpu::Buffer>,
    global_start: u32,
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
            material: "clay".to_string(),
            shade_style: ShadeStyle::default(),
            edge_width: 1.0,
            edge_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            show_edges: false,
            backface_policy: BackfacePolicy::default(),
            backface_color: Vec4::new(0.3, 0.3, 0.3, 1.0),
            surface_color: Vec4::new(0.5, 0.5, 0.8, 1.0),
            transparency: 0.0, // 0.0 = fully opaque, 1.0 = fully transparent

            render_data: None,

            pick_uniform_buffer: None,
            pick_bind_group: None,
            pick_face_index_buffer: None,
            global_start: 0,
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
    #[must_use]
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of faces.
    #[must_use]
    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }

    /// Returns the number of triangles in the triangulation.
    #[must_use]
    pub fn num_triangles(&self) -> usize {
        self.triangulation.len()
    }

    /// Returns the number of edges.
    #[must_use]
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Returns the vertices.
    #[must_use]
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Returns the faces (polygon indices).
    #[must_use]
    pub fn faces(&self) -> &[Vec<u32>] {
        &self.faces
    }

    /// Returns the triangulation.
    #[must_use]
    pub fn triangulation(&self) -> &[[u32; 3]] {
        &self.triangulation
    }

    /// Returns the mapping from face index to triangle range.
    #[must_use]
    pub fn face_to_tri_range(&self) -> &[Range<usize>] {
        &self.face_to_tri_range
    }

    /// Returns the vertex normals.
    #[must_use]
    pub fn vertex_normals(&self) -> &[Vec3] {
        &self.vertex_normals
    }

    /// Returns the face normals.
    #[must_use]
    pub fn face_normals(&self) -> &[Vec3] {
        &self.face_normals
    }

    /// Returns the corner normals (per triangle vertex).
    #[must_use]
    pub fn corner_normals(&self) -> &[Vec3] {
        &self.corner_normals
    }

    /// Returns the `edge_is_real` flags for each triangle corner.
    /// For each triangle vertex, this is a Vec3 where:
    /// - x = 1.0 if edge from this vertex to next is real, 0.0 if internal
    /// - y = 1.0 if edge from next to prev is real (always real for middle edge)
    /// - z = 1.0 if edge from prev to this is real, 0.0 if internal
    #[must_use]
    pub fn edge_is_real(&self) -> &[Vec3] {
        &self.edge_is_real
    }

    /// Returns the unique edges as sorted pairs.
    #[must_use]
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
    #[must_use]
    pub fn shade_style(&self) -> ShadeStyle {
        self.shade_style
    }

    /// Sets the shade style.
    pub fn set_shade_style(&mut self, style: ShadeStyle) {
        self.shade_style = style;
    }

    /// Gets the edge width.
    #[must_use]
    pub fn edge_width(&self) -> f32 {
        self.edge_width
    }

    /// Sets the edge width.
    pub fn set_edge_width(&mut self, width: f32) {
        self.edge_width = width;
    }

    /// Gets the edge color.
    #[must_use]
    pub fn edge_color(&self) -> Vec4 {
        self.edge_color
    }

    /// Sets the edge color.
    pub fn set_edge_color(&mut self, color: Vec3) {
        self.edge_color = color.extend(1.0);
    }

    /// Gets whether edges are shown.
    #[must_use]
    pub fn show_edges(&self) -> bool {
        self.show_edges
    }

    /// Sets whether edges are shown.
    pub fn set_show_edges(&mut self, show: bool) {
        self.show_edges = show;
    }

    /// Gets the backface policy.
    #[must_use]
    pub fn backface_policy(&self) -> BackfacePolicy {
        self.backface_policy
    }

    /// Sets the backface policy.
    pub fn set_backface_policy(&mut self, policy: BackfacePolicy) {
        self.backface_policy = policy;
    }

    /// Gets the backface color.
    #[must_use]
    pub fn backface_color(&self) -> Vec4 {
        self.backface_color
    }

    /// Sets the backface color.
    pub fn set_backface_color(&mut self, color: Vec3) {
        self.backface_color = color.extend(1.0);
    }

    /// Gets the surface color.
    #[must_use]
    pub fn surface_color(&self) -> Vec4 {
        self.surface_color
    }

    /// Sets the surface color.
    pub fn set_surface_color(&mut self, color: Vec3) {
        self.surface_color = color.extend(1.0);
    }

    /// Gets the transparency (0.0 = opaque, 1.0 = fully transparent).
    #[must_use]
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

        for range in &self.face_to_tri_range {
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
        self.edges.sort_unstable(); // Sort for deterministic ordering
    }

    /// Builds the egui UI for this surface mesh.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui, available_materials: &[&str]) {
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
            &mut self.material,
            available_materials,
        ) {
            self.shade_style = match shade_style {
                0 => ShadeStyle::Smooth,
                1 => ShadeStyle::Flat,
                _ => ShadeStyle::TriFlat,
            };
            self.surface_color = Vec4::new(color[0], color[1], color[2], self.surface_color.w);
            self.transparency = transparency;
            self.show_edges = show_edges;
            self.edge_width = edge_width;
            self.edge_color = Vec4::new(
                edge_color[0],
                edge_color[1],
                edge_color[2],
                self.edge_color.w,
            );
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
                } else if let Some(vq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshFaceVectorQuantity>()
                {
                    vq.build_egui_ui(ui);
                } else if let Some(pq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshVertexParameterizationQuantity>()
                {
                    pq.build_egui_ui(ui);
                } else if let Some(pq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshCornerParameterizationQuantity>()
                {
                    pq.build_egui_ui(ui);
                } else if let Some(iq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshVertexIntrinsicVectorQuantity>()
                {
                    iq.build_egui_ui(ui);
                } else if let Some(iq) = quantity
                    .as_any_mut()
                    .downcast_mut::<MeshFaceIntrinsicVectorQuantity>()
                {
                    iq.build_egui_ui(ui);
                } else if let Some(oq) = quantity.as_any_mut().downcast_mut::<MeshOneFormQuantity>()
                {
                    oq.build_egui_ui(ui);
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
    #[must_use]
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

    /// Returns the currently active face scalar quantity, if any.
    #[must_use]
    pub fn active_face_scalar_quantity(&self) -> Option<&MeshFaceScalarQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<MeshFaceScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex color quantity, if any.
    #[must_use]
    pub fn active_vertex_color_quantity(&self) -> Option<&MeshVertexColorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<MeshVertexColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active face color quantity, if any.
    #[must_use]
    pub fn active_face_color_quantity(&self) -> Option<&MeshFaceColorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<MeshFaceColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex vector quantity (immutable), if any.
    #[must_use]
    pub fn active_vertex_vector_quantity(&self) -> Option<&MeshVertexVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any().downcast_ref::<MeshVertexVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex vector quantity (mutable), if any.
    pub fn active_vertex_vector_quantity_mut(&mut self) -> Option<&mut MeshVertexVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any_mut().downcast_mut::<MeshVertexVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active face vector quantity (immutable), if any.
    #[must_use]
    pub fn active_face_vector_quantity(&self) -> Option<&MeshFaceVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any().downcast_ref::<MeshFaceVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active face vector quantity (mutable), if any.
    pub fn active_face_vector_quantity_mut(&mut self) -> Option<&mut MeshFaceVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any_mut().downcast_mut::<MeshFaceVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Computes face centroids (center of each face).
    #[must_use]
    pub fn face_centroids(&self) -> Vec<Vec3> {
        self.faces
            .iter()
            .map(|face| {
                if face.is_empty() {
                    return Vec3::ZERO;
                }
                let sum: Vec3 = face.iter().map(|&vi| self.vertices[vi as usize]).sum();
                sum / face.len() as f32
            })
            .collect()
    }

    /// Returns the currently active vertex parameterization quantity, if any.
    #[must_use]
    pub fn active_vertex_parameterization_quantity(
        &self,
    ) -> Option<&MeshVertexParameterizationQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Parameterization {
                if let Some(pq) = q
                    .as_any()
                    .downcast_ref::<MeshVertexParameterizationQuantity>()
                {
                    return Some(pq);
                }
            }
        }
        None
    }

    /// Returns the currently active corner parameterization quantity, if any.
    #[must_use]
    pub fn active_corner_parameterization_quantity(
        &self,
    ) -> Option<&MeshCornerParameterizationQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Parameterization {
                if let Some(pq) = q
                    .as_any()
                    .downcast_ref::<MeshCornerParameterizationQuantity>()
                {
                    return Some(pq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex intrinsic vector quantity (immutable), if any.
    #[must_use]
    pub fn active_vertex_intrinsic_vector_quantity(
        &self,
    ) -> Option<&MeshVertexIntrinsicVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any()
                    .downcast_ref::<MeshVertexIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex intrinsic vector quantity (mutable), if any.
    pub fn active_vertex_intrinsic_vector_quantity_mut(
        &mut self,
    ) -> Option<&mut MeshVertexIntrinsicVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any_mut()
                    .downcast_mut::<MeshVertexIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active face intrinsic vector quantity (immutable), if any.
    #[must_use]
    pub fn active_face_intrinsic_vector_quantity(
        &self,
    ) -> Option<&MeshFaceIntrinsicVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q.as_any().downcast_ref::<MeshFaceIntrinsicVectorQuantity>() {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active face intrinsic vector quantity (mutable), if any.
    pub fn active_face_intrinsic_vector_quantity_mut(
        &mut self,
    ) -> Option<&mut MeshFaceIntrinsicVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any_mut()
                    .downcast_mut::<MeshFaceIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active one-form quantity (immutable), if any.
    #[must_use]
    pub fn active_one_form_quantity(&self) -> Option<&MeshOneFormQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(oq) = q.as_any().downcast_ref::<MeshOneFormQuantity>() {
                    return Some(oq);
                }
            }
        }
        None
    }

    /// Returns the currently active one-form quantity (mutable), if any.
    pub fn active_one_form_quantity_mut(&mut self) -> Option<&mut MeshOneFormQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(oq) = q.as_any_mut().downcast_mut::<MeshOneFormQuantity>() {
                    return Some(oq);
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

    /// Adds a vertex color quantity to this mesh (RGB, alpha defaults to 1.0).
    pub fn add_vertex_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshVertexColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex color quantity with explicit per-vertex RGBA alpha values.
    pub fn add_vertex_color_quantity_with_alpha(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> &mut Self {
        let quantity = MeshVertexColorQuantity::new_with_alpha(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face color quantity to this mesh (RGB, alpha defaults to 1.0).
    pub fn add_face_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshFaceColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face color quantity with explicit per-face RGBA alpha values.
    pub fn add_face_color_quantity_with_alpha(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> &mut Self {
        let quantity = MeshFaceColorQuantity::new_with_alpha(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex vector quantity to this mesh.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_vertex_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshVertexVectorQuantity::new(name, self.name.clone(), vectors);
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face vector quantity to this mesh.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_face_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshFaceVectorQuantity::new(name, self.name.clone(), vectors);
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex parameterization (UV) quantity to this mesh.
    pub fn add_vertex_parameterization_quantity(
        &mut self,
        name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> &mut Self {
        let quantity = MeshVertexParameterizationQuantity::new(name, self.name.clone(), coords);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a corner parameterization (UV) quantity to this mesh.
    pub fn add_corner_parameterization_quantity(
        &mut self,
        name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> &mut Self {
        let quantity = MeshCornerParameterizationQuantity::new(name, self.name.clone(), coords);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex intrinsic vector quantity with explicit tangent basis.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_vertex_intrinsic_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshVertexIntrinsicVectorQuantity::new(
            name,
            self.name.clone(),
            vectors,
            basis_x,
            basis_y,
        );
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_vertex_intrinsic_vector_quantity_auto(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
    ) -> &mut Self {
        let (bx, by) = self.compute_vertex_tangent_basis();
        self.add_vertex_intrinsic_vector_quantity(name, vectors, bx, by)
    }

    /// Adds a face intrinsic vector quantity with explicit tangent basis.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_face_intrinsic_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshFaceIntrinsicVectorQuantity::new(
            name,
            self.name.clone(),
            vectors,
            basis_x,
            basis_y,
        );
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_face_intrinsic_vector_quantity_auto(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
    ) -> &mut Self {
        let (bx, by) = self.compute_face_tangent_basis();
        self.add_face_intrinsic_vector_quantity(name, vectors, bx, by)
    }

    /// Adds a one-form quantity to this mesh.
    ///
    /// A one-form assigns a scalar value to each edge, rendered as arrows
    /// at edge midpoints. The `orientations` array specifies the sign convention
    /// for each edge (true = canonical low→high vertex direction).
    /// Arrow length and radius are auto-scaled based on mesh extent and edge flow magnitudes.
    pub fn add_one_form_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
        orientations: Vec<bool>,
    ) -> &mut Self {
        let mut quantity = MeshOneFormQuantity::new(name, self.name.clone(), values, orientations);
        quantity.auto_scale(self.length_scale(), &self.vertices, &self.edges);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Compute default per-face tangent basis from first edge direction.
    #[must_use]
    pub fn compute_face_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut basis_x = Vec::with_capacity(self.faces.len());
        let mut basis_y = Vec::with_capacity(self.faces.len());

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() >= 3 {
                let v0 = self.vertices[face[0] as usize];
                let v1 = self.vertices[face[1] as usize];
                let normal = self.face_normals[face_idx];

                let bx = (v1 - v0).normalize_or_zero();
                let by = normal.cross(bx).normalize_or_zero();
                basis_x.push(bx);
                basis_y.push(by);
            } else {
                basis_x.push(Vec3::X);
                basis_y.push(Vec3::Y);
            }
        }

        (basis_x, basis_y)
    }

    /// Compute default per-vertex tangent basis from area-weighted face bases.
    #[must_use]
    pub fn compute_vertex_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let (face_bx, _face_by) = self.compute_face_tangent_basis();

        let mut vert_bx = vec![Vec3::ZERO; self.vertices.len()];

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                continue;
            }

            // Compute face area
            let v0 = self.vertices[face[0] as usize];
            let mut area = 0.0f32;
            for i in 1..(face.len() - 1) {
                let v1 = self.vertices[face[i] as usize];
                let v2 = self.vertices[face[i + 1] as usize];
                area += (v1 - v0).cross(v2 - v0).length() * 0.5;
            }

            let weighted_bx = face_bx[face_idx] * area;
            for &vi in face {
                vert_bx[vi as usize] += weighted_bx;
            }
        }

        // Orthonormalize against vertex normals
        let mut basis_x = Vec::with_capacity(self.vertices.len());
        let mut basis_y = Vec::with_capacity(self.vertices.len());

        for (i, normal) in self.vertex_normals.iter().enumerate() {
            let mut bx = vert_bx[i];
            // Project out normal component and normalize
            bx -= *normal * normal.dot(bx);
            bx = bx.normalize_or_zero();

            // If degenerate, pick an arbitrary tangent
            if bx.length_squared() < 1e-6 {
                bx = if normal.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                bx -= *normal * normal.dot(bx);
                bx = bx.normalize_or_zero();
            }

            let by = normal.cross(bx).normalize_or_zero();
            basis_x.push(bx);
            basis_y.push(by);
        }

        (basis_x, basis_y)
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
    #[must_use]
    pub fn render_data(&self) -> Option<&SurfaceMeshRenderData> {
        self.render_data.as_ref()
    }

    /// Initializes shadow rendering resources.
    ///
    /// Creates the bind group needed to render this mesh in the shadow pass.
    pub fn init_shadow_resources(
        &mut self,
        device: &wgpu::Device,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
        light_buffer: &wgpu::Buffer,
    ) {
        if let Some(render_data) = &mut self.render_data {
            render_data.init_shadow_resources(device, shadow_bind_group_layout, light_buffer);
        }
    }

    /// Returns the shadow bind group if initialized.
    #[must_use]
    pub fn shadow_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.render_data.as_ref()?.shadow_bind_group.as_ref()
    }

    /// Returns whether shadow resources are initialized.
    #[must_use]
    pub fn has_shadow_resources(&self) -> bool {
        self.render_data
            .as_ref()
            .is_some_and(polyscope_render::SurfaceMeshRenderData::has_shadow_resources)
    }

    /// Initializes GPU resources for pick rendering.
    ///
    /// Creates the pick uniform buffer, face index mapping buffer, and bind group.
    /// The face index buffer maps each GPU triangle to its original polygon face index.
    pub fn init_pick_resources(
        &mut self,
        device: &wgpu::Device,
        mesh_pick_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        global_start: u32,
    ) {
        use wgpu::util::DeviceExt;

        self.global_start = global_start;

        // Create pick uniform buffer with MeshPickUniforms
        let model = self.transform.to_cols_array_2d();
        let pick_uniforms = MeshPickUniforms {
            global_start,
            _padding: [0.0; 3],
            model,
        };
        let pick_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh pick uniforms"),
            contents: bytemuck::cast_slice(&[pick_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Build face index mapping buffer: tri_index -> face_index
        // For each triangle in the triangulation, store which polygon face it belongs to
        let mut face_index_data: Vec<u32> = Vec::with_capacity(self.triangulation.len());
        for (face_idx, range) in self.face_to_tri_range.iter().enumerate() {
            for _ in range.clone() {
                face_index_data.push(face_idx as u32);
            }
        }
        let pick_face_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh pick face indices"),
            contents: bytemuck::cast_slice(&face_index_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create pick bind group (reuses position buffer from render_data)
        if let Some(render_data) = &self.render_data {
            let pick_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mesh pick bind group"),
                layout: mesh_pick_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: pick_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: render_data.vertex_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: pick_face_index_buffer.as_entire_binding(),
                    },
                ],
            });
            self.pick_bind_group = Some(pick_bind_group);
        }

        self.pick_uniform_buffer = Some(pick_uniform_buffer);
        self.pick_face_index_buffer = Some(pick_face_index_buffer);
    }

    /// Returns the pick bind group if initialized.
    #[must_use]
    pub fn pick_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.pick_bind_group.as_ref()
    }

    /// Updates pick uniforms (model transform) when the structure is moved.
    pub fn update_pick_uniforms(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.pick_uniform_buffer {
            let model = self.transform.to_cols_array_2d();
            let pick_uniforms = MeshPickUniforms {
                global_start: self.global_start,
                _padding: [0.0; 3],
                model,
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[pick_uniforms]));
        }
    }

    /// Returns the total number of vertices in the triangulation (for draw calls).
    #[must_use]
    pub fn num_triangulation_vertices(&self) -> u32 {
        (self.triangulation.len() * 3) as u32
    }

    /// Updates GPU buffers with current mesh settings.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
        let Some(render_data) = &self.render_data else {
            return;
        };

        // Convert glam Mat4 to [[f32; 4]; 4] for GPU
        let model_matrix = self.transform.to_cols_array_2d();

        let mut use_vertex_color = false;

        // Apply quantity colors with priority:
        // vertex param > corner param > vertex color > face color > vertex scalar > face scalar > surface color
        if let Some(pq) = self.active_vertex_parameterization_quantity() {
            use_vertex_color = true;
            let colors = pq.compute_colors();
            render_data.update_colors(queue, &colors, &self.triangulation);
        } else if let Some(pq) = self.active_corner_parameterization_quantity() {
            use_vertex_color = true;
            // Corner parameterization: compute per-corner colors, expand to per-vertex
            // (for now, treat as per-face by averaging corners)
            let corner_colors = pq.compute_colors();
            let mut vertex_colors = vec![Vec4::splat(0.5); self.vertices.len()];
            let mut counts = vec![0u32; self.vertices.len()];
            let mut corner_idx = 0;
            for face in &self.faces {
                for &vi in face {
                    if corner_idx < corner_colors.len() {
                        vertex_colors[vi as usize] += corner_colors[corner_idx];
                        counts[vi as usize] += 1;
                        corner_idx += 1;
                    }
                }
            }
            for (i, count) in counts.iter().enumerate() {
                if *count > 0 {
                    vertex_colors[i] /= *count as f32;
                }
            }
            render_data.update_colors(queue, &vertex_colors, &self.triangulation);
        } else if let Some(cq) = self.active_vertex_color_quantity() {
            use_vertex_color = true;
            // Direct vertex color quantity
            render_data.update_colors(queue, cq.colors(), &self.triangulation);
        } else if let Some(cq) = self.active_face_color_quantity() {
            use_vertex_color = true;
            // Face color expanded to vertices
            let colors = cq.compute_vertex_colors(&self.faces, self.vertices.len());
            render_data.update_colors(queue, &colors, &self.triangulation);
        } else if let Some(sq) = self.active_vertex_scalar_quantity() {
            use_vertex_color = true;
            // Vertex scalar mapped through colormap
            if let Some(colormap) = color_maps.get(sq.colormap_name()) {
                let colors = sq.compute_colors(colormap);
                render_data.update_colors(queue, &colors, &self.triangulation);
            }
        } else if let Some(sq) = self.active_face_scalar_quantity() {
            use_vertex_color = true;
            // Face scalar mapped through colormap and expanded to vertices
            if let Some(colormap) = color_maps.get(sq.colormap_name()) {
                let colors = sq.compute_vertex_colors(&self.faces, self.vertices.len(), colormap);
                render_data.update_colors(queue, &colors, &self.triangulation);
            }
        } else {
            // No quantity enabled - clear colors so shader uses surface_color
            render_data.clear_colors(queue);
        }

        let uniforms = MeshUniforms {
            model_matrix,
            shade_style: self.shade_style as u32,
            show_edges: u32::from(self.show_edges),
            edge_width: self.edge_width,
            transparency: self.transparency,
            surface_color: self.surface_color.to_array(),
            edge_color: self.edge_color.to_array(),
            backface_policy: self.backface_policy as u32,
            slice_planes_enabled: 1,
            use_vertex_color: u32::from(use_vertex_color),
            _pad1: 0.0,
            _pad2: [0.0; 3],
            _pad3: 0.0,
            backface_color: self.backface_color.to_array(),
        };
        render_data.update_uniforms(queue, &uniforms);

        // Update shadow model buffer if initialized
        render_data.update_shadow_model(queue, model_matrix);
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
            .map_or(1.0, |(min, max)| (max - min).length())
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

    fn material(&self) -> &str {
        &self.material
    }

    fn set_material(&mut self, material: &str) {
        self.material = material.to_string();
    }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        // Rendering is handled by polyscope/src/app/render.rs
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        // Pick rendering is handled by polyscope/src/app/render.rs
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is handled by polyscope-ui/src/structure_ui.rs
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // Pick UI is handled by polyscope-ui/src/panels.rs
    }

    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
        self.pick_uniform_buffer = None;
        self.pick_bind_group = None;
        self.pick_face_index_buffer = None;
        for quantity in &mut self.quantities {
            quantity.clear_gpu_resources();
        }
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
        self.quantities
            .iter()
            .find(|q| q.name() == name)
            .map(std::convert::AsRef::as_ref)
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

    /// Test triangle and quad mesh creation and triangulation.
    #[test]
    fn test_surface_mesh_creation_and_triangulation() {
        // Triangle mesh
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let mesh = SurfaceMesh::new("test_tri", vertices, vec![vec![0, 1, 2]]);
        assert_eq!(mesh.num_vertices(), 3);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_triangles(), 1);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);

        // Quad mesh with fan triangulation
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let mesh = SurfaceMesh::new("test_quad", vertices, vec![vec![0, 1, 2, 3]]);
        assert_eq!(mesh.num_vertices(), 4);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_triangles(), 2);
        assert_eq!(mesh.triangulation()[0], [0, 1, 2]);
        assert_eq!(mesh.triangulation()[1], [0, 2, 3]);
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
        assert_eq!(mesh.edge_color(), Vec4::new(1.0, 0.0, 0.0, 1.0));

        mesh.set_surface_color(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.surface_color(), Vec4::new(0.0, 1.0, 0.0, 1.0));

        mesh.set_backface_color(Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(mesh.backface_color(), Vec4::new(0.0, 0.0, 1.0, 1.0));

        mesh.set_transparency(0.5);
        assert_eq!(mesh.transparency(), 0.5);

        // Test transparency clamping
        mesh.set_transparency(1.5);
        assert_eq!(mesh.transparency(), 1.0);
        mesh.set_transparency(-0.5);
        assert_eq!(mesh.transparency(), 0.0);
    }

    /// Test all quantity types on a surface mesh.
    #[test]
    fn test_surface_mesh_quantities() {
        use polyscope_core::quantity::QuantityKind;

        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = SurfaceMesh::new("test", vertices, faces);

        // Vertex quantities (size = 3)
        mesh.add_vertex_scalar_quantity("height", vec![0.0, 0.5, 1.0]);
        mesh.add_vertex_color_quantity("colors", vec![Vec3::X, Vec3::Y, Vec3::Z]);
        mesh.add_vertex_vector_quantity("normals", vec![Vec3::Z, Vec3::Z, Vec3::Z]);

        // Face quantities (size = 1)
        mesh.add_face_scalar_quantity("area", vec![1.0]);
        mesh.add_face_color_quantity("face_colors", vec![Vec3::new(1.0, 0.0, 0.0)]);
        mesh.add_face_vector_quantity("face_normals", vec![Vec3::Z]);

        let cases: &[(&str, usize, QuantityKind)] = &[
            ("height", 3, QuantityKind::Scalar),
            ("colors", 3, QuantityKind::Color),
            ("normals", 3, QuantityKind::Vector),
            ("area", 1, QuantityKind::Scalar),
            ("face_colors", 1, QuantityKind::Color),
            ("face_normals", 1, QuantityKind::Vector),
        ];

        for (name, expected_size, expected_kind) in cases {
            let q = mesh
                .get_quantity(name)
                .unwrap_or_else(|| panic!("quantity '{name}' not found"));
            assert_eq!(
                q.data_size(),
                *expected_size,
                "data_size mismatch for {name}"
            );
            assert_eq!(q.kind(), *expected_kind, "kind mismatch for {name}");
        }
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
        // Vertex 0 is only in face 0 (value 0.0) -> should be near zero RGB
        assert!((colors[0] - Vec4::new(0.0, 0.0, 0.0, 1.0)).length() < 1e-5);
        // Vertex 3 is only in face 1 (value 1.0) -> should be near one RGB
        assert!((colors[3] - Vec4::new(1.0, 1.0, 1.0, 1.0)).length() < 1e-5);
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
        assert_eq!(colors[0], Vec4::new(1.0, 0.0, 0.0, 1.0));
        // Vertex 3 is only in face 1 -> green
        assert_eq!(colors[3], Vec4::new(0.0, 1.0, 0.0, 1.0));
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
