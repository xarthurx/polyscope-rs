use crate::{
    add_slice_plane, deselect_structure, remove_slice_plane,
    reset_selected_transform, with_context, with_context_mut, GizmoSpace, Mat4, Vec3,
};
use polyscope_core::gizmo::Transform;

/// Syncs `CameraSettings` from UI to the actual Camera.
pub fn apply_camera_settings(
    camera: &mut polyscope_render::Camera,
    settings: &polyscope_ui::CameraSettings,
) {
    use polyscope_render::{AxisDirection, NavigationStyle, ProjectionMode};

    camera.navigation_style = match settings.navigation_style {
        0 => NavigationStyle::Turntable,
        1 => NavigationStyle::Free,
        2 => NavigationStyle::Planar,
        3 => NavigationStyle::Arcball,
        4 => NavigationStyle::FirstPerson,
        _ => NavigationStyle::None,
    };

    camera.projection_mode = match settings.projection_mode {
        0 => ProjectionMode::Perspective,
        _ => ProjectionMode::Orthographic,
    };

    camera.set_up_direction(match settings.up_direction {
        0 => AxisDirection::PosX,
        1 => AxisDirection::NegX,
        2 => AxisDirection::PosY,
        3 => AxisDirection::NegY,
        4 => AxisDirection::PosZ,
        _ => AxisDirection::NegZ,
    });
    // Note: front_direction is now auto-derived by set_up_direction()

    camera.set_fov_degrees(settings.fov_degrees);
    camera.set_near(settings.near);
    camera.set_far(settings.far);
    camera.set_move_speed(settings.move_speed);
    camera.set_ortho_scale(settings.ortho_scale);
}

/// Creates `CameraSettings` from the current Camera state.
#[must_use]
pub fn camera_to_settings(camera: &polyscope_render::Camera) -> polyscope_ui::CameraSettings {
    use polyscope_render::{AxisDirection, NavigationStyle, ProjectionMode};

    polyscope_ui::CameraSettings {
        navigation_style: match camera.navigation_style {
            NavigationStyle::Turntable => 0,
            NavigationStyle::Free => 1,
            NavigationStyle::Planar => 2,
            NavigationStyle::Arcball => 3,
            NavigationStyle::FirstPerson => 4,
            NavigationStyle::None => 5,
        },
        projection_mode: match camera.projection_mode {
            ProjectionMode::Perspective => 0,
            ProjectionMode::Orthographic => 1,
        },
        up_direction: match camera.up_direction {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        },
        // Note: front_direction is auto-derived from up_direction
        fov_degrees: camera.fov_degrees(),
        near: camera.near,
        far: camera.far,
        move_speed: camera.move_speed,
        ortho_scale: camera.ortho_scale,
    }
}

/// Gets scene extents from the global context.
#[must_use]
pub fn get_scene_extents() -> polyscope_ui::SceneExtents {
    polyscope_core::state::with_context(|ctx| polyscope_ui::SceneExtents {
        auto_compute: ctx.options.auto_compute_scene_extents,
        length_scale: ctx.length_scale,
        bbox_min: ctx.bounding_box.0.to_array(),
        bbox_max: ctx.bounding_box.1.to_array(),
    })
}

/// Sets auto-compute scene extents option.
///
/// When re-enabling auto-compute, immediately recomputes extents from
/// all registered structures (matching C++ Polyscope behavior).
pub fn set_auto_compute_extents(auto: bool) {
    polyscope_core::state::with_context_mut(|ctx| {
        ctx.options.auto_compute_scene_extents = auto;
        if auto {
            ctx.recompute_extents();
        }
    });
}

// ============================================================================
// Slice Plane UI Sync Functions
// ============================================================================

/// Gets all slice planes as UI settings.
#[must_use]
pub fn get_slice_plane_settings() -> Vec<polyscope_ui::SlicePlaneSettings> {
    with_context(|ctx| {
        let selected = ctx.selected_slice_plane();
        ctx.slice_planes
            .values()
            .map(|plane| polyscope_ui::SlicePlaneSettings {
                name: plane.name().to_string(),
                enabled: plane.is_enabled(),
                origin: plane.origin().to_array(),
                normal: plane.normal().to_array(),
                draw_plane: plane.draw_plane(),
                draw_widget: plane.draw_widget(),
                color: plane.color().truncate().to_array(),
                transparency: plane.transparency(),
                plane_size: plane.plane_size(),
                is_selected: selected == Some(plane.name()),
            })
            .collect()
    })
}

