//! Global state management for polyscope.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use glam::Vec3;

use crate::error::{PolyscopeError, Result};
use crate::gizmo::GizmoConfig;
use crate::group::Group;
use crate::options::Options;
use crate::registry::Registry;
use crate::slice_plane::SlicePlane;

/// Global context singleton.
static CONTEXT: OnceLock<RwLock<Context>> = OnceLock::new();

/// The global context containing all polyscope state.
pub struct Context {
    /// Whether polyscope has been initialized.
    pub initialized: bool,

    /// The structure registry.
    pub registry: Registry,

    /// Groups for organizing structures.
    pub groups: HashMap<String, Group>,

    /// Slice planes for cutting through geometry.
    pub slice_planes: HashMap<String, SlicePlane>,

    /// Gizmo configuration for transformation controls.
    pub gizmo_config: GizmoConfig,

    /// Currently selected structure (type_name, name) for gizmo operations.
    pub selected_structure: Option<(String, String)>,

    /// Global options.
    pub options: Options,

    /// Representative length scale for all registered structures.
    pub length_scale: f32,

    /// Axis-aligned bounding box for all registered structures.
    pub bounding_box: (Vec3, Vec3),
    // User callback will be added later with proper thread-safety handling
}

impl Default for Context {
    fn default() -> Self {
        Self {
            initialized: false,
            registry: Registry::new(),
            groups: HashMap::new(),
            slice_planes: HashMap::new(),
            gizmo_config: GizmoConfig::default(),
            selected_structure: None,
            options: Options::default(),
            length_scale: 1.0,
            bounding_box: (Vec3::ZERO, Vec3::ONE),
        }
    }
}

