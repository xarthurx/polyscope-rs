//! Color quantities for volume meshes.

use glam::{Vec3, Vec4};
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A color quantity defined at mesh vertices.
pub struct VolumeMeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec4>,
    enabled: bool,
}

impl VolumeMeshVertexColorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors: colors.into_iter().map(|c| c.extend(1.0)).collect(),
            enabled: false,
        }
    }

    #[must_use]
    pub fn colors(&self) -> &[Vec4] {
        &self.colors
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(vertex color)");
        });
    }
}

impl Quantity for VolumeMeshVertexColorQuantity {
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
    fn data_size(&self) -> usize {
        self.colors.len()
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

impl VertexQuantity for VolumeMeshVertexColorQuantity {}

/// A color quantity defined at mesh cells.
pub struct VolumeMeshCellColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec4>,
    enabled: bool,
}

impl VolumeMeshCellColorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors: colors.into_iter().map(|c| c.extend(1.0)).collect(),
            enabled: false,
        }
    }

    #[must_use]
    pub fn colors(&self) -> &[Vec4] {
        &self.colors
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(cell color)");
        });
    }
}

impl Quantity for VolumeMeshCellColorQuantity {
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
    fn data_size(&self) -> usize {
        self.colors.len()
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

impl CellQuantity for VolumeMeshCellColorQuantity {}
