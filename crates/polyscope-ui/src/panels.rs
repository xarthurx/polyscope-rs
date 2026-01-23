//! UI panel builders.

use egui::{CollapsingHeader, Context, SidePanel, Ui};

/// Builds the main left panel.
pub fn build_left_panel(ctx: &Context, build_contents: impl FnOnce(&mut Ui)) {
    SidePanel::left("polyscope_main_panel")
        .default_width(305.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("polyscope-rs");
            ui.separator();
            build_contents(ui);
        });
}

/// Builds the polyscope controls section.
pub fn build_controls_section(ui: &mut Ui, background_color: &mut [f32; 3]) {
    CollapsingHeader::new("View")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Background:");
                ui.color_edit_button_rgb(background_color);
            });

            if ui.button("Reset View").clicked() {
                // TODO: Reset camera
            }
        });
}

/// Builds the ground plane settings section.
pub fn build_ground_plane_section(
    ui: &mut Ui,
    mode: &mut u32, // 0=None, 1=Tile
    height: &mut f32,
    height_is_relative: &mut bool,
    color1: &mut [f32; 3],
    color2: &mut [f32; 3],
    tile_size: &mut f32,
    transparency: &mut f32,
) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Ground Plane")
        .default_open(false)
        .show(ui, |ui| {
            // Mode selector
            egui::ComboBox::from_label("Mode")
                .selected_text(match *mode {
                    0 => "None",
                    _ => "Tile",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(mode, 0, "None").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 1, "Tile").changed() {
                        changed = true;
                    }
                });

            if *mode > 0 {
                ui.separator();

                // Height settings
                if ui.checkbox(height_is_relative, "Auto height").changed() {
                    changed = true;
                }

                if !*height_is_relative {
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        if ui.add(egui::DragValue::new(height).speed(0.1)).changed() {
                            changed = true;
                        }
                    });
                }

                ui.separator();

                // Colors
                ui.horizontal(|ui| {
                    ui.label("Color 1:");
                    if ui.color_edit_button_rgb(color1).changed() {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Color 2:");
                    if ui.color_edit_button_rgb(color2).changed() {
                        changed = true;
                    }
                });

                // Tile size
                ui.horizontal(|ui| {
                    ui.label("Tile size:");
                    if ui
                        .add(egui::DragValue::new(tile_size).speed(0.1).range(0.1..=100.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                // Transparency (displayed as opacity)
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    let mut opacity = 1.0 - *transparency;
                    if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed() {
                        *transparency = 1.0 - opacity;
                        changed = true;
                    }
                });
            }
        });

    changed
}

/// Builds the structure tree section.
pub fn build_structure_tree<F>(
    ui: &mut Ui,
    structures: &[(String, String, bool)], // (type_name, name, enabled)
    mut on_toggle: F,
) where
    F: FnMut(&str, &str, bool), // (type_name, name, new_enabled)
{
    CollapsingHeader::new("Structures")
        .default_open(true)
        .show(ui, |ui| {
            if structures.is_empty() {
                ui.label("No structures registered");
                return;
            }

            // Group by type
            let mut by_type: std::collections::HashMap<&str, Vec<(&str, bool)>> =
                std::collections::HashMap::new();
            for (type_name, name, enabled) in structures {
                by_type
                    .entry(type_name.as_str())
                    .or_default()
                    .push((name.as_str(), *enabled));
            }

            for (type_name, instances) in &by_type {
                let header = format!("{} ({})", type_name, instances.len());
                CollapsingHeader::new(header)
                    .default_open(instances.len() <= 8)
                    .show(ui, |ui| {
                        for (name, enabled) in instances {
                            let mut enabled_mut = *enabled;
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut enabled_mut, "").changed() {
                                    on_toggle(type_name, name, enabled_mut);
                                }
                                ui.label(*name);
                            });
                        }
                    });
            }
        });
}

/// Builds the structure tree section with per-structure UI support.
///
/// When a structure is expanded, the `build_ui` callback is invoked to build
/// the structure-specific UI (color picker, radius slider, etc.).
pub fn build_structure_tree_with_ui<F, U>(
    ui: &mut Ui,
    structures: &[(String, String, bool)], // (type_name, name, enabled)
    mut on_toggle: F,
    mut build_ui: U,
) where
    F: FnMut(&str, &str, bool),    // (type_name, name, new_enabled)
    U: FnMut(&mut Ui, &str, &str), // (ui, type_name, name)
{
    CollapsingHeader::new("Structures")
        .default_open(true)
        .show(ui, |ui| {
            if structures.is_empty() {
                ui.label("No structures registered");
                return;
            }

            // Group by type
            let mut by_type: std::collections::HashMap<&str, Vec<(&str, bool)>> =
                std::collections::HashMap::new();
            for (type_name, name, enabled) in structures {
                by_type
                    .entry(type_name.as_str())
                    .or_default()
                    .push((name.as_str(), *enabled));
            }

            for (type_name, instances) in &by_type {
                let header = format!("{} ({})", type_name, instances.len());
                CollapsingHeader::new(header)
                    .default_open(instances.len() <= 8)
                    .show(ui, |ui| {
                        for (name, enabled) in instances {
                            let mut enabled_mut = *enabled;

                            // Use a collapsing header for each structure to show its UI
                            let structure_header = CollapsingHeader::new(*name)
                                .default_open(false)
                                .show(ui, |ui| {
                                    // Checkbox for enable/disable
                                    ui.horizontal(|ui| {
                                        ui.label("Enabled:");
                                        if ui.checkbox(&mut enabled_mut, "").changed() {
                                            on_toggle(type_name, name, enabled_mut);
                                        }
                                    });

                                    ui.separator();

                                    // Build structure-specific UI
                                    build_ui(ui, type_name, name);
                                });

                            // body_returned indicates if the collapsing header was expanded
                            // We don't need additional action when collapsed since the header shows the name
                            let _ = structure_header.body_returned;
                        }
                    });
            }
        });
}