/// Applies UI settings to a slice plane.
pub fn apply_slice_plane_settings(settings: &polyscope_ui::SlicePlaneSettings) {
    with_context_mut(|ctx| {
        if let Some(plane) = ctx.get_slice_plane_mut(&settings.name) {
            plane.set_enabled(settings.enabled);
            plane.set_origin(Vec3::from_array(settings.origin));
            plane.set_normal(Vec3::from_array(settings.normal));
            plane.set_draw_plane(settings.draw_plane);
            plane.set_draw_widget(settings.draw_widget);
            plane.set_color(Vec3::from_array(settings.color));
            plane.set_transparency(settings.transparency);
            plane.set_plane_size(settings.plane_size);
        }
    });
}

/// Handles a slice plane UI action.
/// Returns the new list of settings after the action.
pub fn handle_slice_plane_action(
    action: polyscope_ui::SlicePlanesAction,
    current_settings: &mut Vec<polyscope_ui::SlicePlaneSettings>,
) {
    match action {
        polyscope_ui::SlicePlanesAction::None => {}
        polyscope_ui::SlicePlanesAction::Add(name) => {
            add_slice_plane(&name);
            // Get the actual settings from the created plane (it has scene-relative values)
            let settings = with_context(|ctx| {
                if let Some(plane) = ctx.get_slice_plane(&name) {
                    polyscope_ui::SlicePlaneSettings {
                        name: plane.name().to_string(),
                        enabled: plane.is_enabled(),
                        origin: plane.origin().to_array(),
                        normal: plane.normal().to_array(),
                        draw_plane: plane.draw_plane(),
                        draw_widget: plane.draw_widget(),
                        color: plane.color().truncate().to_array(),
                        transparency: plane.transparency(),
                        plane_size: plane.plane_size(),
                        is_selected: false,
                    }
                } else {
                    polyscope_ui::SlicePlaneSettings::with_name(&name)
                }
            });
            current_settings.push(settings);
        }
        polyscope_ui::SlicePlanesAction::Remove(idx) => {
            if idx < current_settings.len() {
                let name = &current_settings[idx].name;
                remove_slice_plane(name);
                current_settings.remove(idx);
            }
        }
        polyscope_ui::SlicePlanesAction::Modified(idx) => {
            if idx < current_settings.len() {
                apply_slice_plane_settings(&current_settings[idx]);
            }
        }
    }
}

// ============================================================================
// Slice Plane Gizmo Functions
// ============================================================================

/// Gets slice plane selection info for gizmo rendering.
#[must_use]
pub fn get_slice_plane_selection_info() -> polyscope_ui::SlicePlaneSelectionInfo {
    with_context(|ctx| {
        if let Some(name) = ctx.selected_slice_plane() {
            if let Some(plane) = ctx.get_slice_plane(name) {
                let transform = plane.to_transform();
                let (_, rotation, _) = transform.to_scale_rotation_translation();
                let euler = rotation.to_euler(glam::EulerRot::XYZ);

                polyscope_ui::SlicePlaneSelectionInfo {
                    has_selection: true,
                    name: name.to_string(),
                    origin: plane.origin().to_array(),
                    rotation_degrees: [
                        euler.0.to_degrees(),
                        euler.1.to_degrees(),
                        euler.2.to_degrees(),
                    ],
                }
            } else {
                polyscope_ui::SlicePlaneSelectionInfo::default()
            }
        } else {
            polyscope_ui::SlicePlaneSelectionInfo::default()
        }
    })
}

/// Selects a slice plane for gizmo manipulation.
pub fn select_slice_plane_for_gizmo(name: &str) {
    with_context_mut(|ctx| {
        ctx.select_slice_plane(name);
    });
}

/// Deselects the current slice plane from gizmo.
pub fn deselect_slice_plane_gizmo() {
    with_context_mut(|ctx| {
        ctx.deselect_slice_plane();
    });
}

/// Applies gizmo transform to the selected slice plane.
pub fn apply_slice_plane_gizmo_transform(origin: [f32; 3], rotation_degrees: [f32; 3]) {
    with_context_mut(|ctx| {
        if let Some(name) = ctx.selected_slice_plane.clone() {
            if let Some(plane) = ctx.get_slice_plane_mut(&name) {
                // Reconstruct transform from origin + rotation
                let rotation = glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    rotation_degrees[0].to_radians(),
                    rotation_degrees[1].to_radians(),
                    rotation_degrees[2].to_radians(),
                );
                let transform =
                    glam::Mat4::from_rotation_translation(rotation, Vec3::from_array(origin));
                plane.set_from_transform(transform);
            }
        }
    });
}

// ============================================================================
// Group UI Sync Functions
// ============================================================================

/// Gets all groups as UI settings.
#[must_use]
pub fn get_group_settings() -> Vec<polyscope_ui::GroupSettings> {
    with_context(|ctx| {
        ctx.groups
            .values()
            .map(|group| polyscope_ui::GroupSettings {
                name: group.name().to_string(),
                enabled: group.is_enabled(),
                show_child_details: group.show_child_details(),
                parent_group: group.parent_group().map(std::string::ToString::to_string),
                child_structures: group
                    .child_structures()
                    .map(|(t, n)| (t.to_string(), n.to_string()))
                    .collect(),
                child_groups: group
                    .child_groups()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })
            .collect()
    })
}

