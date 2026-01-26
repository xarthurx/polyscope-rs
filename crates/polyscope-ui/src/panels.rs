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
    /// Note: Front direction is automatically derived using right-hand coordinate conventions.
    pub up_direction: u32,
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
            up_direction: 2,     // +Y (front direction auto-derived as -Z)
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
    /// SSAO enabled
    pub ssao_enabled: bool,
    /// SSAO radius (world units)
    pub ssao_radius: f32,
    /// SSAO intensity
    pub ssao_intensity: f32,
    /// SSAO bias
    pub ssao_bias: f32,
    /// SSAO sample count
    pub ssao_sample_count: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            transparency_mode: 1, // Simple
            ssaa_factor: 1,
            max_fps: 60,
            ssao_enabled: false,
            ssao_radius: 0.5,
            ssao_intensity: 1.5,
            ssao_bias: 0.025,
            ssao_sample_count: 32,
        }
    }
}

/// Tone mapping settings for UI.
#[derive(Debug, Clone)]
pub struct ToneMappingSettings {
    /// Whether tone mapping is enabled.
    pub enabled: bool,
    /// Exposure value (0.1 - 4.0).
    pub exposure: f32,
    /// White level (0.5 - 4.0).
    pub white_level: f32,
    /// Gamma value (1.0 - 3.0).
    pub gamma: f32,
}

impl Default for ToneMappingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 1.0,
            white_level: 1.0,
            gamma: 2.2,
        }
    }
}

/// Settings for a single slice plane in the UI.
#[derive(Debug, Clone)]
pub struct SlicePlaneSettings {
    /// Name of the slice plane.
    pub name: String,
    /// Whether the slice plane is enabled.
    pub enabled: bool,
    /// Origin point (x, y, z).
    pub origin: [f32; 3],
    /// Normal direction (x, y, z).
    pub normal: [f32; 3],
    /// Whether to draw the plane visualization.
    pub draw_plane: bool,
    /// Whether to draw the widget.
    pub draw_widget: bool,
    /// Color of the plane (r, g, b).
    pub color: [f32; 3],
    /// Transparency (0.0 = transparent, 1.0 = opaque).
    pub transparency: f32,
    /// Size of the plane visualization (half-extent in each direction).
    pub plane_size: f32,
    /// Whether this plane is currently selected for gizmo manipulation.
    pub is_selected: bool,
}

impl Default for SlicePlaneSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            draw_plane: true,
            draw_widget: true,
            color: [0.5, 0.5, 0.5],
            transparency: 0.3,
            plane_size: 0.1,
            is_selected: false,
        }
    }
}

impl SlicePlaneSettings {
    /// Creates settings with a name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Selection info for slice plane gizmo manipulation.
#[derive(Debug, Clone, Default)]
pub struct SlicePlaneSelectionInfo {
    /// Whether a slice plane is selected.
    pub has_selection: bool,
    /// Name of the selected slice plane.
    pub name: String,
    /// Current origin position.
    pub origin: [f32; 3],
    /// Current rotation as Euler angles (degrees).
    pub rotation_degrees: [f32; 3],
}

/// Actions specific to slice plane gizmo manipulation.
#[derive(Debug, Clone, PartialEq)]
pub enum SlicePlaneGizmoAction {
    /// No action.
    None,
    /// Slice plane selection changed.
    SelectionChanged,
    /// Transform was updated via gizmo.
    TransformChanged,
    /// Deselect the slice plane.
    Deselect,
}

impl Default for SlicePlaneGizmoAction {
    fn default() -> Self {
        Self::None
    }
}

/// Settings for a single group in the UI.
#[derive(Debug, Clone)]
pub struct GroupSettings {
    /// Name of the group.
    pub name: String,
    /// Whether the group is enabled (visible).
    pub enabled: bool,
    /// Whether to show child details in UI.
    pub show_child_details: bool,
    /// Parent group name (if any).
    pub parent_group: Option<String>,
    /// Child structure identifiers (type_name, name).
    pub child_structures: Vec<(String, String)>,
    /// Child group names.
    pub child_groups: Vec<String>,
}

impl Default for GroupSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            show_child_details: true,
            parent_group: None,
            child_structures: Vec::new(),
            child_groups: Vec::new(),
        }
    }
}

