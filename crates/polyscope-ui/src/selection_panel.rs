//! Selection/pick results panel.

use egui::{Context, SidePanel, Ui};
use polyscope_render::PickResult;

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
            ui.label(format!("Element #{}", selection.element_index));

            ui.separator();
            build_structure_pick_ui(ui);
        });
}
