//! Point cloud quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind, VertexQuantity};
use polyscope_render::{ColorMap, PointCloudRenderData};

/// A scalar quantity on a point cloud.
pub struct PointCloudScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl PointCloudScalarQuantity {
    /// Creates a new scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max = values
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

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
}

impl Quantity for PointCloudScalarQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl VertexQuantity for PointCloudScalarQuantity {}

/// A vector quantity on a point cloud.
pub struct PointCloudVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    // TODO: Add scale, color, style, GPU resources
}

impl PointCloudVectorQuantity {
    /// Creates a new vector quantity.
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
        }
    }

    /// Returns the vectors.
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }
}

impl Quantity for PointCloudVectorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl VertexQuantity for PointCloudVectorQuantity {}

/// A color quantity on a point cloud.
pub struct PointCloudColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl PointCloudColorQuantity {
    /// Creates a new color quantity.
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

    /// Applies this color quantity to the point cloud render data.
    pub fn apply_to_render_data(&self, queue: &wgpu::Queue, render_data: &PointCloudRenderData) {
        render_data.update_colors(queue, &self.colors);
    }
}

impl Quantity for PointCloudColorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl VertexQuantity for PointCloudColorQuantity {}
