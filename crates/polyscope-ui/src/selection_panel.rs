//! Selection/pick results panel.

use egui::{Context, Panel, Ui};
use polyscope_render::{PickElementType, PickResult};

/// Builds the selection panel on the right side.
/// Only shows if there is an active selection.
pub fn build_selection_panel(
    ctx: &Context,
    selection: &PickResult,
    build_structure_pick_ui: impl FnOnce(&mut Ui),
) {
    // egui no longer shows panels on a `Context`. Host the panel in a top-level
    // background `Ui`, as `SidePanel::show(ctx)` did before.
    let mut root_ui = Ui::new(
        ctx.clone(),
        egui::Id::new("selection_panel"),
        egui::UiBuilder::new().layer_id(egui::LayerId::background()),
    );
    root_ui
        .response()
        .widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Panel));
    Panel::right("selection_panel")
        .default_size(300.0)
        .show(&mut root_ui, |ui| {
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
