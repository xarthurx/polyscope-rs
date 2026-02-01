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
    /// Transparency mode (0=None, 1=Simple, 2=Pretty/DepthPeeling)
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
            transparency_mode: 1, // Simple (default)
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
            exposure: 1.1,
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
            plane_size: 0.05,
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
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SlicePlaneGizmoAction {
    /// No action.
    #[default]
    None,
    /// Slice plane selection changed.
    SelectionChanged,
    /// Transform was updated via gizmo.
    TransformChanged,
    /// Deselect the slice plane.
    Deselect,
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
    /// Child structure identifiers (`type_name`, name).
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
    /// Enabled state changed for groups at these indices (includes cascaded children).
    SyncEnabled(Vec<usize>),
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

    CollapsingHeader::new("Transform")
        .default_open(false)
        .show(ui, |ui| {
            // Selection info
            if selection.has_selection {
                ui.horizontal(|ui| {
                    ui.label(format!("[{}] {}", selection.type_name, selection.name));
                    if ui.small_button("x").clicked() {
                        action = GizmoAction::Deselect;
                    }
                });

                ui.separator();

                // Transform editing
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
                    if changed {
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

                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        action = GizmoAction::ResetTransform;
                    }
                    ui.separator();
                    if ui.checkbox(&mut settings.visible, "Gizmo").changed() {
                        action = GizmoAction::SettingsChanged;
                    }
                    if settings.visible {
                        if ui
                            .selectable_label(!settings.local_space, "W")
                            .on_hover_text("World space")
                            .clicked()
                        {
                            settings.local_space = false;
                            action = GizmoAction::SettingsChanged;
                        }
                        if ui
                            .selectable_label(settings.local_space, "L")
                            .on_hover_text("Local space")
                            .clicked()
                        {
                            settings.local_space = true;
                            action = GizmoAction::SettingsChanged;
                        }
                    }
                });
            } else {
                ui.label("No selection");
            }
        });

    action
}

/// Counts the total number of structures in a group and all its descendant groups.
fn count_structures_recursive(idx: usize, groups: &[GroupSettings]) -> usize {
    let mut total = groups[idx].child_structures.len();
    let name = &groups[idx].name;
    for (i, g) in groups.iter().enumerate() {
        if g.parent_group.as_deref() == Some(name) {
            total += count_structures_recursive(i, groups);
        }
    }
    total
}

/// Collects the index of a group and all its descendant groups (recursive).
fn collect_descendant_indices(idx: usize, groups: &[GroupSettings], out: &mut Vec<usize>) {
    let name = &groups[idx].name;
    for (i, g) in groups.iter().enumerate() {
        if g.parent_group.as_deref() == Some(name) {
            out.push(i);
            collect_descendant_indices(i, groups, out);
        }
    }
}

/// Renders a group checkbox and recursively renders its child groups indented.
fn build_group_tree(
    ui: &mut Ui,
    idx: usize,
    groups: &mut Vec<GroupSettings>,
    toggled_idx: &mut Option<usize>,
) {
    let member_count = count_structures_recursive(idx, groups);
    let label = format!("{} ({member_count})", groups[idx].name);

    ui.horizontal(|ui| {
        if ui.checkbox(&mut groups[idx].enabled, label).changed() && toggled_idx.is_none() {
            *toggled_idx = Some(idx);
        }
    });

    // Collect child group indices
    let child_name = groups[idx].name.clone();
    let child_indices: Vec<usize> = groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.parent_group.as_deref() == Some(child_name.as_str()))
        .map(|(i, _)| i)
        .collect();

    if !child_indices.is_empty() {
        ui.indent(format!("group_children_{idx}"), |ui| {
            for child_idx in child_indices {
                build_group_tree(ui, child_idx, groups, toggled_idx);
            }
        });
    }
}

/// Builds the groups section.
/// Only shown when groups exist (groups are created programmatically via the API).
/// Each group is shown with a checkbox to toggle visibility.
/// Child groups are indented under their parent.
/// Toggling a parent cascades the enabled state to all descendant groups.
/// Returns an action if one was triggered.
pub fn build_groups_section(
    ui: &mut Ui,
    groups: &mut Vec<GroupSettings>,
) -> GroupsAction {
    if groups.is_empty() {
        return GroupsAction::None;
    }

    let mut toggled_idx: Option<usize> = None;

    CollapsingHeader::new("Groups")
        .default_open(true)
        .show(ui, |ui| {
            // Find root groups (no parent)
            let root_indices: Vec<usize> = groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.parent_group.is_none())
                .map(|(i, _)| i)
                .collect();

            for idx in root_indices {
                build_group_tree(ui, idx, groups, &mut toggled_idx);
            }
        });

    if let Some(idx) = toggled_idx {
        // Cascade the new enabled state to all descendant groups
        let new_state = groups[idx].enabled;
        let mut affected = vec![idx];
        collect_descendant_indices(idx, groups, &mut affected);
        for &i in &affected[1..] {
            groups[i].enabled = new_state;
        }
        GroupsAction::SyncEnabled(affected)
    } else {
        GroupsAction::None
    }
}

