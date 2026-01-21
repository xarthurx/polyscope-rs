//! Structure-specific UI builders.

use egui::Ui;

/// Builds UI for a point cloud.
pub fn build_point_cloud_ui(
    ui: &mut Ui,
    num_points: usize,
    point_radius: &mut f32,
    base_color: &mut [f32; 3],
) -> bool {
    let mut changed = false;

    ui.label(format!("Points: {num_points}"));

    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(base_color).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Radius:");
        if ui
            .add(
                egui::DragValue::new(point_radius)
                    .speed(0.001)
                    .range(0.001..=0.5),
            )
            .changed()
        {
            changed = true;
        }
    });

    changed
}
