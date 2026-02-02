//! Group management for organizing structures.
//!
//! Groups provide hierarchical organization of structures. When a group is
//! disabled, all structures within it (and child groups) are hidden.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     // Create structures
//!     register_point_cloud("points1", vec![Vec3::ZERO]);
//!     register_point_cloud("points2", vec![Vec3::X]);
//!
//!     // Organize into a group
//!     let group = create_group("my group");
//!     group.add_point_cloud("points1");
//!     group.add_point_cloud("points2");
//!
//!     // Toggle visibility of entire group
//!     group.set_enabled(false);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{with_context, with_context_mut};

/// Creates a new group for organizing structures.
///
/// Groups allow organizing structures hierarchically. When a group is disabled,
/// all structures and child groups within it are hidden.
pub fn create_group(name: impl Into<String>) -> GroupHandle {
    let name = name.into();
    with_context_mut(|ctx| {
        ctx.create_group(&name);
    });
    GroupHandle { name }
}

/// Gets an existing group by name.
#[must_use]
pub fn get_group(name: &str) -> Option<GroupHandle> {
    with_context(|ctx| {
        if ctx.has_group(name) {
            Some(GroupHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Removes a group by name.
///
/// Note: This does not remove structures from the group, only the group itself.
pub fn remove_group(name: &str) {
    with_context_mut(|ctx| {
        ctx.remove_group(name);
    });
}

/// Returns all group names.
#[must_use]
pub fn get_all_groups() -> Vec<String> {
    with_context(|ctx| {
        ctx.group_names()
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Handle for a group.
#[derive(Clone)]
pub struct GroupHandle {
    name: String,
}

impl GroupHandle {
    /// Returns the name of this group.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets whether this group is enabled (visible).
    pub fn set_enabled(&self, enabled: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.set_enabled(enabled);
            }
        });
        self
    }

    /// Returns whether this group is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .is_some_and(polyscope_core::Group::is_enabled)
        })
    }

    /// Sets whether child details are shown in UI.
    pub fn set_show_child_details(&self, show: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.set_show_child_details(show);
            }
        });
        self
    }

    /// Adds a point cloud to this group.
    pub fn add_point_cloud(&self, pc_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("PointCloud", pc_name);
            }
        });
        self
    }

    /// Adds a surface mesh to this group.
    pub fn add_surface_mesh(&self, mesh_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("SurfaceMesh", mesh_name);
            }
        });
        self
    }

    /// Adds a curve network to this group.
    pub fn add_curve_network(&self, cn_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("CurveNetwork", cn_name);
            }
        });
        self
    }

    /// Adds a volume mesh to this group.
    pub fn add_volume_mesh(&self, vm_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("VolumeMesh", vm_name);
            }
        });
        self
    }

    /// Adds a volume grid to this group.
    pub fn add_volume_grid(&self, vg_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("VolumeGrid", vg_name);
            }
        });
        self
    }

    /// Adds a camera view to this group.
    pub fn add_camera_view(&self, cv_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("CameraView", cv_name);
            }
        });
        self
    }

    /// Removes a structure from this group.
    pub fn remove_structure(&self, type_name: &str, name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.remove_structure(type_name, name);
            }
        });
        self
    }

    /// Adds a child group to this group.
    pub fn add_child_group(&self, child_name: &str) -> &Self {
        with_context_mut(|ctx| {
            // Set parent on child group
            if let Some(child) = ctx.get_group_mut(child_name) {
                child.set_parent_group(Some(self.name.clone()));
            }
            // Add child reference to this group
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_child_group(child_name);
            }
        });
        self
    }

    /// Removes a child group from this group.
    pub fn remove_child_group(&self, child_name: &str) -> &Self {
        with_context_mut(|ctx| {
            // Remove parent from child group
            if let Some(child) = ctx.get_group_mut(child_name) {
                child.set_parent_group(None);
            }
            // Remove child reference from this group
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.remove_child_group(child_name);
            }
        });
        self
    }

    /// Returns the number of structures in this group.
    #[must_use]
    pub fn num_structures(&self) -> usize {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .map_or(0, polyscope_core::Group::num_child_structures)
        })
    }

    /// Returns the number of child groups.
    #[must_use]
    pub fn num_child_groups(&self) -> usize {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .map_or(0, polyscope_core::Group::num_child_groups)
        })
    }
}