impl GroupSettings {
    /// Creates settings with a name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Actions that can be triggered from the groups UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsAction {
    /// No action.
    None,
    /// Create a new group with the given name.
    Create(String),
    /// Remove group at the given index.
    Remove(usize),
    /// Toggle enabled state for group at index.
    ToggleEnabled(usize),
    /// Toggle show_child_details for group at index.
    ToggleDetails(usize),
}

/// Gizmo settings for UI.
#[derive(Debug, Clone)]
pub struct GizmoSettings {
    /// Use local coordinate space (true) or world space (false).
    pub local_space: bool,
    /// Whether gizmo is visible.
    pub visible: bool,
    /// Translation snap value (0 = disabled).
    pub snap_translate: f32,
    /// Rotation snap value in degrees (0 = disabled).
    pub snap_rotate: f32,
    /// Scale snap value (0 = disabled).
    pub snap_scale: f32,
}

impl Default for GizmoSettings {
    fn default() -> Self {
        Self {
            local_space: false, // World space by default
            visible: true,
            snap_translate: 0.0,
            snap_rotate: 0.0,
            snap_scale: 0.0,
        }
    }
}

/// Current selection info for UI.
#[derive(Debug, Clone, Default)]
pub struct SelectionInfo {
    /// Whether something is selected.
    pub has_selection: bool,
    /// Selected structure type name.
    pub type_name: String,
    /// Selected structure name.
    pub name: String,
    /// Transform translation.
    pub translation: [f32; 3],
    /// Transform rotation as Euler angles in degrees.
    pub rotation_degrees: [f32; 3],
    /// Transform scale.
    pub scale: [f32; 3],
    /// Bounding box centroid (world space) - used for gizmo positioning.
    pub centroid: [f32; 3],
}

/// Actions that can be triggered from the gizmo UI.
#[derive(Debug, Clone, PartialEq)]
pub enum GizmoAction {
    /// No action.
    None,
    /// Gizmo settings changed.
    SettingsChanged,
    /// Transform was edited.
    TransformChanged,
    /// Deselect was clicked.
    Deselect,
    /// Reset transform was clicked.
    ResetTransform,
}

/// Actions that can be triggered from the view/controls UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewAction {
    /// No action.
    None,
    /// Reset view requested.
    ResetView,
    /// Screenshot requested.
    Screenshot,
}

