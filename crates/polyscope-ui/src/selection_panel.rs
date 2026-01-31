//! Selection/pick results panel.

use egui::{Context, SidePanel, Ui};
use polyscope_render::{PickElementType, PickResult};

/// Builds the selection panel on the right side.
/// Only shows if there is an active selection.
pub fn build_selection_panel(
    ctx: &Context,
    selection: &PickResult,
    build_structure_pick_ui: impl FnOnce(&mut Ui),
) {
    SidePanel::right("selection_panel")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Selection");
            ui.separator();

            ui.label(format!(
                "Screen: ({:.0}, {:.0})",
                selection.screen_pos.x, selection.screen_pos.y
            ));
            ui.label(format!("Depth: {:.4}", selection.depth));

            ui.separator();
            ui.label(format!(
                "{}: {}",
                selection.structure_type, selection.structure_name
            ));

            // Show element type and index
            let element_type_str = match selection.element_type {
                PickElementType::None => "Element",
                PickElementType::Point => "Point",
                PickElementType::Vertex => "Vertex",
                PickElementType::Face => "Face",
                PickElementType::Edge => "Edge",
                PickElementType::Cell => "Cell",
            };
            ui.label(format!("{} #{}", element_type_str, selection.element_index));

            ui.separator();
            build_structure_pick_ui(ui);
        });
}
