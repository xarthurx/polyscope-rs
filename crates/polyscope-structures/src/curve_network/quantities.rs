//! Curve network quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{EdgeQuantity, Quantity, QuantityKind, VertexQuantity};
use polyscope_render::{ColorMap, CurveNetworkRenderData};

/// A scalar quantity on curve network nodes.
pub struct CurveNodeScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl CurveNodeScalarQuantity {
    /// Creates a new node scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

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

    /// Builds the egui UI for this scalar quantity.
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

impl Quantity for CurveNodeScalarQuantity {
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
        // Implemented via build_egui_ui
    }

    fn refresh(&mut self) {
        // GPU resources refreshed externally
    }

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl VertexQuantity for CurveNodeScalarQuantity {}

/// A scalar quantity on curve network edges.
pub struct CurveEdgeScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl CurveEdgeScalarQuantity {
    /// Creates a new edge scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

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

    /// Builds the egui UI for this scalar quantity.
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

impl Quantity for CurveEdgeScalarQuantity {
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
        // Implemented via build_egui_ui
    }

    fn refresh(&mut self) {
        // GPU resources refreshed externally
    }

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl EdgeQuantity for CurveEdgeScalarQuantity {}

/// A color quantity on curve network nodes.
pub struct CurveNodeColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl CurveNodeColorQuantity {
    /// Creates a new node color quantity.
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

    /// Applies this color quantity to the curve network render data.
    pub fn apply_to_render_data(&self, queue: &wgpu::Queue, render_data: &CurveNetworkRenderData) {
        render_data.update_node_colors(queue, &self.colors);
    }

    /// Builds the egui UI for this color quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for CurveNodeColorQuantity {
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
        // Implemented via build_egui_ui
    }

    fn refresh(&mut self) {
        // GPU resources refreshed externally
    }

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl VertexQuantity for CurveNodeColorQuantity {}

/// A color quantity on curve network edges.
pub struct CurveEdgeColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl CurveEdgeColorQuantity {
    /// Creates a new edge color quantity.
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

    /// Applies this color quantity to the curve network render data.
    pub fn apply_to_render_data(&self, queue: &wgpu::Queue, render_data: &CurveNetworkRenderData) {
        render_data.update_edge_colors(queue, &self.colors);
    }

    /// Builds the egui UI for this color quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for CurveEdgeColorQuantity {
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
        // Implemented via build_egui_ui
    }

    fn refresh(&mut self) {
        // GPU resources refreshed externally
    }

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl EdgeQuantity for CurveEdgeColorQuantity {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_scalar_quantity() {
        let values = vec![0.0, 0.5, 1.0];
        let q = CurveNodeScalarQuantity::new("test", "parent", values.clone());

        assert_eq!(q.name(), "test");
        assert_eq!(q.structure_name(), "parent");
        assert_eq!(q.values(), &values);
        assert_eq!(q.range_min(), 0.0);
        assert_eq!(q.range_max(), 1.0);
        assert!(!q.is_enabled());
    }

    #[test]
    fn test_edge_scalar_quantity() {
        let values = vec![1.0, 2.0, 3.0];
        let q = CurveEdgeScalarQuantity::new("edge_scalar", "parent", values.clone());

        assert_eq!(q.name(), "edge_scalar");
        assert_eq!(q.values(), &values);
        assert_eq!(q.range_min(), 1.0);
        assert_eq!(q.range_max(), 3.0);
    }

    #[test]
    fn test_node_color_quantity() {
        let colors = vec![Vec3::X, Vec3::Y, Vec3::Z];
        let q = CurveNodeColorQuantity::new("colors", "parent", colors.clone());

        assert_eq!(q.name(), "colors");
        assert_eq!(q.colors(), &colors);
        assert_eq!(q.data_size(), 3);
    }

    #[test]
    fn test_edge_color_quantity() {
        let colors = vec![Vec3::ONE, Vec3::ZERO];
        let q = CurveEdgeColorQuantity::new("edge_colors", "parent", colors.clone());

        assert_eq!(q.name(), "edge_colors");
        assert_eq!(q.colors(), &colors);
        assert_eq!(q.data_size(), 2);
    }
}