/// Applies UI settings to a group.
pub fn apply_group_settings(settings: &polyscope_ui::GroupSettings) {
    with_context_mut(|ctx| {
        if let Some(group) = ctx.get_group_mut(&settings.name) {
            group.set_enabled(settings.enabled);
            group.set_show_child_details(settings.show_child_details);
        }
    });
}

/// Handles a group UI action.
pub fn handle_group_action(
    action: polyscope_ui::GroupsAction,
    current_settings: &mut [polyscope_ui::GroupSettings],
) {
    match action {
        polyscope_ui::GroupsAction::None => {}
        polyscope_ui::GroupsAction::SyncEnabled(indices) => {
            for idx in indices {
                if idx < current_settings.len() {
                    apply_group_settings(&current_settings[idx]);
                }
            }
        }
    }
}

// ============================================================================
// Gizmo UI Sync Functions
// ============================================================================

/// Gets gizmo settings for UI.
#[must_use]
pub fn get_gizmo_settings() -> polyscope_ui::GizmoSettings {
    with_context(|ctx| {
        let gizmo = ctx.gizmo();
        polyscope_ui::GizmoSettings {
            local_space: matches!(gizmo.space, GizmoSpace::Local),
            visible: gizmo.visible,
            snap_translate: gizmo.snap_translate,
            snap_rotate: gizmo.snap_rotate,
            snap_scale: gizmo.snap_scale,
        }
    })
}

/// Applies gizmo settings from UI.
pub fn apply_gizmo_settings(settings: &polyscope_ui::GizmoSettings) {
    with_context_mut(|ctx| {
        let gizmo = ctx.gizmo_mut();
        gizmo.space = if settings.local_space {
            GizmoSpace::Local
        } else {
            GizmoSpace::World
        };
        gizmo.visible = settings.visible;
        gizmo.snap_translate = settings.snap_translate;
        gizmo.snap_rotate = settings.snap_rotate;
        gizmo.snap_scale = settings.snap_scale;
    });
}

/// Gets selection info for UI.
#[must_use]
pub fn get_selection_info() -> polyscope_ui::SelectionInfo {
    with_context(|ctx| {
        if let Some((type_name, name)) = ctx.selected_structure() {
            // Get transform and bounding box from selected structure
            let (transform, bbox) = ctx
                .registry
                .get(type_name, name)
                .map_or((Mat4::IDENTITY, None), |s| {
                    (s.transform(), s.bounding_box())
                });

            let t = Transform::from_matrix(transform);
            let euler = t.euler_angles_degrees();

            // Compute centroid from bounding box (world space)
            let centroid = bbox.map_or(t.translation, |(min, max)| (min + max) * 0.5);

            polyscope_ui::SelectionInfo {
                has_selection: true,
                type_name: type_name.to_string(),
                name: name.to_string(),
                translation: t.translation.to_array(),
                rotation_degrees: euler.to_array(),
                scale: t.scale.to_array(),
                centroid: centroid.to_array(),
            }
        } else {
            polyscope_ui::SelectionInfo::default()
        }
    })
}

/// Applies transform from selection info to the selected structure.
pub fn apply_selection_transform(selection: &polyscope_ui::SelectionInfo) {
    if !selection.has_selection {
        return;
    }

    let translation = Vec3::from_array(selection.translation);
    let rotation = glam::Quat::from_euler(
        glam::EulerRot::XYZ,
        selection.rotation_degrees[0].to_radians(),
        selection.rotation_degrees[1].to_radians(),
        selection.rotation_degrees[2].to_radians(),
    );
    let scale = Vec3::from_array(selection.scale);

    let transform = Mat4::from_scale_rotation_translation(scale, rotation, translation);

    with_context_mut(|ctx| {
        if let Some((type_name, name)) = ctx.selected_structure.clone() {
            if let Some(structure) = ctx.registry.get_mut(&type_name, &name) {
                structure.set_transform(transform);
            }
        }
    });
}

/// Handles a gizmo UI action.
pub fn handle_gizmo_action(
    action: polyscope_ui::GizmoAction,
    settings: &polyscope_ui::GizmoSettings,
    selection: &polyscope_ui::SelectionInfo,
) {
    match action {
        polyscope_ui::GizmoAction::None => {}
        polyscope_ui::GizmoAction::SettingsChanged => {
            apply_gizmo_settings(settings);
        }
        polyscope_ui::GizmoAction::TransformChanged => {
            apply_selection_transform(selection);
        }
        polyscope_ui::GizmoAction::Deselect => {
            deselect_structure();
        }
        polyscope_ui::GizmoAction::ResetTransform => {
            reset_selected_transform();
        }
    }
}
