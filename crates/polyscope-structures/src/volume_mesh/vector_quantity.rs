//! Vector quantities for volume meshes.

use glam::{Vec3, Vec4};
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A vector quantity defined at mesh vertices.
pub struct VolumeMeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec4,
}

impl VolumeMeshVertexVectorQuantity {
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
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec4::new(0.1, 0.1, 0.8, 1.0),
        }
    }

    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color.extend(1.0);
        self
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(vertex vector)");
        });

        if self.enabled {
            ui.horizontal(|ui| {
                ui.label("Length:");
                ui.add(
                    egui::DragValue::new(&mut self.vector_length_scale)
                        .speed(0.01)
                        .range(0.001..=10.0),
                );
            });
        }
    }
}

impl Quantity for VolumeMeshVertexVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
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

impl VertexQuantity for VolumeMeshVertexVectorQuantity {}

/// A vector quantity defined at mesh cells.
pub struct VolumeMeshCellVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec4,
}

impl VolumeMeshCellVectorQuantity {
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
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec4::new(0.1, 0.1, 0.8, 1.0),
        }
    }

    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color.extend(1.0);
        self
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(cell vector)");
        });

        if self.enabled {
            ui.horizontal(|ui| {
                ui.label("Length:");
                ui.add(
                    egui::DragValue::new(&mut self.vector_length_scale)
                        .speed(0.01)
                        .range(0.001..=10.0),
                );
            });
        }
    }
}

impl Quantity for VolumeMeshCellVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
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

impl CellQuantity for VolumeMeshCellVectorQuantity {}
