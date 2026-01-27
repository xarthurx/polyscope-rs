//! Intrinsic (tangent-space) vector quantities for surface meshes.

use glam::{Vec2, Vec3};
use polyscope_core::quantity::{FaceQuantity, Quantity, QuantityKind, VertexQuantity};

/// A vertex intrinsic vector quantity on a surface mesh.
///
/// Stores 2D tangent-space vectors along with a per-vertex tangent basis.
/// These are projected to 3D world space for rendering.
pub struct MeshVertexIntrinsicVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec2>, // 2D tangent-space vectors
    basis_x: Vec<Vec3>, // Per-element X axis of tangent frame
    basis_y: Vec<Vec3>, // Per-element Y axis of tangent frame
    n_sym: u32,         // Symmetry: 1=vector, 2=line, 4=cross
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
}

impl MeshVertexIntrinsicVectorQuantity {
    /// Creates a new vertex intrinsic vector quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            basis_x,
            basis_y,
            n_sym: 1,
            enabled: false,
            length_scale: 1.0,
            radius: 0.005,
            color: Vec3::new(0.8, 0.2, 0.8),
        }
    }

    /// Returns the 2D tangent-space vectors.
    #[must_use]
    pub fn vectors(&self) -> &[Vec2] {
        &self.vectors
    }

    /// Returns the tangent basis X axes.
    #[must_use]
    pub fn basis_x(&self) -> &[Vec3] {
        &self.basis_x
    }

    /// Returns the tangent basis Y axes.
    #[must_use]
    pub fn basis_y(&self) -> &[Vec3] {
        &self.basis_y
    }

    /// Gets the symmetry order.
    #[must_use]
    pub fn n_sym(&self) -> u32 {
        self.n_sym
    }

    /// Sets the symmetry order (1=vector, 2=line, 4=cross).
    pub fn set_n_sym(&mut self, n: u32) -> &mut Self {
        self.n_sym = n;
        self
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
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) -> &mut Self {
        self.color = c;
        self
    }

    /// Project 2D tangent-space vectors to 3D world space.
    #[must_use]
    pub fn compute_world_vectors(&self) -> Vec<Vec3> {
        self.vectors
            .iter()
            .enumerate()
            .map(|(i, v2d)| self.basis_x[i] * v2d.x + self.basis_y[i] * v2d.y)
            .collect()
    }

    /// Generate symmetry-rotated copies of the world vectors.
    ///
    /// For `n_sym = 1`, returns the original vectors.
    /// For `n_sym = 2`, returns each vector and its negation (line field).
    /// For `n_sym = 4`, returns each vector rotated by 0, 90, 180, 270 degrees in the tangent plane.
    #[must_use]
    pub fn compute_symmetric_world_vectors(&self) -> Vec<(usize, Vec3)> {
        let mut result = Vec::new();
        for (i, v2d) in self.vectors.iter().enumerate() {
            for k in 0..self.n_sym {
                let angle = k as f32 * std::f32::consts::TAU / self.n_sym as f32;
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let rotated =
                    Vec2::new(v2d.x * cos_a - v2d.y * sin_a, v2d.x * sin_a + v2d.y * cos_a);
                let world_vec = self.basis_x[i] * rotated.x + self.basis_y[i] * rotated.y;
                result.push((i, world_vec));
            }
        }
        result
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut color = [self.color.x, self.color.y, self.color.z];
        let changed = polyscope_ui::build_intrinsic_vector_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.length_scale,
            &mut self.radius,
            &mut color,
            &mut self.n_sym,
        );
        if changed {
            self.color = Vec3::new(color[0], color[1], color[2]);
        }
        changed
    }
}

impl Quantity for MeshVertexIntrinsicVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl VertexQuantity for MeshVertexIntrinsicVectorQuantity {}

/// A face intrinsic vector quantity on a surface mesh.
///
/// Stores 2D tangent-space vectors along with a per-face tangent basis.
pub struct MeshFaceIntrinsicVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec2>,
    basis_x: Vec<Vec3>,
    basis_y: Vec<Vec3>,
    n_sym: u32,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
}

impl MeshFaceIntrinsicVectorQuantity {
    /// Creates a new face intrinsic vector quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            basis_x,
            basis_y,
            n_sym: 1,
            enabled: false,
            length_scale: 1.0,
            radius: 0.005,
            color: Vec3::new(0.2, 0.8, 0.8),
        }
    }

    /// Returns the 2D tangent-space vectors.
    #[must_use]
    pub fn vectors(&self) -> &[Vec2] {
        &self.vectors
    }

    /// Returns the tangent basis X axes.
    #[must_use]
    pub fn basis_x(&self) -> &[Vec3] {
        &self.basis_x
    }

    /// Returns the tangent basis Y axes.
    #[must_use]
    pub fn basis_y(&self) -> &[Vec3] {
        &self.basis_y
    }

    /// Gets the symmetry order.
    #[must_use]
    pub fn n_sym(&self) -> u32 {
        self.n_sym
    }

    /// Sets the symmetry order.
    pub fn set_n_sym(&mut self, n: u32) -> &mut Self {
        self.n_sym = n;
        self
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
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) -> &mut Self {
        self.color = c;
        self
    }

    /// Project 2D tangent-space vectors to 3D world space.
    #[must_use]
    pub fn compute_world_vectors(&self) -> Vec<Vec3> {
        self.vectors
            .iter()
            .enumerate()
            .map(|(i, v2d)| self.basis_x[i] * v2d.x + self.basis_y[i] * v2d.y)
            .collect()
    }

    /// Generate symmetry-rotated copies of the world vectors.
    #[must_use]
    pub fn compute_symmetric_world_vectors(&self) -> Vec<(usize, Vec3)> {
        let mut result = Vec::new();
        for (i, v2d) in self.vectors.iter().enumerate() {
            for k in 0..self.n_sym {
                let angle = k as f32 * std::f32::consts::TAU / self.n_sym as f32;
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let rotated =
                    Vec2::new(v2d.x * cos_a - v2d.y * sin_a, v2d.x * sin_a + v2d.y * cos_a);
                let world_vec = self.basis_x[i] * rotated.x + self.basis_y[i] * rotated.y;
                result.push((i, world_vec));
            }
        }
        result
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut color = [self.color.x, self.color.y, self.color.z];
        let changed = polyscope_ui::build_intrinsic_vector_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.length_scale,
            &mut self.radius,
            &mut color,
            &mut self.n_sym,
        );
        if changed {
            self.color = Vec3::new(color[0], color[1], color[2]);
        }
        changed
    }
}

impl Quantity for MeshFaceIntrinsicVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl FaceQuantity for MeshFaceIntrinsicVectorQuantity {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_intrinsic_creation() {
        let vectors = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let basis_x = vec![Vec3::X, Vec3::X];
        let basis_y = vec![Vec3::Y, Vec3::Y];

        let q = MeshVertexIntrinsicVectorQuantity::new(
            "tangent_field",
            "mesh",
            vectors,
            basis_x,
            basis_y,
        );

        assert_eq!(q.name(), "tangent_field");
        assert_eq!(q.data_size(), 2);
        assert_eq!(q.kind(), QuantityKind::Vector);
        assert_eq!(q.n_sym(), 1);
        assert!(!q.is_enabled());
    }

    #[test]
    fn test_vertex_intrinsic_setters() {
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let mut q =
            MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);

        q.set_n_sym(4);
        assert_eq!(q.n_sym(), 4);

        q.set_length_scale(2.0);
        assert_eq!(q.length_scale(), 2.0);

        q.set_radius(0.01);
        assert_eq!(q.radius(), 0.01);

        q.set_color(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(q.color(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_world_vector_projection() {
        // Vector (1, 0) in tangent space with basis X=X, Y=Y -> world vector = X
        let vectors = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let basis_x = vec![Vec3::X, Vec3::X];
        let basis_y = vec![Vec3::Y, Vec3::Y];

        let q = MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);

        let world = q.compute_world_vectors();
        assert_eq!(world.len(), 2);
        assert!((world[0] - Vec3::X).length() < 1e-5);
        assert!((world[1] - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn test_world_vector_projection_rotated_basis() {
        // Vector (1, 0) in tangent space with basis X=Y, Y=Z -> world vector = Y
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::Y];
        let basis_y = vec![Vec3::Z];

        let q = MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);

        let world = q.compute_world_vectors();
        assert!((world[0] - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn test_symmetry_n1() {
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let q = MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);

        let sym = q.compute_symmetric_world_vectors();
        assert_eq!(sym.len(), 1); // 1 vector * 1 symmetry = 1
        assert_eq!(sym[0].0, 0);
        assert!((sym[0].1 - Vec3::X).length() < 1e-5);
    }

    #[test]
    fn test_symmetry_n2() {
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let mut q =
            MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);
        q.set_n_sym(2);

        let sym = q.compute_symmetric_world_vectors();
        assert_eq!(sym.len(), 2); // 1 vector * 2 symmetry = 2
                                  // First copy: original direction (+X)
        assert!((sym[0].1 - Vec3::X).length() < 1e-5);
        // Second copy: 180 degree rotation (-X)
        assert!((sym[1].1 + Vec3::X).length() < 1e-5);
    }

    #[test]
    fn test_symmetry_n4() {
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let mut q =
            MeshVertexIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);
        q.set_n_sym(4);

        let sym = q.compute_symmetric_world_vectors();
        assert_eq!(sym.len(), 4); // 1 vector * 4 symmetry = 4
                                  // 0 deg: +X
        assert!((sym[0].1 - Vec3::X).length() < 1e-5);
        // 90 deg: +Y
        assert!((sym[1].1 - Vec3::Y).length() < 1e-5);
        // 180 deg: -X
        assert!((sym[2].1 + Vec3::X).length() < 1e-5);
        // 270 deg: -Y
        assert!((sym[3].1 + Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn test_face_intrinsic_creation() {
        let vectors = vec![Vec2::new(1.0, 0.0)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let q =
            MeshFaceIntrinsicVectorQuantity::new("face_tangent", "mesh", vectors, basis_x, basis_y);

        assert_eq!(q.name(), "face_tangent");
        assert_eq!(q.data_size(), 1);
        assert_eq!(q.kind(), QuantityKind::Vector);
    }

    #[test]
    fn test_face_intrinsic_world_vectors() {
        let vectors = vec![Vec2::new(0.5, 0.5)];
        let basis_x = vec![Vec3::X];
        let basis_y = vec![Vec3::Y];

        let q = MeshFaceIntrinsicVectorQuantity::new("test", "mesh", vectors, basis_x, basis_y);

        let world = q.compute_world_vectors();
        let expected = Vec3::new(0.5, 0.5, 0.0);
        assert!((world[0] - expected).length() < 1e-5);
    }
}