/// Builds the gizmo/transform section.
/// Returns an action if one was triggered.
pub fn build_gizmo_section(
    ui: &mut Ui,
    settings: &mut GizmoSettings,
    selection: &mut SelectionInfo,
) -> GizmoAction {
    let mut action = GizmoAction::None;

    CollapsingHeader::new("Transform / Gizmo")
        .default_open(false)
        .show(ui, |ui| {
            // Selection info
            if selection.has_selection {
                ui.horizontal(|ui| {
                    ui.label("Selected:");
                    ui.label(format!("[{}] {}", selection.type_name, selection.name));
                });

                if ui.button("Deselect").clicked() {
                    action = GizmoAction::Deselect;
                }

                ui.separator();
            } else {
                ui.label("No selection");
                ui.label("Click a structure to select");
                ui.separator();
            }

            // Gizmo visibility
            if ui.checkbox(&mut settings.visible, "Show gizmo").changed() {
                action = GizmoAction::SettingsChanged;
            }

            ui.separator();

            // Gizmo space
            ui.horizontal(|ui| {
                ui.label("Space:");
                if ui.selectable_label(!settings.local_space, "World").clicked() {
                    settings.local_space = false;
                    action = GizmoAction::SettingsChanged;
                }
                if ui.selectable_label(settings.local_space, "Local").clicked() {
                    settings.local_space = true;
                    action = GizmoAction::SettingsChanged;
                }
            });

            ui.separator();

            // Snap settings
            ui.label("Snap:");
            ui.horizontal(|ui| {
                ui.label("Translate:");
                if ui
                    .add(
                        DragValue::new(&mut settings.snap_translate)
                            .speed(0.1)
                            .range(0.0..=10.0),
                    )
                    .changed()
                {
                    action = GizmoAction::SettingsChanged;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Rotate:");
                if ui
                    .add(
                        DragValue::new(&mut settings.snap_rotate)
                            .speed(1.0)
                            .range(0.0..=90.0)
                            .suffix("°"),
                    )
                    .changed()
                {
                    action = GizmoAction::SettingsChanged;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Scale:");
                if ui
                    .add(
                        DragValue::new(&mut settings.snap_scale)
                            .speed(0.1)
                            .range(0.0..=1.0),
                    )
                    .changed()
                {
                    action = GizmoAction::SettingsChanged;
                }
            });

            // Transform editing (only if selected)
            if selection.has_selection {
                ui.separator();
                ui.label("Transform:");

                // Translation
                ui.horizontal(|ui| {
                    ui.label("Pos:");
                    let mut changed = false;
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.translation[0])
                                .speed(0.1)
                                .prefix("X:"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.translation[1])
                                .speed(0.1)
                                .prefix("Y:"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.translation[2])
                                .speed(0.1)
                                .prefix("Z:"),
                        )
                        .changed();
                    if changed && action == GizmoAction::None {
                        action = GizmoAction::TransformChanged;
                    }
                });

                // Rotation (Euler angles)
                ui.horizontal(|ui| {
                    ui.label("Rot:");
                    let mut changed = false;
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.rotation_degrees[0])
                                .speed(1.0)
                                .prefix("X:")
                                .suffix("°"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.rotation_degrees[1])
                                .speed(1.0)
                                .prefix("Y:")
                                .suffix("°"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.rotation_degrees[2])
                                .speed(1.0)
                                .prefix("Z:")
                                .suffix("°"),
                        )
                        .changed();
                    if changed && action == GizmoAction::None {
                        action = GizmoAction::TransformChanged;
                    }
                });

                // Scale
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    let mut changed = false;
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.scale[0])
                                .speed(0.01)
                                .prefix("X:")
                                .range(0.01..=100.0),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.scale[1])
                                .speed(0.01)
                                .prefix("Y:")
                                .range(0.01..=100.0),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            DragValue::new(&mut selection.scale[2])
                                .speed(0.01)
                                .prefix("Z:")
                                .range(0.01..=100.0),
                        )
                        .changed();
                    if changed && action == GizmoAction::None {
                        action = GizmoAction::TransformChanged;
                    }
                });

                ui.separator();
                if ui.button("Reset Transform").clicked() {
                    action = GizmoAction::ResetTransform;
                }
            }
        });

    action
}

/// Builds UI for a single group item.
/// Returns true if enabled was toggled.
fn build_group_item(ui: &mut Ui, settings: &mut GroupSettings) -> bool {
    let mut toggled = false;

    // Enabled checkbox
    ui.horizontal(|ui| {
        if ui.checkbox(&mut settings.enabled, "Enabled").changed() {
            toggled = true;
        }
    });

    // Show child details checkbox
    ui.horizontal(|ui| {
        ui.checkbox(&mut settings.show_child_details, "Show details");
    });

    // Show parent if any
    if let Some(ref parent) = settings.parent_group {
        ui.horizontal(|ui| {
            ui.label("Parent:");
            ui.label(parent);
        });
    }

    // Show child structures
    if !settings.child_structures.is_empty() {
        ui.separator();
        ui.label("Structures:");
        ui.indent("structures", |ui| {
            for (type_name, name) in &settings.child_structures {
                ui.horizontal(|ui| {
                    ui.label(format!("[{}]", type_name));
                    ui.label(name);
                });
            }
        });
    }

    // Show child groups
    if !settings.child_groups.is_empty() {
        ui.separator();
        ui.label("Child groups:");
        ui.indent("child_groups", |ui| {
            for child_name in &settings.child_groups {
                ui.label(format!("  {}", child_name));
            }
        });
    }

    // Show empty state
    if settings.child_structures.is_empty() && settings.child_groups.is_empty() {
        ui.label("(empty)");
    }

    toggled
}