impl Context {
    /// Computes the center of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.bounding_box.0 + self.bounding_box.1) * 0.5
    }

    /// Updates the global bounding box and length scale from all structures.
    pub fn update_extents(&mut self) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut has_extent = false;

        for structure in self.registry.iter() {
            if let Some((bb_min, bb_max)) = structure.bounding_box() {
                min = min.min(bb_min);
                max = max.max(bb_max);
                has_extent = true;
            }
        }

        if has_extent {
            self.bounding_box = (min, max);
            self.length_scale = (max - min).length();
        } else {
            self.bounding_box = (Vec3::ZERO, Vec3::ONE);
            self.length_scale = 1.0;
        }
    }

    /// Creates a new group.
    pub fn create_group(&mut self, name: &str) -> &mut Group {
        self.groups
            .entry(name.to_string())
            .or_insert_with(|| Group::new(name))
    }

    /// Gets a group by name.
    pub fn get_group(&self, name: &str) -> Option<&Group> {
        self.groups.get(name)
    }

    /// Gets a mutable group by name.
    pub fn get_group_mut(&mut self, name: &str) -> Option<&mut Group> {
        self.groups.get_mut(name)
    }

    /// Removes a group by name.
    pub fn remove_group(&mut self, name: &str) -> Option<Group> {
        self.groups.remove(name)
    }

    /// Returns true if a group with the given name exists.
    pub fn has_group(&self, name: &str) -> bool {
        self.groups.contains_key(name)
    }

    /// Returns all group names.
    pub fn group_names(&self) -> Vec<&str> {
        self.groups.keys().map(|s| s.as_str()).collect()
    }

    /// Checks if a structure should be visible based on its group membership.
    ///
    /// A structure is visible if:
    /// - It's not in any group, or
    /// - All of its ancestor groups are enabled
    pub fn is_structure_visible_in_groups(&self, type_name: &str, name: &str) -> bool {
        // Find all groups that contain this structure
        for group in self.groups.values() {
            if group.contains_structure(type_name, name) {
                // Check if this group and all its ancestors are enabled
                if !self.is_group_and_ancestors_enabled(group.name()) {
                    return false;
                }
            }
        }
        true
    }

    /// Checks if a group and all its ancestors are enabled.
    fn is_group_and_ancestors_enabled(&self, group_name: &str) -> bool {
        let mut current = group_name;
        loop {
            if let Some(group) = self.groups.get(current) {
                if !group.is_enabled() {
                    return false;
                }
                if let Some(parent) = group.parent_group() {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        true
    }

    // ========================================================================
    // Slice Plane Management
    // ========================================================================

    /// Adds a new slice plane.
    pub fn add_slice_plane(&mut self, name: &str) -> &mut SlicePlane {
        self.slice_planes
            .entry(name.to_string())
            .or_insert_with(|| SlicePlane::new(name))
    }

    /// Gets a slice plane by name.
    pub fn get_slice_plane(&self, name: &str) -> Option<&SlicePlane> {
        self.slice_planes.get(name)
    }

    /// Gets a mutable slice plane by name.
    pub fn get_slice_plane_mut(&mut self, name: &str) -> Option<&mut SlicePlane> {
        self.slice_planes.get_mut(name)
    }

    /// Removes a slice plane by name.
    pub fn remove_slice_plane(&mut self, name: &str) -> Option<SlicePlane> {
        self.slice_planes.remove(name)
    }

    /// Returns true if a slice plane with the given name exists.
    pub fn has_slice_plane(&self, name: &str) -> bool {
        self.slice_planes.contains_key(name)
    }

    /// Returns all slice plane names.
    pub fn slice_plane_names(&self) -> Vec<&str> {
        self.slice_planes.keys().map(|s| s.as_str()).collect()
    }

    /// Returns the number of slice planes.
    pub fn num_slice_planes(&self) -> usize {
        self.slice_planes.len()
    }

    /// Returns an iterator over all slice planes.
    pub fn slice_planes(&self) -> impl Iterator<Item = &SlicePlane> {
        self.slice_planes.values()
    }

    /// Returns an iterator over all enabled slice planes.
    pub fn enabled_slice_planes(&self) -> impl Iterator<Item = &SlicePlane> {
        self.slice_planes.values().filter(|sp| sp.is_enabled())
    }

    // ========================================================================
    // Gizmo and Selection Management
    // ========================================================================

    /// Selects a structure for gizmo manipulation.
    pub fn select_structure(&mut self, type_name: &str, name: &str) {
        self.selected_structure = Some((type_name.to_string(), name.to_string()));
    }

    /// Deselects the current structure.
    pub fn deselect_structure(&mut self) {
        self.selected_structure = None;
    }

    /// Returns the currently selected structure, if any.
    pub fn selected_structure(&self) -> Option<(&str, &str)> {
        self.selected_structure
            .as_ref()
            .map(|(t, n)| (t.as_str(), n.as_str()))
    }

    /// Returns whether a structure is selected.
    pub fn has_selection(&self) -> bool {
        self.selected_structure.is_some()
    }

    /// Returns the gizmo configuration.
    pub fn gizmo(&self) -> &GizmoConfig {
        &self.gizmo_config
    }

    /// Returns the mutable gizmo configuration.
    pub fn gizmo_mut(&mut self) -> &mut GizmoConfig {
        &mut self.gizmo_config
    }
}

/// Initializes the global context.
///
/// This should be called once at the start of the program.
pub fn init_context() -> Result<()> {
    let context = RwLock::new(Context::default());

    CONTEXT
        .set(context)
        .map_err(|_| PolyscopeError::AlreadyInitialized)?;

    with_context_mut(|ctx| {
        ctx.initialized = true;
    });

    Ok(())
}

/// Returns whether the context has been initialized.
pub fn is_initialized() -> bool {
    CONTEXT
        .get()
        .and_then(|lock| lock.read().ok())
        .map_or(false, |ctx| ctx.initialized)
}

/// Access the global context for reading.
///
/// # Panics
///
/// Panics if polyscope has not been initialized.
pub fn with_context<F, R>(f: F) -> R
where
    F: FnOnce(&Context) -> R,
{
    let lock = CONTEXT.get().expect("polyscope not initialized");
    let guard = lock.read().expect("context lock poisoned");
    f(&guard)
}

/// Access the global context for writing.
///
/// # Panics
///
/// Panics if polyscope has not been initialized.
pub fn with_context_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Context) -> R,
{
    let lock = CONTEXT.get().expect("polyscope not initialized");
    let mut guard = lock.write().expect("context lock poisoned");
    f(&mut guard)
}

/// Try to access the global context for reading.
///
/// Returns `None` if polyscope has not been initialized.
pub fn try_with_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Context) -> R,
{
    let lock = CONTEXT.get()?;
    let guard = lock.read().ok()?;
    Some(f(&guard))
}

/// Try to access the global context for writing.
///
/// Returns `None` if polyscope has not been initialized.
pub fn try_with_context_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Context) -> R,
{
    let lock = CONTEXT.get()?;
    let mut guard = lock.write().ok()?;
    Some(f(&mut guard))
}

/// Shuts down the global context.
///
/// Note: Due to `OnceLock` semantics, the context cannot be re-initialized
/// after shutdown in the same process.
pub fn shutdown_context() {
    if let Some(lock) = CONTEXT.get() {
        if let Ok(mut ctx) = lock.write() {
            ctx.initialized = false;
            ctx.registry.clear();
            ctx.groups.clear();
            ctx.slice_planes.clear();
        }
    }
}
