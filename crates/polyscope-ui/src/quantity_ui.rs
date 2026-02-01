//! Quantity-specific UI builders.

use egui::Ui;
use glam::Vec3;
use polyscope_core::quantity::ParamVizStyle;

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
                ui.label("–");
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
        ui.label(format!("({num_colors} colors)"));
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
            egui::Grid::new(format!("{name}_vector_grid"))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Length:");
                    if ui
                        .add(
                            egui::DragValue::new(length_scale)
                                .speed(0.001)
                                .range(0.0001..=10.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Radius:");
                    if ui
                        .add(
                            egui::DragValue::new(radius)
                                .speed(0.00001)
                                .range(0.00001..=0.1),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Color:");
                    if ui.color_edit_button_rgb(color).changed() {
                        changed = true;
                    }
                    ui.end_row();
                });
        });
    }

    changed
}

/// Builds UI for a parameterization quantity.
pub fn build_parameterization_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    style: &mut ParamVizStyle,
    checker_size: &mut f32,
    checker_colors: &mut [Vec3; 2],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            // Style selector
            let style_label = match style {
                ParamVizStyle::Checker => "Checker",
                ParamVizStyle::Grid => "Grid",
                ParamVizStyle::LocalCheck => "LocalCheck",
                ParamVizStyle::LocalRad => "LocalRad",
            };
            egui::ComboBox::from_label("Style")
                .selected_text(style_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(style, ParamVizStyle::Checker, "Checker")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(style, ParamVizStyle::Grid, "Grid")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(style, ParamVizStyle::LocalCheck, "LocalCheck")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(style, ParamVizStyle::LocalRad, "LocalRad")
                        .changed()
                    {
                        changed = true;
                    }
                });

            egui::Grid::new(format!("{name}_param_grid"))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Checker size:");
                    if ui
                        .add(
                            egui::DragValue::new(checker_size)
                                .speed(0.005)
                                .range(0.001..=10.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Color 1:");
                    let mut c = [
                        checker_colors[0].x,
                        checker_colors[0].y,
                        checker_colors[0].z,
                    ];
                    if ui.color_edit_button_rgb(&mut c).changed() {
                        checker_colors[0] = Vec3::new(c[0], c[1], c[2]);
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Color 2:");
                    let mut c = [
                        checker_colors[1].x,
                        checker_colors[1].y,
                        checker_colors[1].z,
                    ];
                    if ui.color_edit_button_rgb(&mut c).changed() {
                        checker_colors[1] = Vec3::new(c[0], c[1], c[2]);
                        changed = true;
                    }
                    ui.end_row();
                });
        });
    }

    changed
}

/// Builds UI for an intrinsic vector quantity.
pub fn build_intrinsic_vector_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    length_scale: &mut f32,
    radius: &mut f32,
    color: &mut [f32; 3],
    n_sym: &mut u32,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.checkbox(enabled, name).changed() {
            changed = true;
        }
    });

    if *enabled {
        ui.indent(name, |ui| {
            egui::Grid::new(format!("{name}_intrinsic_grid"))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Length:");
                    if ui
                        .add(
                            egui::DragValue::new(length_scale)
                                .speed(0.001)
                                .range(0.0001..=10.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Radius:");
                    if ui
                        .add(
                            egui::DragValue::new(radius)
                                .speed(0.00001)
                                .range(0.00001..=0.1),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Color:");
                    if ui.color_edit_button_rgb(color).changed() {
                        changed = true;
                    }
                    ui.end_row();

                    // Symmetry control
                    ui.label("Symmetry:");
                    let sym_label = match *n_sym {
                        1 => "Vector (1)",
                        2 => "Line (2)",
                        4 => "Cross (4)",
                        _ => "Custom",
                    };
                    egui::ComboBox::from_id_salt(format!("{name}_sym"))
                        .selected_text(sym_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(n_sym, 1, "Vector (1)").changed() {
                                changed = true;
                            }
                            if ui.selectable_value(n_sym, 2, "Line (2)").changed() {
                                changed = true;
                            }
                            if ui.selectable_value(n_sym, 4, "Cross (4)").changed() {
                                changed = true;
                            }
                        });
                    ui.end_row();
                });
        });
    }

    changed
}