/// Builds the groups section.
/// Returns an action if one was triggered.
pub fn build_groups_section(
    ui: &mut Ui,
    groups: &mut Vec<GroupSettings>,
    new_group_name: &mut String,
) -> GroupsAction {
    let mut action = GroupsAction::None;

    CollapsingHeader::new("Groups")
        .default_open(false)
        .show(ui, |ui| {
            // Add new group controls
            ui.horizontal(|ui| {
                ui.label("New group:");
                ui.add_sized([80.0, 18.0], egui::TextEdit::singleline(new_group_name));
                if ui.button("Create").clicked() && !new_group_name.is_empty() {
                    action = GroupsAction::Create(new_group_name.clone());
                }
            });

            if groups.is_empty() {
                ui.label("No groups");
                return;
            }

            ui.separator();

            // Show only root groups (those without parents)
            let root_groups: Vec<usize> = groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.parent_group.is_none())
                .map(|(i, _)| i)
                .collect();

            let mut remove_idx = None;

            for idx in root_groups {
                let group = &mut groups[idx];
                let header_text = format!(
                    "{} {} ({})",
                    if group.enabled { "●" } else { "○" },
                    group.name,
                    group.child_structures.len()
                );

                CollapsingHeader::new(header_text)
                    .id_salt(format!("group_{}", idx))
                    .default_open(false)
                    .show(ui, |ui| {
                        if build_group_item(ui, group) && action == GroupsAction::None {
                            action = GroupsAction::ToggleEnabled(idx);
                        }

                        ui.separator();
                        if ui.button("Remove").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
            }

            if let Some(idx) = remove_idx {
                action = GroupsAction::Remove(idx);
            }
        });

    action
}

/// Builds the main left panel.
pub fn build_left_panel(ctx: &Context, build_contents: impl FnOnce(&mut Ui)) {
    SidePanel::left("polyscope_main_panel")
        .default_width(305.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("polyscope-rs");
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    build_contents(ui);
                });
        });
}

