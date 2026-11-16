//! Scalar quantities for volume meshes.

use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A scalar quantity defined at mesh vertices.
pub struct VolumeMeshVertexScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    color_map: String,
    data_min: f32,
    data_max: f32,
}

impl VolumeMeshVertexScalarQuantity {
    /// Creates a new vertex scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the color map name.
    #[must_use]
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self
    }

    /// Gets the data range.
    #[must_use]
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Builds the egui UI for this quantity: enable checkbox, color map and range.
    ///
    /// Returns true if a setting changed.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_scalar_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.color_map,
            &mut self.data_min,
            &mut self.data_max,
            &polyscope_render::BUILTIN_COLOR_MAP_NAMES,
        )
    }
}

impl Quantity for VolumeMeshVertexScalarQuantity {
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
    fn data_size(&self) -> usize {
        self.values.len()
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl VertexQuantity for VolumeMeshVertexScalarQuantity {}

/// A scalar quantity defined at mesh cells.
pub struct VolumeMeshCellScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    color_map: String,
    data_min: f32,
    data_max: f32,
}

impl VolumeMeshCellScalarQuantity {
    /// Creates a new cell scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the color map name.
    #[must_use]
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self
    }

    /// Gets the data range.
    #[must_use]
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Builds the egui UI for this quantity: enable checkbox, color map and range.
    ///
    /// Returns true if a setting changed.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_scalar_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.color_map,
            &mut self.data_min,
            &mut self.data_max,
            &polyscope_render::BUILTIN_COLOR_MAP_NAMES,
        )
    }
}

impl Quantity for VolumeMeshCellScalarQuantity {
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
    fn data_size(&self) -> usize {
        self.values.len()
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl CellQuantity for VolumeMeshCellScalarQuantity {}
