//! Surface mesh quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{FaceQuantity, Quantity, QuantityKind, VertexQuantity};
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
    /// Creates a new vertex scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

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

    /// Returns the scalar values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the colormap name.
    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    /// Sets the colormap name.
    pub fn set_colormap(&mut self, name: impl Into<String>) {
        self.colormap_name = name.into();
    }

    /// Gets the range minimum.
    pub fn range_min(&self) -> f32 {
        self.range_min
    }

    /// Gets the range maximum.
    pub fn range_max(&self) -> f32 {
        self.range_max
    }

    /// Sets the range.
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.range_min = min;
        self.range_max = max;
    }

    /// Maps scalar values to colors using the colormap.
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

    /// Builds the egui UI for this quantity.
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
        QuantityKind::Scalar
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
        self.values.len()
    }
}

impl VertexQuantity for MeshVertexScalarQuantity {}

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
    /// Creates a new face scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

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

    /// Returns the scalar values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the colormap name.
    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    /// Sets the colormap name.
    pub fn set_colormap(&mut self, name: impl Into<String>) {
        self.colormap_name = name.into();
    }

    /// Gets the range minimum.
    pub fn range_min(&self) -> f32 {
        self.range_min
    }

    /// Gets the range maximum.
    pub fn range_max(&self) -> f32 {
        self.range_max
    }

    /// Sets the range.
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.range_min = min;
        self.range_max = max;
    }

    /// Computes vertex colors by expanding face values to all vertices of each face.
    /// For each vertex, uses the color of the last face it belongs to.
    pub fn compute_vertex_colors(
        &self,
        faces: &[Vec<u32>],
        num_vertices: usize,
        colormap: &ColorMap,
    ) -> Vec<Vec3> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        let mut colors = vec![Vec3::splat(0.5); num_vertices];

        for (face_idx, face) in faces.iter().enumerate() {
            let t = (self.values[face_idx] - self.range_min) / range;
            let color = colormap.sample(t);
            for &vi in face {
                colors[vi as usize] = color;
            }
        }

        colors
    }

    /// Builds the egui UI for this quantity.
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

impl Quantity for MeshFaceScalarQuantity {
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
        QuantityKind::Scalar
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
        self.values.len()
    }
}

impl FaceQuantity for MeshFaceScalarQuantity {}

/// A vertex color quantity on a surface mesh.
pub struct MeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl MeshVertexColorQuantity {
    /// Creates a new vertex color quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
        }
    }

    /// Returns the colors.
    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for MeshVertexColorQuantity {
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
        QuantityKind::Color
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
        self.colors.len()
    }
}

impl VertexQuantity for MeshVertexColorQuantity {}

/// A face color quantity on a surface mesh.
pub struct MeshFaceColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl MeshFaceColorQuantity {
    /// Creates a new face color quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
        }
    }

    /// Returns the colors.
    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    /// Computes vertex colors by expanding face colors to all vertices of each face.
    /// For each vertex, uses the color of the last face it belongs to.
    pub fn compute_vertex_colors(&self, faces: &[Vec<u32>], num_vertices: usize) -> Vec<Vec3> {
        let mut colors = vec![Vec3::splat(0.5); num_vertices];

        for (face_idx, face) in faces.iter().enumerate() {
            let color = self.colors[face_idx];
            for &vi in face {
                colors[vi as usize] = color;
            }
        }

        colors
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for MeshFaceColorQuantity {
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
        QuantityKind::Color
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
        self.colors.len()
    }
}

impl FaceQuantity for MeshFaceColorQuantity {}

/// A vertex vector quantity on a surface mesh.
pub struct MeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
}

impl MeshVertexVectorQuantity {
    /// Creates a new vertex vector quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            enabled: false,
            length_scale: 1.0,
            radius: 0.005,
            color: Vec3::new(0.8, 0.2, 0.2),
        }
    }

    /// Returns the vectors.
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    /// Gets the length scale.
    pub fn length_scale(&self) -> f32 {
        self.length_scale
    }

    /// Sets the length scale.
    pub fn set_length_scale(&mut self, scale: f32) {
        self.length_scale = scale;
    }

    /// Gets the radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, r: f32) {
        self.radius = r;
    }

    /// Gets the color.
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) {
        self.color = c;
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
            self.color = Vec3::new(color[0], color[1], color[2]);
        }
        changed
    }
}

impl Quantity for MeshVertexVectorQuantity {
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

impl VertexQuantity for MeshVertexVectorQuantity {}