/// Builds the polyscope controls section.
/// Returns an action if any button was clicked.
pub fn build_controls_section(ui: &mut Ui, background_color: &mut [f32; 3]) -> ViewAction {
    let mut action = ViewAction::None;

    CollapsingHeader::new("View")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Background:");
                ui.color_edit_button_rgb(background_color);
            });

            ui.horizontal(|ui| {
                if ui.button("Reset View").clicked() {
                    action = ViewAction::ResetView;
                }
                if ui.button("Screenshot").clicked() {
                    action = ViewAction::Screenshot;
                }
            });

            ui.label("Tip: Press F12 for quick screenshot");
        });

    action
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
                    for (i, name) in ["Turntable", "Free", "Planar", "First Person", "None"]
                        .iter()
                        .enumerate()
                    {
                        if ui
                            .selectable_value(&mut settings.navigation_style, i as u32, *name)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });

            // Projection mode
            egui::ComboBox::from_label("Projection")
                .selected_text(if settings.projection_mode == 0 {
                    "Perspective"
                } else {
                    "Orthographic"
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut settings.projection_mode, 0, "Perspective")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut settings.projection_mode, 1, "Orthographic")
                        .changed()
                    {
                        changed = true;
                    }
                });

            ui.separator();

            // Up direction (front direction is auto-derived using right-hand rule)
            let directions = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];
            // Front directions corresponding to each up direction (right-hand rule)
            let front_for_up = ["+Y", "-Y", "-Z", "+Z", "+X", "-X"];

            egui::ComboBox::from_label("Up")
                .selected_text(directions[settings.up_direction as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in directions.iter().enumerate() {
                        if ui
                            .selectable_value(&mut settings.up_direction, i as u32, *name)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });

            // Show the auto-derived front direction (read-only)
            ui.horizontal(|ui| {
                ui.label("Front:");
                ui.label(front_for_up[settings.up_direction as usize]);
                ui.label("(auto)");
            });

            ui.separator();

            // FOV (only for perspective)
            if settings.projection_mode == 0 {
                ui.horizontal(|ui| {
                    ui.label("FOV:");
                    if ui
                        .add(Slider::new(&mut settings.fov_degrees, 10.0..=170.0).suffix("°"))
                        .changed()
                    {
                        changed = true;
                    }
                });
            } else {
                // Ortho scale
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    if ui
                        .add(
                            DragValue::new(&mut settings.ortho_scale)
                                .speed(0.1)
                                .range(0.1..=100.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
            }

            // Clip planes
            ui.horizontal(|ui| {
                ui.label("Near:");
                if ui
                    .add(
                        DragValue::new(&mut settings.near)
                            .speed(0.001)
                            .range(0.001..=10.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Far:");
                if ui
                    .add(
                        DragValue::new(&mut settings.far)
                            .speed(1.0)
                            .range(10.0..=10000.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });

            // Move speed
            ui.horizontal(|ui| {
                ui.label("Move Speed:");
                if ui
                    .add(
                        DragValue::new(&mut settings.move_speed)
                            .speed(0.1)
                            .range(0.1..=10.0),
                    )
                    .changed()
                {
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
            if ui
                .checkbox(&mut extents.auto_compute, "Auto-compute")
                .changed()
            {
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
                ui.label(format!(
                    "({:.2}, {:.2}, {:.2})",
                    center[0], center[1], center[2]
                ));
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
                    if ui
                        .selectable_value(&mut settings.transparency_mode, 0, "None")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut settings.transparency_mode, 1, "Simple")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut settings.transparency_mode, 2, "Weighted Blended")
                        .changed()
                    {
                        changed = true;
                    }
                });

            ui.separator();

            // SSAA factor
            egui::ComboBox::from_label("Anti-aliasing")
                .selected_text(format!("{}x SSAA", settings.ssaa_factor))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut settings.ssaa_factor, 1, "1x (Off)")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut settings.ssaa_factor, 2, "2x SSAA")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut settings.ssaa_factor, 4, "4x SSAA")
                        .changed()
                    {
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

            ui.separator();

            // SSAO
            if ui.checkbox(&mut settings.ssao_enabled, "SSAO").changed() {
                changed = true;
            }

            if settings.ssao_enabled {
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_radius, 0.01..=2.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Intensity:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_intensity, 0.1..=3.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Bias:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_bias, 0.001..=0.1))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Samples:");
                    let mut samples = settings.ssao_sample_count as i32;
                    if ui.add(DragValue::new(&mut samples).range(4..=64)).changed() {
                        settings.ssao_sample_count = samples.max(4) as u32;
                        changed = true;
                    }
                });
            }
        });

    changed
}

