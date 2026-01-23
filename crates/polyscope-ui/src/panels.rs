//! UI panel builders.

use egui::{CollapsingHeader, Context, DragValue, SidePanel, Slider, Ui};

/// Camera settings exposed in UI.
#[derive(Debug, Clone)]
pub struct CameraSettings {
    /// Navigation style (0=Turntable, 1=Free, 2=Planar, 3=FirstPerson, 4=None)
    pub navigation_style: u32,
    /// Projection mode (0=Perspective, 1=Orthographic)
    pub projection_mode: u32,
    /// Up direction (0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z)
    pub up_direction: u32,
    /// Front direction
    pub front_direction: u32,
    /// Field of view in degrees
    pub fov_degrees: f32,
    /// Near clip plane
    pub near: f32,
    /// Far clip plane
    pub far: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Orthographic scale
    pub ortho_scale: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            navigation_style: 0, // Turntable
            projection_mode: 0,  // Perspective
            up_direction: 2,     // +Y
            front_direction: 5,  // -Z
            fov_degrees: 45.0,
            near: 0.01,
            far: 1000.0,
            move_speed: 1.0,
            ortho_scale: 1.0,
        }
    }
}

/// Scene extents information for UI display.
#[derive(Debug, Clone, Default)]
pub struct SceneExtents {
    /// Whether to auto-compute extents.
    pub auto_compute: bool,
    /// Length scale of the scene.
    pub length_scale: f32,
    /// Bounding box minimum.
    pub bbox_min: [f32; 3],
    /// Bounding box maximum.
    pub bbox_max: [f32; 3],
}

/// Appearance settings for UI.
#[derive(Debug, Clone)]
pub struct AppearanceSettings {
    /// Transparency mode (0=None, 1=Simple, 2=WeightedBlended)
    pub transparency_mode: u32,
    /// SSAA factor (1, 2, or 4)
    pub ssaa_factor: u32,
    /// Max FPS (0 = unlimited)
    pub max_fps: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            transparency_mode: 1, // Simple
            ssaa_factor: 1,
            max_fps: 60,
        }
    }
}

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

/// Builds the camera settings section.
/// Returns true if any setting changed.
pub fn build_camera_settings_section(ui: &mut Ui, settings: &mut CameraSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Camera")
        .default_open(false)
        .show(ui, |ui| {
            // Navigation style
            egui::ComboBox::from_label("Navigation")
                .selected_text(match settings.navigation_style {
                    0 => "Turntable",
                    1 => "Free",
                    2 => "Planar",
                    3 => "First Person",
                    _ => "None",
                })
                .show_ui(ui, |ui| {
                    for (i, name) in ["Turntable", "Free", "Planar", "First Person", "None"].iter().enumerate() {
                        if ui.selectable_value(&mut settings.navigation_style, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            // Projection mode
            egui::ComboBox::from_label("Projection")
                .selected_text(if settings.projection_mode == 0 { "Perspective" } else { "Orthographic" })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.projection_mode, 0, "Perspective").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.projection_mode, 1, "Orthographic").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // Up direction
            let directions = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];
            egui::ComboBox::from_label("Up")
                .selected_text(directions[settings.up_direction as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in directions.iter().enumerate() {
                        if ui.selectable_value(&mut settings.up_direction, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            // Front direction
            egui::ComboBox::from_label("Front")
                .selected_text(directions[settings.front_direction as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in directions.iter().enumerate() {
                        if ui.selectable_value(&mut settings.front_direction, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            ui.separator();

            // FOV (only for perspective)
            if settings.projection_mode == 0 {
                ui.horizontal(|ui| {
                    ui.label("FOV:");
                    if ui.add(Slider::new(&mut settings.fov_degrees, 10.0..=170.0).suffix("°")).changed() {
                        changed = true;
                    }
                });
            } else {
                // Ortho scale
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    if ui.add(DragValue::new(&mut settings.ortho_scale).speed(0.1).range(0.1..=100.0)).changed() {
                        changed = true;
                    }
                });
            }

            // Clip planes
            ui.horizontal(|ui| {
                ui.label("Near:");
                if ui.add(DragValue::new(&mut settings.near).speed(0.001).range(0.001..=10.0)).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Far:");
                if ui.add(DragValue::new(&mut settings.far).speed(1.0).range(10.0..=10000.0)).changed() {
                    changed = true;
                }
            });

            // Move speed
            ui.horizontal(|ui| {
                ui.label("Move Speed:");
                if ui.add(DragValue::new(&mut settings.move_speed).speed(0.1).range(0.1..=10.0)).changed() {
                    changed = true;
                }
            });
        });

    changed
}

/// Builds the scene extents section.
/// Returns true if auto_compute changed.
pub fn build_scene_extents_section(ui: &mut Ui, extents: &mut SceneExtents) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Scene Extents")
        .default_open(false)
        .show(ui, |ui| {
            if ui.checkbox(&mut extents.auto_compute, "Auto-compute").changed() {
                changed = true;
            }

            ui.separator();

            // Display length scale (read-only)
            ui.horizontal(|ui| {
                ui.label("Length scale:");
                ui.label(format!("{:.4}", extents.length_scale));
            });

            // Display bounding box (read-only)
            ui.label("Bounding box:");
            ui.indent("bbox", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Min:");
                    ui.label(format!(
                        "({:.2}, {:.2}, {:.2})",
                        extents.bbox_min[0], extents.bbox_min[1], extents.bbox_min[2]
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Max:");
                    ui.label(format!(
                        "({:.2}, {:.2}, {:.2})",
                        extents.bbox_max[0], extents.bbox_max[1], extents.bbox_max[2]
                    ));
                });
            });

            // Compute center and size
            let center = [
                (extents.bbox_min[0] + extents.bbox_max[0]) / 2.0,
                (extents.bbox_min[1] + extents.bbox_max[1]) / 2.0,
                (extents.bbox_min[2] + extents.bbox_max[2]) / 2.0,
            ];
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.label(format!("({:.2}, {:.2}, {:.2})", center[0], center[1], center[2]));
            });
        });

    changed
}

/// Builds the appearance settings section.
/// Returns true if any setting changed.
pub fn build_appearance_section(ui: &mut Ui, settings: &mut AppearanceSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Appearance")
        .default_open(false)
        .show(ui, |ui| {
            // Transparency mode
            egui::ComboBox::from_label("Transparency")
                .selected_text(match settings.transparency_mode {
                    0 => "None",
                    1 => "Simple",
                    _ => "Weighted Blended",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.transparency_mode, 0, "None").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.transparency_mode, 1, "Simple").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.transparency_mode, 2, "Weighted Blended").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // SSAA factor
            egui::ComboBox::from_label("Anti-aliasing")
                .selected_text(format!("{}x SSAA", settings.ssaa_factor))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.ssaa_factor, 1, "1x (Off)").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.ssaa_factor, 2, "2x SSAA").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.ssaa_factor, 4, "4x SSAA").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // Max FPS
            ui.horizontal(|ui| {
                ui.label("Max FPS:");
                let mut fps = settings.max_fps as i32;
                if ui.add(DragValue::new(&mut fps).range(0..=240)).changed() {
                    settings.max_fps = fps.max(0) as u32;
                    changed = true;
                }
                if settings.max_fps == 0 {
                    ui.label("(unlimited)");
                }
            });
        });

    changed
}

/// Builds the ground plane settings section.
pub fn build_ground_plane_section(
    ui: &mut Ui,
    mode: &mut u32, // 0=None, 1=Tile
    height: &mut f32,
    height_is_relative: &mut bool,
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
