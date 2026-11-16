//! Quantity list of the volume mesh panel.

use super::{
    VolumeMesh, VolumeMeshCellColorQuantity, VolumeMeshCellScalarQuantity,
    VolumeMeshCellVectorQuantity, VolumeMeshVertexColorQuantity, VolumeMeshVertexScalarQuantity,
    VolumeMeshVertexVectorQuantity,
};

impl VolumeMesh {
    /// Builds the quantity list of the volume mesh panel.
    ///
    /// Each quantity gets an enable checkbox and the controls of the matching
    /// surface mesh quantity. If a scalar or color quantity becomes enabled,
    /// the other scalar and color quantities become disabled.
    pub(super) fn build_quantities_ui(&mut self, ui: &mut egui::Ui) {
        if self.quantities.is_empty() {
            return;
        }
        ui.separator();
        ui.label("Quantities:");

        let mut newly_enabled = None;
        for (index, quantity) in self.quantities.iter_mut().enumerate() {
            let was_enabled = quantity.is_enabled();
            let q = quantity.as_any_mut();
            if let Some(sq) = q.downcast_mut::<VolumeMeshVertexScalarQuantity>() {
                sq.build_egui_ui(ui);
            } else if let Some(sq) = q.downcast_mut::<VolumeMeshCellScalarQuantity>() {
                sq.build_egui_ui(ui);
            } else if let Some(cq) = q.downcast_mut::<VolumeMeshVertexColorQuantity>() {
                cq.build_egui_ui(ui);
            } else if let Some(cq) = q.downcast_mut::<VolumeMeshCellColorQuantity>() {
                cq.build_egui_ui(ui);
            } else if let Some(vq) = q.downcast_mut::<VolumeMeshVertexVectorQuantity>() {
                vq.build_egui_ui(ui);
            } else if let Some(vq) = q.downcast_mut::<VolumeMeshCellVectorQuantity>() {
                vq.build_egui_ui(ui);
            }
            if !was_enabled && quantity.is_enabled() {
                newly_enabled = Some(index);
            }
        }
        if let Some(index) = newly_enabled {
            self.disable_other_color_quantities(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn test_panel_lists_every_quantity_type() {
        let vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let mut mesh = VolumeMesh::new_tet_mesh("tet", vertices, vec![[0, 1, 2, 3]]);
        mesh.add_vertex_scalar_quantity("vs", vec![0.0; 4]);
        mesh.add_cell_scalar_quantity("cs", vec![0.0]);
        mesh.add_vertex_color_quantity("vc", vec![Vec3::ONE; 4]);
        mesh.add_cell_color_quantity("cc", vec![Vec3::ONE]);
        mesh.add_vertex_vector_quantity("vv", vec![Vec3::X; 4]);
        mesh.add_cell_vector_quantity("cv", vec![Vec3::X]);
        for name in ["vs", "cs", "vc", "cc", "vv", "cv"] {
            mesh.set_quantity_enabled(name, true);
        }

        // Build the panel twice in a headless egui context: it must not panic.
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                mesh.build_egui_ui(ui, &[]);
            });
            // No renderer consumes the font textures here.
            output.textures_delta.clear();
        }
    }
}