/// Builds the main left panel.
/// Returns the actual panel width in logical pixels (for dynamic pointer checks).
pub fn build_left_panel(ctx: &Context, build_contents: impl FnOnce(&mut Ui)) -> f32 {
    let resp = SidePanel::left("polyscope_main_panel")
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
    resp.response.rect.width()
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

            ui.columns(2, |cols| {
                let w = cols[0].available_width();
                let h = cols[0].spacing().interact_size.y;
                if cols[0].add_sized([w, h], egui::Button::new("Reset View")).clicked() {
                    action = ViewAction::ResetView;
                }
                if cols[1].add_sized([w, h], egui::Button::new("Screenshot")).clicked() {
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
                    3 => "Arcball",
                    4 => "First Person",
                    _ => "None",
                })
                .show_ui(ui, |ui| {
                    for (i, name) in [
                        "Turntable",
                        "Free",
                        "Planar",
                        "Arcball",
                        "First Person",
                        "None",
                    ]
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
/// Returns true if any setting changed (auto-compute toggle or manual edits).
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

            if extents.auto_compute {
                // Auto-compute ON: read-only display
                ui.horizontal(|ui| {
                    ui.label("Length scale:");
                    ui.label(format!("{:.4}", extents.length_scale));
                });

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
            } else {
                // Auto-compute OFF: editable controls (matching C++ Polyscope)
                ui.horizontal(|ui| {
                    ui.label("Length scale:");
                    if ui
                        .add(
                            DragValue::new(&mut extents.length_scale)
                                .speed(0.01)
                                .range(0.0001..=f32::MAX),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.label("Bounding box:");
                ui.indent("bbox_edit", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Min:");
                        for val in &mut extents.bbox_min {
                            if ui.add(DragValue::new(val).speed(0.01)).changed() {
                                changed = true;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max:");
                        for val in &mut extents.bbox_max {
                            if ui.add(DragValue::new(val).speed(0.01)).changed() {
                                changed = true;
                            }
                        }
                    });
                });
            }

            // Compute center and display
            let center = [
                f32::midpoint(extents.bbox_min[0], extents.bbox_max[0]),
                f32::midpoint(extents.bbox_min[1], extents.bbox_max[1]),
                f32::midpoint(extents.bbox_min[2], extents.bbox_max[2]),
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
                    _ => "Pretty",
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
                        .selectable_value(&mut settings.transparency_mode, 2, "Pretty")
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
                egui::Grid::new("ssao_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Radius:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_radius, 0.01..=2.0))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Intensity:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_intensity, 0.1..=3.0))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Bias:");
                    if ui
                        .add(Slider::new(&mut settings.ssao_bias, 0.001..=0.1))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Samples:");
                    let mut samples = settings.ssao_sample_count as i32;
                    if ui.add(DragValue::new(&mut samples).range(4..=64)).changed() {
                        settings.ssao_sample_count = samples.max(4) as u32;
                        changed = true;
                    }
                    ui.end_row();
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
            egui::Grid::new("tone_mapping_grid").num_columns(2).show(ui, |ui| {
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
                ui.end_row();

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
                ui.end_row();

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
                ui.end_row();
            });

            ui.separator();
            if ui.button("Reset to Defaults").clicked() {
                *settings = ToneMappingSettings::default();
                changed = true;
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
    ui.columns(6, |cols| {
        let w = cols[0].available_width();
        let h = cols[0].spacing().interact_size.y;
        if cols[0].add_sized([w, h], egui::Button::new("+X")).clicked() {
            settings.normal = [1.0, 0.0, 0.0];
            changed = true;
        }
        if cols[1].add_sized([w, h], egui::Button::new("-X")).clicked() {
            settings.normal = [-1.0, 0.0, 0.0];
            changed = true;
        }
        if cols[2].add_sized([w, h], egui::Button::new("+Y")).clicked() {
            settings.normal = [0.0, 1.0, 0.0];
            changed = true;
        }
        if cols[3].add_sized([w, h], egui::Button::new("-Y")).clicked() {
            settings.normal = [0.0, -1.0, 0.0];
            changed = true;
        }
        if cols[4].add_sized([w, h], egui::Button::new("+Z")).clicked() {
            settings.normal = [0.0, 0.0, 1.0];
            changed = true;
        }
        if cols[5].add_sized([w, h], egui::Button::new("-Z")).clicked() {
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
        let gizmo_text = if settings.is_selected {
            "Deselect Gizmo"
        } else {
            "Edit with Gizmo"
        };
        if ui.button(gizmo_text).clicked() {
            settings.is_selected = !settings.is_selected;
            changed = true;
        }
    }

    // Plane size & Color
    egui::Grid::new("slice_plane_props").num_columns(2).show(ui, |ui| {
        ui.label("Plane size:");
        if ui
            .add(Slider::new(&mut settings.plane_size, 0.01..=1.0).logarithmic(true))
            .changed()
        {
            changed = true;
        }
        ui.end_row();

        ui.label("Color:");
        if ui.color_edit_button_rgb(&mut settings.color).changed() {
            changed = true;
        }
        ui.end_row();
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
                    .id_salt(format!("slice_plane_{idx}"))
                    .default_open(false)
                    .show(ui, |ui| {
                        if build_slice_plane_item(ui, plane) && action == SlicePlanesAction::None {
                            action = SlicePlanesAction::Modified(idx);
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

                egui::Grid::new("shadow_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Blur iterations:");
                    if ui.add(Slider::new(shadow_blur_iters, 0..=5)).changed() {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Darkness:");
                    if ui.add(Slider::new(shadow_darkness, 0.0..=1.0)).changed() {
                        changed = true;
                    }
                    ui.end_row();
                });

                // Reflection settings (only for mode 3 - TileReflection)
                if *mode == 3 {
                    ui.separator();
                    ui.label("Reflection Settings:");

                    egui::Grid::new("reflection_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Intensity:");
                        if ui
                            .add(Slider::new(reflection_intensity, 0.0..=1.0))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();
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
                                    if ui.checkbox(&mut enabled_mut, "Enabled").changed() {
                                        on_toggle(type_name, name, enabled_mut);
                                    }

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
