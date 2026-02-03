//! Point cloud registration and manipulation.
//!
//! Point clouds are sets of points in 3D space, rendered as spheres.
//! This module provides functions to register point clouds and add
//! quantities (scalar, vector, color) to them.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     let points = vec![
//!         Vec3::new(0.0, 0.0, 0.0),
//!         Vec3::new(1.0, 0.0, 0.0),
//!         Vec3::new(0.5, 1.0, 0.0),
//!     ];
//!
//!     let pc = register_point_cloud("my points", points);
//!     pc.add_scalar_quantity("height", vec![0.0, 0.0, 1.0]);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{PointCloud, Vec3, with_context_mut};

/// Registers a point cloud with polyscope.
///
/// Creates a new point cloud structure from the given points and registers
/// it with the global state. Returns a handle for adding quantities and
/// configuring appearance.
///
/// # Arguments
///
/// * `name` - Unique name for this point cloud
/// * `points` - Vector of 3D point positions
///
/// # Panics
///
/// Panics if a structure with the same name already exists.
///
/// # Example
///
/// ```no_run
/// use polyscope::*;
///
/// init().unwrap();
/// let pc = register_point_cloud("my points", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
/// pc.add_scalar_quantity("values", vec![0.0, 0.5, 1.0]);
/// ```
pub fn register_point_cloud(name: impl Into<String>, points: Vec<Vec3>) -> PointCloudHandle {
    let name = name.into();
    let point_cloud = PointCloud::new(name.clone(), points);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(point_cloud))
            .expect("failed to register point cloud");
        ctx.update_extents();
    });

    PointCloudHandle { name }
}

impl_structure_accessors! {
    get_fn = get_point_cloud,
    with_fn = with_point_cloud,
    with_ref_fn = with_point_cloud_ref,
    handle = PointCloudHandle,
    type_name = "PointCloud",
    rust_type = PointCloud,
    doc_name = "point cloud"
}

/// Handle for a registered point cloud.
///
/// This handle provides methods to add quantities and configure the
/// appearance of a point cloud. Methods return `&Self` to allow chaining.
///
/// # Example
///
/// ```no_run
/// use polyscope::*;
///
/// init().unwrap();
/// register_point_cloud("pts", vec![Vec3::ZERO, Vec3::X])
///     .add_scalar_quantity("height", vec![0.0, 1.0])
///     .add_vector_quantity("velocity", vec![Vec3::X, Vec3::Y]);
/// ```
#[derive(Clone)]
pub struct PointCloudHandle {
    name: String,
}

impl PointCloudHandle {
    /// Returns the name of this point cloud.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a scalar quantity to this point cloud.
    ///
    /// Scalar quantities assign a single value to each point, visualized
    /// using a colormap. The values vector must have the same length as
    /// the number of points.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for this quantity (shown in UI)
    /// * `values` - One scalar value per point
    pub fn add_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_scalar_quantity(name, values);
                }
            }
        });
        self
    }

    /// Adds a vector quantity to this point cloud.
    ///
    /// Vector quantities display an arrow at each point. Vectors are
    /// automatically scaled based on scene size. The vectors array must
    /// have the same length as the number of points.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for this quantity (shown in UI)
    /// * `vectors` - One 3D vector per point
    pub fn add_vector_quantity(&self, name: &str, vectors: Vec<Vec3>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_vector_quantity(name, vectors);
                }
            }
        });
        self
    }

    /// Adds a color quantity to this point cloud.
    ///
    /// Color quantities assign an RGB color to each point. The colors
    /// vector must have the same length as the number of points.
    /// Components should be in range [0, 1].
    ///
    /// # Arguments
    ///
    /// * `name` - Name for this quantity (shown in UI)
    /// * `colors` - One RGB color (Vec3) per point
    pub fn add_color_quantity(&self, name: &str, colors: Vec<Vec3>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_color_quantity(name, colors);
                }
            }
        });
        self
    }
}
