//! Surface mesh quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind, VertexQuantity};
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
