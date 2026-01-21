//! Quantity-specific UI builders.

use egui::Ui;

/// Builds UI for a scalar quantity.
pub fn build_scalar_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    colormap: &mut String,
    range_min: &mut f32,
    range_max: &mut f32,
    available_colormaps: &[&str],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            // Colormap selector
            egui::ComboBox::from_label("Colormap")
                .selected_text(colormap.as_str())
                .show_ui(ui, |ui| {
                    for &cmap in available_colormaps {
                        if ui
                            .selectable_value(colormap, cmap.to_string(), cmap)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });

            // Range controls
            ui.horizontal(|ui| {
                ui.label("Range:");
                if ui
                    .add(egui::DragValue::new(range_min).speed(0.01))
                    .changed()
                {
                    changed = true;
                }
                ui.label("to");
                if ui
                    .add(egui::DragValue::new(range_max).speed(0.01))
                    .changed()
                {
                    changed = true;
                }
            });
        });
    }

    changed
}

/// Builds UI for a color quantity.
pub fn build_color_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    num_colors: usize,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
        ui.label(format!("({} colors)", num_colors));
    });

    changed
}

/// Builds UI for a vector quantity.
pub fn build_vector_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    length_scale: &mut f32,
    radius: &mut f32,
    color: &mut [f32; 3],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            ui.horizontal(|ui| {
                ui.label("Length:");
                if ui
                    .add(
                        egui::DragValue::new(length_scale)
                            .speed(0.01)
                            .range(0.01..=5.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui
                    .add(egui::DragValue::new(radius).speed(0.001).range(0.001..=0.1))
                    .changed()
                {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_rgb(color).changed() {
                    changed = true;
                }
            });
        });
    }

    changed
}
