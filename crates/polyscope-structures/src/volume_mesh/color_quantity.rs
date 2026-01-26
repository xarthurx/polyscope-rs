//! Color quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A color quantity defined at mesh vertices.
pub struct VolumeMeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
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
            colors,
            enabled: false,
        }
    }

    pub fn colors(&self) -> &[Vec3] {
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
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.colors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl VertexQuantity for VolumeMeshVertexColorQuantity {}

/// A color quantity defined at mesh cells.
pub struct VolumeMeshCellColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
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
            colors,
            enabled: false,
        }
    }

    pub fn colors(&self) -> &[Vec3] {
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
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.colors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl CellQuantity for VolumeMeshCellColorQuantity {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_color_quantity() {
        let colors = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let quantity = VolumeMeshVertexColorQuantity::new("colors", "mesh", colors.clone());

        assert_eq!(quantity.name(), "colors");
        assert_eq!(quantity.colors().len(), 2);
    }

    #[test]
    fn test_cell_color_quantity() {
        let colors = vec![Vec3::new(0.0, 0.0, 1.0)];
        let quantity = VolumeMeshCellColorQuantity::new("cell_colors", "mesh", colors.clone());

        assert_eq!(quantity.name(), "cell_colors");
        assert_eq!(quantity.colors().len(), 1);
    }
}
