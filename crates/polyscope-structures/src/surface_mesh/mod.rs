//! Surface mesh structure.

use glam::{Mat4, UVec3, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};

/// A surface mesh structure (triangular or polygonal).
pub struct SurfaceMesh {
    name: String,
    vertices: Vec<Vec3>,
    faces: Vec<UVec3>, // For now, triangles only. TODO: Support polygons
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,
    // TODO: Add computed normals, edges, GPU buffers
}

impl SurfaceMesh {
    /// Creates a new surface mesh from triangles.
    pub fn new(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        faces: Vec<UVec3>,
    ) -> Self {
        Self {
            name: name.into(),
            vertices,
            faces,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
        }
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

    /// Returns the faces (triangle indices).
    pub fn faces(&self) -> &[UVec3] {
        &self.faces
    }

    /// Updates the vertex positions.
    pub fn update_vertices(&mut self, vertices: Vec<Vec3>) {
        self.vertices = vertices;
        self.refresh();
    }

    // TODO: Add quantity methods (vertex scalar, face scalar, vertex vector, etc.)
}

impl Structure for SurfaceMesh {
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
        // TODO: Refresh GPU buffers, recompute normals
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
        self.quantities
            .iter_mut()
            .find(|q| q.name() == name)
    }

    fn remove_quantity(&mut self, name: &str) -> Option<Box<dyn Quantity>> {
        let idx = self.quantities.iter().position(|q| q.name() == name)?;
        Some(self.quantities.remove(idx))
    }

    fn quantities(&self) -> &[Box<dyn Quantity>] {
        &self.quantities
    }
}