/// Builds the tone mapping settings section.
/// Returns true if any setting changed.
pub fn build_tone_mapping_section(ui: &mut Ui, settings: &mut ToneMappingSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Tone Mapping")
        .default_open(false)
        .show(ui, |ui| {
            if ui.checkbox(&mut settings.enabled, "Enable").changed() {
                changed = true;
            }

            if settings.enabled {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Exposure:");
                    if ui
                        .add(
                            Slider::new(&mut settings.exposure, 0.1..=4.0)
                                .logarithmic(true)
                                .clamping(egui::SliderClamping::Always),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("White Level:");
                    if ui
                        .add(
                            Slider::new(&mut settings.white_level, 0.5..=4.0)
                                .logarithmic(true)
                                .clamping(egui::SliderClamping::Always),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Gamma:");
                    if ui
                        .add(
                            Slider::new(&mut settings.gamma, 1.0..=3.0)
                                .clamping(egui::SliderClamping::Always),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.separator();
                if ui.button("Reset to Defaults").clicked() {
                    *settings = ToneMappingSettings::default();
                    changed = true;
                }
            }
        });

    changed
}

/// Actions that can be triggered from the slice planes UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlicePlanesAction {
    /// No action.
    None,
    /// Add a new slice plane with the given name.
    Add(String),
    /// Remove slice plane at the given index.
    Remove(usize),
    /// Settings for a plane were modified.
    Modified(usize),
}

/// Builds UI for a single slice plane.
/// Returns true if any setting changed.
fn build_slice_plane_item(ui: &mut Ui, settings: &mut SlicePlaneSettings) -> bool {
    let mut changed = false;

    // Enabled checkbox
    ui.horizontal(|ui| {
        if ui.checkbox(&mut settings.enabled, "Enabled").changed() {
            changed = true;
        }
    });

    ui.separator();

    // Origin
    ui.label("Origin:");
    ui.horizontal(|ui| {
        ui.label("X:");
        if ui
            .add(DragValue::new(&mut settings.origin[0]).speed(0.1))
            .changed()
        {
            changed = true;
        }
        ui.label("Y:");
        if ui
            .add(DragValue::new(&mut settings.origin[1]).speed(0.1))
            .changed()
        {
            changed = true;
        }
        ui.label("Z:");
        if ui
            .add(DragValue::new(&mut settings.origin[2]).speed(0.1))
            .changed()
        {
            changed = true;
        }
    });

    // Normal direction with preset buttons
    ui.label("Normal:");
    ui.horizontal(|ui| {
        if ui.button("+X").clicked() {
            settings.normal = [1.0, 0.0, 0.0];
            changed = true;
        }
        if ui.button("-X").clicked() {
            settings.normal = [-1.0, 0.0, 0.0];
            changed = true;
        }
        if ui.button("+Y").clicked() {
            settings.normal = [0.0, 1.0, 0.0];
            changed = true;
        }
        if ui.button("-Y").clicked() {
            settings.normal = [0.0, -1.0, 0.0];
            changed = true;
        }
        if ui.button("+Z").clicked() {
            settings.normal = [0.0, 0.0, 1.0];
            changed = true;
        }
        if ui.button("-Z").clicked() {
            settings.normal = [0.0, 0.0, -1.0];
            changed = true;
        }
    });

    // Custom normal input
    ui.horizontal(|ui| {
        ui.label("X:");
        if ui
            .add(
                DragValue::new(&mut settings.normal[0])
                    .speed(0.01)
                    .range(-1.0..=1.0),
            )
            .changed()
        {
            changed = true;
        }
        ui.label("Y:");
        if ui
            .add(
                DragValue::new(&mut settings.normal[1])
                    .speed(0.01)
                    .range(-1.0..=1.0),
            )
            .changed()
        {
            changed = true;
        }
        ui.label("Z:");
        if ui
            .add(
                DragValue::new(&mut settings.normal[2])
                    .speed(0.01)
                    .range(-1.0..=1.0),
            )
            .changed()
        {
            changed = true;
        }
    });

    ui.separator();

    // Visualization options
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut settings.draw_plane, "Draw plane")
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(&mut settings.draw_widget, "Draw widget")
            .changed()
        {
            changed = true;
        }
    });

    // Gizmo control button (only show when draw_widget is enabled)
    if settings.draw_widget && settings.enabled {
        ui.horizontal(|ui| {
            let button_text = if settings.is_selected {
                "Editing (click to deselect)"
            } else {
                "Edit with Gizmo"
            };
            if ui.button(button_text).clicked() {
                settings.is_selected = !settings.is_selected;
                changed = true;
            }
        });
    }

    // Plane size
    ui.horizontal(|ui| {
        ui.label("Plane size:");
        if ui
            .add(Slider::new(&mut settings.plane_size, 0.01..=1.0).logarithmic(true))
            .changed()
        {
            changed = true;
        }
    });

    // Color
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(&mut settings.color).changed() {
            changed = true;
        }
    });

    changed
}

