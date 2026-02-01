//! One-form quantities for surface meshes.
//!
//! A one-form assigns a scalar value to each edge of the mesh,
//! representing integrated flux or circulation along the edge.
//! It is rendered as vector arrows at edge midpoints.

use glam::{Vec3, Vec4};
use polyscope_core::quantity::{EdgeQuantity, Quantity, QuantityKind};
use polyscope_render::{VectorRenderData, VectorUniforms};

/// A one-form quantity on a surface mesh.
///
/// Stores one scalar value per edge with orientation conventions.
/// Rendered as arrows at edge midpoints, where the arrow direction is
/// along the edge and length is proportional to the value.
pub struct MeshOneFormQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,        // One scalar per edge
    orientations: Vec<bool>, // Edge orientation: true = default (low→high index)
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec4,
    render_data: Option<VectorRenderData>,
}

impl MeshOneFormQuantity {
    /// Creates a new one-form quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
        orientations: Vec<bool>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            orientations,
            enabled: false,
            length_scale: 1.0,
            radius: 0.005,
            color: Vec4::new(0.2, 0.7, 0.2, 1.0),
            render_data: None,
        }
    }

    /// Returns the per-edge scalar values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Returns the edge orientation flags.
    #[must_use]
    pub fn orientations(&self) -> &[bool] {
        &self.orientations
    }

    /// Gets the length scale.
    #[must_use]
    pub fn length_scale(&self) -> f32 {
        self.length_scale
    }

    /// Sets the length scale.
    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.length_scale = scale;
        self
    }

    /// Gets the radius.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, r: f32) -> &mut Self {
        self.radius = r;
        self
    }

    /// Gets the color.
    #[must_use]
    pub fn color(&self) -> Vec4 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) -> &mut Self {
        self.color = c.extend(1.0);
        self
    }

    /// Convert edge scalars + orientations to vector field for rendering.
    ///
    /// Returns `(positions, vectors)` — one arrow per edge at the edge midpoint.
    /// The vector direction is along the edge, scaled by the one-form value.
    #[must_use]
    pub fn compute_edge_vectors(
        &self,
        vertices: &[Vec3],
        edges: &[(u32, u32)],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut positions = Vec::with_capacity(self.values.len());
        let mut vectors = Vec::with_capacity(self.values.len());

        for (i, &(v0_idx, v1_idx)) in edges.iter().enumerate() {
            if i >= self.values.len() {
                break;
            }

            let v0 = vertices[v0_idx as usize];
            let v1 = vertices[v1_idx as usize];

            // Midpoint
            let midpoint = (v0 + v1) * 0.5;

            // Canonical direction: v0 → v1 (low → high index, since edges are sorted)
            let mut direction = (v1 - v0).normalize_or_zero();

            // Flip direction if orientation is false
            if !self.orientations[i] {
                direction = -direction;
            }

            let vector = direction * self.values[i] * self.length_scale;

            positions.push(midpoint);
            vectors.push(vector);
        }

        (positions, vectors)
    }

    /// Auto-scales length and radius based on the structure's bounding box diagonal.
    pub fn auto_scale(
        &mut self,
        structure_length_scale: f32,
        vertices: &[Vec3],
        edges: &[(u32, u32)],
    ) {
        let (_positions, vecs) = self.compute_edge_vectors(vertices, edges);
        let avg_length: f32 = if vecs.is_empty() {
            1.0
        } else {
            let sum: f32 = vecs.iter().map(|v| v.length()).sum();
            sum / vecs.len() as f32
        };
        if avg_length > 1e-8 {
            self.length_scale = 0.02 * structure_length_scale / avg_length;
        }
        self.radius = 0.002 * structure_length_scale;
    }

    /// Initializes GPU resources for this vector quantity.
    ///
    /// Computes edge midpoint positions and direction vectors from the mesh,
    /// then creates GPU buffers for arrow rendering.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        edges: &[(u32, u32)],
    ) {
        let (positions, vectors) = self.compute_edge_vectors(vertices, edges);
        self.render_data = Some(VectorRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &positions,
            &vectors,
        ));
    }

    /// Returns the render data if initialized.
    #[must_use]
    pub fn render_data(&self) -> Option<&VectorRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU uniforms with the given model transform.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, model: &glam::Mat4) {
        if let Some(render_data) = &self.render_data {
            let uniforms = VectorUniforms {
                model: model.to_cols_array(),
                length_scale: self.length_scale,
                radius: self.radius,
                _padding: [0.0; 2],
                color: self.color.to_array(),
            };
            render_data.update_uniforms(queue, &uniforms);
        }
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut color = [self.color.x, self.color.y, self.color.z];
        let changed = polyscope_ui::build_vector_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.length_scale,
            &mut self.radius,
            &mut color,
        );
        if changed {
            self.color = Vec4::new(color[0], color[1], color[2], self.color.w);
        }
        changed
    }
}

impl Quantity for MeshOneFormQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn structure_name(&self) -> &str {
        &self.structure_name
    }
    fn kind(&self) -> QuantityKind {
        QuantityKind::Vector
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
    }
    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl EdgeQuantity for MeshOneFormQuantity {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_form_creation() {
        let values = vec![1.0, -0.5, 0.3];
        let orientations = vec![true, true, false];
        let q = MeshOneFormQuantity::new("flow", "mesh", values, orientations);

        assert_eq!(q.name(), "flow");
        assert_eq!(q.structure_name(), "mesh");
        assert_eq!(q.data_size(), 3);
        assert_eq!(q.kind(), QuantityKind::Vector);
        assert!(!q.is_enabled());
    }

    #[test]
    fn test_edge_vector_computation() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0), // v0
            Vec3::new(2.0, 0.0, 0.0), // v1
            Vec3::new(1.0, 2.0, 0.0), // v2
        ];
        // Edges: (0,1), (0,2), (1,2) - sorted
        let edges: Vec<(u32, u32)> = vec![(0, 1), (0, 2), (1, 2)];

        // All orientations match canonical direction
        let values = vec![1.0, 0.5, -0.5];
        let orientations = vec![true, true, true];
        let q = MeshOneFormQuantity::new("test", "mesh", values, orientations);

        let (positions, vectors) = q.compute_edge_vectors(&vertices, &edges);

        assert_eq!(positions.len(), 3);
        assert_eq!(vectors.len(), 3);

        // Edge (0,1): midpoint = (1, 0, 0), direction = +X, value = 1.0
        assert!((positions[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
        assert!((vectors[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);

        // Edge (0,2): midpoint = (0.5, 1, 0), value = 0.5
        assert!((positions[1] - Vec3::new(0.5, 1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_edge_vector_orientation_flip() {
        let vertices = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)];
        let edges: Vec<(u32, u32)> = vec![(0, 1)];

        // Orientation = false means flip direction
        let values = vec![1.0];
        let orientations = vec![false];
        let q = MeshOneFormQuantity::new("test", "mesh", values, orientations);

        let (positions, vectors) = q.compute_edge_vectors(&vertices, &edges);

        // Edge (0,1): canonical is +X, but orientation=false flips to -X
        assert!((positions[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
        assert!((vectors[0] - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_edge_vector_negative_value() {
        let vertices = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)];
        let edges: Vec<(u32, u32)> = vec![(0, 1)];

        // Negative value with default orientation
        let values = vec![-1.0];
        let orientations = vec![true];
        let q = MeshOneFormQuantity::new("test", "mesh", values, orientations);

        let (_positions, vectors) = q.compute_edge_vectors(&vertices, &edges);

        // Negative value: direction is +X, value is -1.0, so vector points -X
        assert!((vectors[0] - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5);
    }
}
