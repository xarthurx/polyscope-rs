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