/// Builds the slice planes section.
/// Returns an action if one was triggered (add, remove, or modify).
pub fn build_slice_planes_section(
    ui: &mut Ui,
    planes: &mut Vec<SlicePlaneSettings>,
    new_plane_name: &mut String,
) -> SlicePlanesAction {
    let mut action = SlicePlanesAction::None;

    CollapsingHeader::new("Slice Planes")
        .default_open(false)
        .show(ui, |ui| {
            // Add new plane controls
            ui.horizontal(|ui| {
                ui.label("New plane:");
                ui.add_sized([80.0, 18.0], egui::TextEdit::singleline(new_plane_name));
                if ui.button("Add").clicked() && !new_plane_name.is_empty() {
                    action = SlicePlanesAction::Add(new_plane_name.clone());
                }
            });

            if planes.is_empty() {
                ui.label("No slice planes");
                return;
            }

            ui.separator();

            // List existing planes
            let mut remove_idx = None;
            for (idx, plane) in planes.iter_mut().enumerate() {
                let header_text =
                    format!("{} {}", if plane.enabled { "●" } else { "○" }, plane.name);

                CollapsingHeader::new(header_text)
                    .id_salt(format!("slice_plane_{}", idx))
                    .default_open(false)
                    .show(ui, |ui| {
                        if build_slice_plane_item(ui, plane) {
                            if action == SlicePlanesAction::None {
                                action = SlicePlanesAction::Modified(idx);
                            }
                        }

                        ui.separator();
                        if ui.button("Remove").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
            }

            if let Some(idx) = remove_idx {
                action = SlicePlanesAction::Remove(idx);
            }
        });

    action
}

/// Builds the ground plane settings section.
pub fn build_ground_plane_section(
    ui: &mut Ui,
    mode: &mut u32, // 0=None, 1=Tile, 2=ShadowOnly, 3=TileReflection
    height: &mut f32,
    height_is_relative: &mut bool,
    shadow_blur_iters: &mut u32,
    shadow_darkness: &mut f32,
    reflection_intensity: &mut f32,
) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Ground Plane")
        .default_open(false)
        .show(ui, |ui| {
            // Mode selector
            egui::ComboBox::from_label("Mode")
                .selected_text(match *mode {
                    0 => "None",
                    1 => "Tile",
                    2 => "Shadow Only",
                    3 => "Tile + Reflection",
                    _ => "Unknown",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(mode, 0, "None").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 1, "Tile").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 2, "Shadow Only").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(mode, 3, "Tile + Reflection").changed() {
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

                // Shadow settings
                ui.separator();
                ui.label("Shadow Settings:");

                ui.horizontal(|ui| {
                    ui.label("Blur iterations:");
                    if ui
                        .add(Slider::new(shadow_blur_iters, 0..=5))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Darkness:");
                    if ui
                        .add(Slider::new(shadow_darkness, 0.0..=1.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                // Reflection settings (only for mode 3 - TileReflection)
                if *mode == 3 {
                    ui.separator();
                    ui.label("Reflection Settings:");

                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        if ui
                            .add(Slider::new(reflection_intensity, 0.0..=1.0))
                            .changed()
                        {
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

            // Group by type using BTreeMap for stable, sorted ordering
            let mut by_type: std::collections::BTreeMap<&str, Vec<(&str, bool)>> =
                std::collections::BTreeMap::new();
            for (type_name, name, enabled) in structures {
                by_type
                    .entry(type_name.as_str())
                    .or_default()
                    .push((name.as_str(), *enabled));
            }

            for (type_name, instances) in &by_type {
                // Sort instances by name for stable ordering
                let mut sorted_instances: Vec<_> = instances.iter().collect();
                sorted_instances.sort_by_key(|(name, _)| *name);

                let header = format!("{} ({})", type_name, instances.len());
                CollapsingHeader::new(header)
                    .default_open(instances.len() <= 8)
                    .show(ui, |ui| {
                        for (name, enabled) in sorted_instances {
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

            // Group by type using BTreeMap for stable, sorted ordering
            let mut by_type: std::collections::BTreeMap<&str, Vec<(&str, bool)>> =
                std::collections::BTreeMap::new();
            for (type_name, name, enabled) in structures {
                by_type
                    .entry(type_name.as_str())
                    .or_default()
                    .push((name.as_str(), *enabled));
            }

            for (type_name, instances) in &by_type {
                // Sort instances by name for stable ordering
                let mut sorted_instances: Vec<_> = instances.iter().collect();
                sorted_instances.sort_by_key(|(name, _)| *name);

                let header = format!("{} ({})", type_name, instances.len());
                CollapsingHeader::new(header)
                    .default_open(instances.len() <= 8)
                    .show(ui, |ui| {
                        for (name, enabled) in sorted_instances {
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
