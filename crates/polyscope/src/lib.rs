//! polyscope-rs: A Rust-native 3D visualization library for geometric data.
//!
//! Polyscope is a viewer and user interface for 3D data such as meshes and point clouds.
//! It allows you to register your data and quickly generate informative visualizations.
//!
//! # Quick Start
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     // Initialize polyscope
//!     init()?;
//!
//!     // Register a point cloud
//!     let points = vec![
//!         Vec3::new(0.0, 0.0, 0.0),
//!         Vec3::new(1.0, 0.0, 0.0),
//!         Vec3::new(0.0, 1.0, 0.0),
//!     ];
//!     register_point_cloud("my points", points);
//!
//!     // Show the viewer
//!     show();
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! Polyscope uses a paradigm of **structures** and **quantities**:
//!
//! - A **structure** is a geometric object in the scene (point cloud, mesh, etc.)
//! - A **quantity** is data associated with a structure (scalar field, vector field, colors)
//!
//! # Structures
//!
//! - [`PointCloud`] - A set of points in 3D space
//! - [`SurfaceMesh`] - A triangular or polygonal mesh
//! - [`CurveNetwork`] - A network of curves/edges
//! - [`VolumeMesh`] - A tetrahedral or hexahedral mesh
//! - [`VolumeGrid`] - A regular grid of values (for implicit surfaces)
//! - [`CameraView`] - A camera frustum visualization

mod app;

// Re-export core types
pub use polyscope_core::{
    error::{PolyscopeError, Result},
    options::Options,
    pick::{PickResult, Pickable},
    quantity::{Quantity, QuantityKind},
    registry::Registry,
    state::{with_context, with_context_mut, Context},
    structure::{HasQuantities, Structure},
    Mat4, Vec2, Vec3, Vec4,
};

// Re-export render types
pub use polyscope_render::{
    Camera, ColorMap, ColorMapRegistry, Material, MaterialRegistry, RenderContext, RenderEngine,
};

// Re-export structures
pub use polyscope_structures::{
    CameraView, CurveNetwork, PointCloud, SurfaceMesh, VolumeGrid, VolumeMesh,
};

use glam::UVec3;

/// Initializes polyscope with default settings.
///
/// This must be called before any other polyscope functions.
pub fn init() -> Result<()> {
    polyscope_core::state::init_context()?;
    log::info!("polyscope-rs initialized");
    Ok(())
}

/// Returns whether polyscope has been initialized.
pub fn is_initialized() -> bool {
    polyscope_core::state::is_initialized()
}

/// Shuts down polyscope and releases all resources.
pub fn shutdown() {
    polyscope_core::state::shutdown_context();
    log::info!("polyscope-rs shut down");
}

/// Shows the polyscope viewer window.
///
/// This function blocks until the window is closed.
pub fn show() {
    let _ = env_logger::try_init();
    app::run_app();
}

/// Performs one iteration of the main loop.
///
/// Use this for integration with external event loops.
pub fn frame_tick() {
    // TODO: Implement frame tick
}

/// Registers a point cloud with polyscope.
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

/// Registers a surface mesh with polyscope.
pub fn register_surface_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    faces: Vec<UVec3>,
) -> SurfaceMeshHandle {
    let name = name.into();
    // Convert UVec3 faces to Vec<Vec<u32>> for the SurfaceMesh constructor
    let faces: Vec<Vec<u32>> = faces
        .into_iter()
        .map(|f| vec![f.x, f.y, f.z])
        .collect();
    let mesh = SurfaceMesh::new(name.clone(), vertices, faces);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register surface mesh");
        ctx.update_extents();
    });

    SurfaceMeshHandle { name }
}

/// Gets a registered point cloud by name.
pub fn get_point_cloud(name: &str) -> Option<PointCloudHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("PointCloud", name) {
            Some(PointCloudHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Gets a registered surface mesh by name.
pub fn get_surface_mesh(name: &str) -> Option<SurfaceMeshHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("SurfaceMesh", name) {
            Some(SurfaceMeshHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Removes a structure by name.
pub fn remove_structure(name: &str) {
    with_context_mut(|ctx| {
        // Try removing from each structure type
        ctx.registry.remove("PointCloud", name);
        ctx.registry.remove("SurfaceMesh", name);
        ctx.registry.remove("CurveNetwork", name);
        ctx.registry.remove("VolumeMesh", name);
        ctx.registry.remove("VolumeGrid", name);
        ctx.registry.remove("CameraView", name);
        ctx.update_extents();
    });
}

/// Removes all structures.
pub fn remove_all_structures() {
    with_context_mut(|ctx| {
        ctx.registry.clear();
        ctx.update_extents();
    });
}

/// Requests a redraw of the scene.
pub fn request_redraw() {
    // TODO: Implement redraw request
}

/// Handle for a registered point cloud.
#[derive(Clone)]
pub struct PointCloudHandle {
    name: String,
}

impl PointCloudHandle {
    /// Returns the name of this point cloud.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a scalar quantity to this point cloud.
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

/// Handle for a registered surface mesh.
#[derive(Clone)]
pub struct SurfaceMeshHandle {
    name: String,
}

impl SurfaceMeshHandle {
    /// Returns the name of this mesh.
    pub fn name(&self) -> &str {
        &self.name
    }

    // TODO: Add quantity methods
}
