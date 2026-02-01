use crate::{GizmoMode, GizmoSpace, Mat4, with_context, with_context_mut};

/// Selects a structure for gizmo manipulation.
///
/// Only one structure can be selected at a time. The gizmo will appear
/// at the selected structure's position when enabled.
pub fn select_structure(type_name: &str, name: &str) {
    with_context_mut(|ctx| {
        ctx.select_structure(type_name, name);
    });
}

/// Deselects the currently selected structure.
pub fn deselect_structure() {
    with_context_mut(|ctx| {
        ctx.deselect_structure();
    });
}

/// Returns the currently selected structure, if any.
#[must_use]
pub fn get_selected_structure() -> Option<(String, String)> {
    with_context(|ctx| {
        ctx.selected_structure()
            .map(|(t, n)| (t.to_string(), n.to_string()))
    })
}

/// Returns whether any structure is currently selected.
#[must_use]
pub fn has_selection() -> bool {
    with_context(polyscope_core::Context::has_selection)
}

/// Sets the gizmo mode (translate, rotate, or scale).
pub fn set_gizmo_mode(mode: GizmoMode) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().mode = mode;
    });
}

/// Returns the current gizmo mode.
#[must_use]
pub fn get_gizmo_mode() -> GizmoMode {
    with_context(|ctx| ctx.gizmo().mode)
}

/// Sets the gizmo coordinate space (world or local).
pub fn set_gizmo_space(space: GizmoSpace) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().space = space;
    });
}

/// Returns the current gizmo coordinate space.
#[must_use]
pub fn get_gizmo_space() -> GizmoSpace {
    with_context(|ctx| ctx.gizmo().space)
}

/// Sets whether the gizmo is visible.
pub fn set_gizmo_visible(visible: bool) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().visible = visible;
    });
}

/// Returns whether the gizmo is visible.
#[must_use]
pub fn is_gizmo_visible() -> bool {
    with_context(|ctx| ctx.gizmo().visible)
}

/// Sets the translation snap value for the gizmo.
///
/// When non-zero, translations will snap to multiples of this value.
pub fn set_gizmo_snap_translate(snap: f32) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().snap_translate = snap;
    });
}

/// Sets the rotation snap value for the gizmo (in degrees).
///
/// When non-zero, rotations will snap to multiples of this value.
pub fn set_gizmo_snap_rotate(snap_degrees: f32) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().snap_rotate = snap_degrees;
    });
}

/// Sets the scale snap value for the gizmo.
///
/// When non-zero, scale will snap to multiples of this value.
pub fn set_gizmo_snap_scale(snap: f32) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().snap_scale = snap;
    });
}

/// Sets the transform of the currently selected structure.
///
/// Does nothing if no structure is selected.
pub fn set_selected_transform(transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some((type_name, name)) = ctx.selected_structure.clone() {
            if let Some(structure) = ctx.registry.get_mut(&type_name, &name) {
                structure.set_transform(transform);
            }
        }
    });
}

/// Gets the transform of the currently selected structure.
///
/// Returns identity matrix if no structure is selected.
#[must_use]
pub fn get_selected_transform() -> Mat4 {
    with_context(|ctx| {
        if let Some((type_name, name)) = ctx.selected_structure() {
            ctx.registry
                .get(type_name, name)
                .map_or(Mat4::IDENTITY, polyscope_core::Structure::transform)
        } else {
            Mat4::IDENTITY
        }
    })
}

/// Resets the transform of the currently selected structure to identity.
///
/// Does nothing if no structure is selected.
pub fn reset_selected_transform() {
    set_selected_transform(Mat4::IDENTITY);
}
