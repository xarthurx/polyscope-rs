//! Slice plane management.
//!
//! Slice planes cut through geometry to reveal interior structure.
//! They can be positioned interactively via gizmos or programmatically.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     // Register some geometry first...
//!     register_point_cloud("points", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
//!
//!     // Add a slice plane
//!     let plane = add_slice_plane("my slice");
//!     plane.set_pose(Vec3::ZERO, Vec3::X); // origin and normal
//!     plane.set_draw_plane(true);
//!     plane.set_draw_widget(true);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{Vec3, Vec4, with_context, with_context_mut};

/// Adds a new slice plane to cut through geometry.
///
/// Slice planes allow visualizing the interior of 3D geometry by
/// discarding fragments on one side of the plane.
///
/// The plane is created at the scene center with a size proportional to the
/// scene's length scale, ensuring it's visible regardless of the scene scale.
pub fn add_slice_plane(name: impl Into<String>) -> SlicePlaneHandle {
    let name = name.into();
    with_context_mut(|ctx| {
        let length_scale = ctx.length_scale;
        // Get scene center before creating the plane (to avoid borrow issues)
        let center = (ctx.bounding_box.0 + ctx.bounding_box.1) * 0.5;
        let plane = ctx.add_slice_plane(&name);
        // Set plane_size to be visible relative to the scene
        // Using length_scale * 0.25 gives a reasonably sized plane
        plane.set_plane_size(length_scale * 0.25);
        // Position the plane at the scene center
        plane.set_origin(center);
    });
    SlicePlaneHandle { name }
}

/// Adds a slice plane with a specific pose.
pub fn add_slice_plane_with_pose(
    name: impl Into<String>,
    origin: Vec3,
    normal: Vec3,
) -> SlicePlaneHandle {
    let name = name.into();
    with_context_mut(|ctx| {
        let plane = ctx.add_slice_plane(&name);
        plane.set_pose(origin, normal);
    });
    SlicePlaneHandle { name }
}

/// Gets an existing slice plane by name.
#[must_use]
pub fn get_slice_plane(name: &str) -> Option<SlicePlaneHandle> {
    with_context(|ctx| {
        if ctx.has_slice_plane(name) {
            Some(SlicePlaneHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Removes a slice plane by name.
pub fn remove_slice_plane(name: &str) {
    with_context_mut(|ctx| {
        ctx.remove_slice_plane(name);
    });
}

/// Removes all slice planes.
pub fn remove_all_slice_planes() {
    with_context_mut(|ctx| {
        ctx.slice_planes.clear();
    });
}

/// Returns all slice plane names.
#[must_use]
pub fn get_all_slice_planes() -> Vec<String> {
    with_context(|ctx| {
        ctx.slice_plane_names()
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect()
    })
}

/// Handle for a slice plane.
#[derive(Clone)]
pub struct SlicePlaneHandle {
    name: String,
}

impl SlicePlaneHandle {
    /// Returns the name of this slice plane.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the pose (origin and normal) of the slice plane.
    pub fn set_pose(&self, origin: Vec3, normal: Vec3) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_pose(origin, normal);
            }
        });
        self
    }

    /// Sets the origin point of the plane.
    pub fn set_origin(&self, origin: Vec3) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_origin(origin);
            }
        });
        self
    }

    /// Gets the origin point of the plane.
    #[must_use]
    pub fn origin(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map_or(Vec3::ZERO, polyscope_core::SlicePlane::origin)
        })
    }

    /// Sets the normal direction of the plane.
    pub fn set_normal(&self, normal: Vec3) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_normal(normal);
            }
        });
        self
    }

    /// Gets the normal direction of the plane.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map_or(Vec3::Y, polyscope_core::SlicePlane::normal)
        })
    }

    /// Sets whether the slice plane is enabled.
    pub fn set_enabled(&self, enabled: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_enabled(enabled);
            }
        });
        self
    }

    /// Returns whether the slice plane is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .is_some_and(polyscope_core::SlicePlane::is_enabled)
        })
    }

    /// Sets whether to draw the plane visualization.
    pub fn set_draw_plane(&self, draw: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_draw_plane(draw);
            }
        });
        self
    }

    /// Returns whether the plane visualization is drawn.
    #[must_use]
    pub fn draw_plane(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .is_some_and(polyscope_core::SlicePlane::draw_plane)
        })
    }

    /// Sets whether to draw the widget.
    pub fn set_draw_widget(&self, draw: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_draw_widget(draw);
            }
        });
        self
    }

    /// Returns whether the widget is drawn.
    #[must_use]
    pub fn draw_widget(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .is_some_and(polyscope_core::SlicePlane::draw_widget)
        })
    }

    /// Sets the color of the plane visualization.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_color(color);
            }
        });
        self
    }

    /// Gets the color of the plane visualization.
    #[must_use]
    pub fn color(&self) -> Vec4 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name).map_or(
                Vec4::new(0.5, 0.5, 0.5, 1.0),
                polyscope_core::SlicePlane::color,
            )
        })
    }

    /// Sets the transparency of the plane visualization.
    pub fn set_transparency(&self, transparency: f32) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_transparency(transparency);
            }
        });
        self
    }

    /// Gets the transparency of the plane visualization.
    #[must_use]
    pub fn transparency(&self) -> f32 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map_or(0.3, polyscope_core::SlicePlane::transparency)
        })
    }

    /// Sets the size of the plane visualization (half-extent in each direction).
    pub fn set_plane_size(&self, size: f32) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_plane_size(size);
            }
        });
        self
    }

    /// Gets the size of the plane visualization (half-extent in each direction).
    #[must_use]
    pub fn plane_size(&self) -> f32 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map_or(0.1, polyscope_core::SlicePlane::plane_size)
        })
    }
}
