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
    Camera, ColorMap, ColorMapRegistry, Material, MaterialRegistry, PickElementType, RenderContext,
    RenderEngine,
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
    let faces: Vec<Vec<u32>> = faces.into_iter().map(|f| vec![f.x, f.y, f.z]).collect();
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

/// Executes a closure with mutable access to a registered surface mesh.
///
/// Returns `None` if the mesh does not exist.
pub fn with_surface_mesh<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut SurfaceMesh) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("SurfaceMesh", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<SurfaceMesh>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered surface mesh.
///
/// Returns `None` if the mesh does not exist.
pub fn with_surface_mesh_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&SurfaceMesh) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("SurfaceMesh", name)
            .and_then(|s| s.as_any().downcast_ref::<SurfaceMesh>())
            .map(f)
    })
}

// ============================================================================
// CurveNetwork Registration
// ============================================================================

/// Registers a curve network with explicit edges.
pub fn register_curve_network(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
    edges: Vec<[u32; 2]>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new(name.clone(), nodes, edges);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a connected line (0-1-2-3-...).
pub fn register_curve_network_line(name: impl Into<String>, nodes: Vec<Vec3>) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_line(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a closed loop (0-1-2-...-n-0).
pub fn register_curve_network_loop(name: impl Into<String>, nodes: Vec<Vec3>) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_loop(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as separate segments (0-1, 2-3, 4-5, ...).
pub fn register_curve_network_segments(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_segments(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Gets a registered curve network by name.
pub fn get_curve_network(name: &str) -> Option<CurveNetworkHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("CurveNetwork", name) {
            Some(CurveNetworkHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered curve network.
#[derive(Clone)]
pub struct CurveNetworkHandle {
    name: String,
}

impl CurveNetworkHandle {
    /// Returns the name of this curve network.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_color(color);
        });
        self
    }

    /// Sets the radius.
    pub fn set_radius(&self, radius: f32, is_relative: bool) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_radius(radius, is_relative);
        });
        self
    }

    /// Sets the material.
    pub fn set_material(&self, material: &str) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_material(material);
        });
        self
    }
}

/// Executes a closure with mutable access to a registered curve network.
///
/// Returns `None` if the curve network does not exist.
pub fn with_curve_network<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut CurveNetwork) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("CurveNetwork", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<CurveNetwork>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered curve network.
///
/// Returns `None` if the curve network does not exist.
pub fn with_curve_network_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&CurveNetwork) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .and_then(|s| s.as_any().downcast_ref::<CurveNetwork>())
            .map(f)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Counter for unique test names to avoid race conditions
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn unique_name(prefix: &str) -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{prefix}_{n}")
    }

    fn setup() {
        // Initialize context (only once)
        if !is_initialized() {
            init().unwrap();
        }
    }

    #[test]
    fn test_register_curve_network() {
        setup();
        let name = unique_name("test_cn");
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let edges = vec![[0, 1], [1, 2]];

        let handle = register_curve_network(&name, nodes, edges);
        assert_eq!(handle.name(), name);

        // Verify it's retrievable
        let found = get_curve_network(&name);
        assert!(found.is_some());

        // Verify non-existent returns None
        let not_found = get_curve_network("nonexistent_xyz_123");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_register_curve_network_line() {
        setup();
        let name = unique_name("line");
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];

        register_curve_network_line(&name, nodes);

        let num_edges = with_curve_network_ref(&name, |cn| cn.num_edges());
        assert_eq!(num_edges, Some(3)); // 0-1, 1-2, 2-3
    }

    #[test]
    fn test_register_curve_network_loop() {
        setup();
        let name = unique_name("loop");
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];

        register_curve_network_loop(&name, nodes);

        let num_edges = with_curve_network_ref(&name, |cn| cn.num_edges());
        assert_eq!(num_edges, Some(3)); // 0-1, 1-2, 2-0
    }

    #[test]
    fn test_register_curve_network_segments() {
        setup();
        let name = unique_name("segs");
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];

        register_curve_network_segments(&name, nodes);

        let num_edges = with_curve_network_ref(&name, |cn| cn.num_edges());
        assert_eq!(num_edges, Some(2)); // 0-1, 2-3
    }

    #[test]
    fn test_curve_network_handle_methods() {
        setup();
        let name = unique_name("handle_test");
        let nodes = vec![Vec3::ZERO, Vec3::X];
        let edges = vec![[0, 1]];

        let handle = register_curve_network(&name, nodes, edges);

        // Test chained setters
        handle
            .set_color(Vec3::new(1.0, 0.0, 0.0))
            .set_radius(0.1, false)
            .set_material("clay");

        // Verify values were set
        with_curve_network_ref(&name, |cn| {
            assert_eq!(cn.color(), Vec3::new(1.0, 0.0, 0.0));
            assert_eq!(cn.radius(), 0.1);
            assert!(!cn.radius_is_relative());
            assert_eq!(cn.material(), "clay");
        });
    }

    #[test]
    fn test_with_curve_network() {
        setup();
        let name = unique_name("with_test");
        let nodes = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
        let edges = vec![[0, 1], [1, 2]];

        register_curve_network(&name, nodes, edges);

        // Test mutable access
        let result = with_curve_network(&name, |cn| {
            cn.set_color(Vec3::new(0.5, 0.5, 0.5));
            cn.num_nodes()
        });
        assert_eq!(result, Some(3));

        // Verify mutation persisted
        let color = with_curve_network_ref(&name, |cn| cn.color());
        assert_eq!(color, Some(Vec3::new(0.5, 0.5, 0.5)));
    }
}
