//! Camera view registration and manipulation.
//!
//! Camera views visualize camera poses as frustum widgets in the scene.
//! Useful for visualizing camera trajectories, multi-view setups, or
//! debugging camera calibration.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     let cam = register_camera_view_look_at(
//!         "my camera",
//!         Vec3::new(2.0, 1.0, 2.0),  // position
//!         Vec3::ZERO,                 // target
//!         Vec3::Y,                    // up
//!         60.0,                       // fov (degrees)
//!         1.5,                        // aspect ratio
//!     );
//!     cam.set_widget_focal_length(0.3, false); // absolute length
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{with_context, with_context_mut, CameraParameters, CameraView, Vec3};

/// Registers a camera view with polyscope using camera parameters.
pub fn register_camera_view(name: impl Into<String>, params: CameraParameters) -> CameraViewHandle {
    let name = name.into();
    let camera_view = CameraView::new(name.clone(), params);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(camera_view))
            .expect("failed to register camera view");
        ctx.update_extents();
    });

    CameraViewHandle { name }
}

/// Registers a camera view from position, target, and up direction.
pub fn register_camera_view_look_at(
    name: impl Into<String>,
    position: Vec3,
    target: Vec3,
    up: Vec3,
    fov_vertical_degrees: f32,
    aspect_ratio: f32,
) -> CameraViewHandle {
    let params =
        CameraParameters::look_at(position, target, up, fov_vertical_degrees, aspect_ratio);
    register_camera_view(name, params)
}

/// Gets a registered camera view by name.
#[must_use]
pub fn get_camera_view(name: &str) -> Option<CameraViewHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("CameraView", name) {
            Some(CameraViewHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered camera view.
#[derive(Clone)]
pub struct CameraViewHandle {
    name: String,
}

impl CameraViewHandle {
    /// Returns the name of this camera view.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the widget color.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_color(color);
        });
        self
    }

    /// Sets the widget focal length.
    pub fn set_widget_focal_length(&self, length: f32, is_relative: bool) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_widget_focal_length(length, is_relative);
        });
        self
    }

    /// Sets the widget thickness.
    pub fn set_widget_thickness(&self, thickness: f32) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_widget_thickness(thickness);
        });
        self
    }

    /// Updates the camera parameters.
    pub fn set_params(&self, params: CameraParameters) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_params(params);
        });
        self
    }
}

/// Executes a closure with mutable access to a registered camera view.
///
/// Returns `None` if the camera view does not exist.
pub fn with_camera_view<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut CameraView) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("CameraView", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<CameraView>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered camera view.
///
/// Returns `None` if the camera view does not exist.
pub fn with_camera_view_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&CameraView) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("CameraView", name)
            .and_then(|s| s.as_any().downcast_ref::<CameraView>())
            .map(f)
    })
}
