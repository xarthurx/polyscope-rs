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
    gizmo::{GizmoAxis, GizmoConfig, GizmoMode, GizmoSpace, Transform},
    group::Group,
    options::Options,
    pick::{PickResult, Pickable},
    quantity::{Quantity, QuantityKind},
    registry::Registry,
    slice_plane::{SlicePlane, SlicePlaneUniforms, MAX_SLICE_PLANES},
    state::{with_context, with_context_mut, Context},
    structure::{HasQuantities, Structure},
    Mat4, Vec2, Vec3, Vec4,
};

// Re-export render types
pub use polyscope_render::{
    AxisDirection, Camera, ColorMap, ColorMapRegistry, Material, MaterialRegistry, NavigationStyle,
    PickElementType, ProjectionMode, RenderContext, RenderEngine, ScreenshotError,
    ScreenshotOptions,
};

// Re-export UI types
pub use polyscope_ui::{
    AppearanceSettings, CameraSettings, SceneExtents, SlicePlaneSettings, SlicePlanesAction,
};

// Re-export structures
pub use polyscope_structures::{
    CameraExtrinsics, CameraIntrinsics, CameraParameters, CameraView, CurveNetwork, PointCloud,
    SurfaceMesh, VolumeCellType, VolumeGrid, VolumeMesh,
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
pub fn register_curve_network_line(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
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
pub fn register_curve_network_loop(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
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

// ============================================================================
// CameraView Registration
// ============================================================================

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

// ============================================================================
// VolumeGrid Registration
// ============================================================================

/// Registers a volume grid with polyscope.
pub fn register_volume_grid(
    name: impl Into<String>,
    node_dim: glam::UVec3,
    bound_min: Vec3,
    bound_max: Vec3,
) -> VolumeGridHandle {
    let name = name.into();
    let grid = VolumeGrid::new(name.clone(), node_dim, bound_min, bound_max);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(grid))
            .expect("failed to register volume grid");
        ctx.update_extents();
    });

    VolumeGridHandle { name }
}

/// Registers a volume grid with uniform dimensions.
pub fn register_volume_grid_uniform(
    name: impl Into<String>,
    dim: u32,
    bound_min: Vec3,
    bound_max: Vec3,
) -> VolumeGridHandle {
    register_volume_grid(name, glam::UVec3::splat(dim), bound_min, bound_max)
}

/// Gets a registered volume grid by name.
pub fn get_volume_grid(name: &str) -> Option<VolumeGridHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("VolumeGrid", name) {
            Some(VolumeGridHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered volume grid.
#[derive(Clone)]
pub struct VolumeGridHandle {
    name: String,
}

impl VolumeGridHandle {
    /// Returns the name of this volume grid.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the edge color.
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_width(width);
        });
        self
    }

    /// Adds a node scalar quantity.
    pub fn add_node_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.add_node_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.add_cell_scalar_quantity(name, values);
        });
        self
    }
}

/// Executes a closure with mutable access to a registered volume grid.
///
/// Returns `None` if the volume grid does not exist.
pub fn with_volume_grid<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut VolumeGrid) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("VolumeGrid", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<VolumeGrid>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered volume grid.
///
/// Returns `None` if the volume grid does not exist.
pub fn with_volume_grid_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&VolumeGrid) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("VolumeGrid", name)
            .and_then(|s| s.as_any().downcast_ref::<VolumeGrid>())
            .map(f)
    })
}

// ============================================================================
// VolumeMesh Registration
// ============================================================================

/// Registers a tetrahedral mesh with polyscope.
pub fn register_tet_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    tets: Vec<[u32; 4]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_tet_mesh(name.clone(), vertices, tets);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register tet mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Registers a hexahedral mesh with polyscope.
pub fn register_hex_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    hexes: Vec<[u32; 8]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_hex_mesh(name.clone(), vertices, hexes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register hex mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Registers a generic volume mesh with polyscope.
///
/// Cells are stored as 8-index arrays. For tetrahedra, indices 4-7 should be u32::MAX.
pub fn register_volume_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    cells: Vec<[u32; 8]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new(name.clone(), vertices, cells);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register volume mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Gets a registered volume mesh by name.
pub fn get_volume_mesh(name: &str) -> Option<VolumeMeshHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("VolumeMesh", name) {
            Some(VolumeMeshHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered volume mesh.
#[derive(Clone)]
pub struct VolumeMeshHandle {
    name: String,
}

impl VolumeMeshHandle {
    /// Returns the name of this volume mesh.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_color(color);
        });
        self
    }

    /// Sets the interior color.
    pub fn set_interior_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_interior_color(color);
        });
        self
    }

    /// Sets the edge color.
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_width(width);
        });
        self
    }
}

/// Executes a closure with mutable access to a registered volume mesh.
///
/// Returns `None` if the volume mesh does not exist.
pub fn with_volume_mesh<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut VolumeMesh) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("VolumeMesh", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<VolumeMesh>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered volume mesh.
///
/// Returns `None` if the volume mesh does not exist.
pub fn with_volume_mesh_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&VolumeMesh) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("VolumeMesh", name)
            .and_then(|s| s.as_any().downcast_ref::<VolumeMesh>())
            .map(f)
    })
}

// ============================================================================
// Groups API
// ============================================================================

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
pub fn get_all_groups() -> Vec<String> {
    with_context(|ctx| {
        ctx.group_names()
            .into_iter()
            .map(|s| s.to_string())
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
    pub fn is_enabled(&self) -> bool {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .map(|g| g.is_enabled())
                .unwrap_or(false)
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
    pub fn num_structures(&self) -> usize {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .map(|g| g.num_child_structures())
                .unwrap_or(0)
        })
    }

    /// Returns the number of child groups.
    pub fn num_child_groups(&self) -> usize {
        with_context(|ctx| {
            ctx.get_group(&self.name)
                .map(|g| g.num_child_groups())
                .unwrap_or(0)
        })
    }
}

// ============================================================================
// Slice Planes API
// ============================================================================

/// Adds a new slice plane to cut through geometry.
///
/// Slice planes allow visualizing the interior of 3D geometry by
/// discarding fragments on one side of the plane.
pub fn add_slice_plane(name: impl Into<String>) -> SlicePlaneHandle {
    let name = name.into();
    with_context_mut(|ctx| {
        ctx.add_slice_plane(&name);
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
pub fn get_all_slice_planes() -> Vec<String> {
    with_context(|ctx| {
        ctx.slice_plane_names()
            .into_iter()
            .map(|s| s.to_string())
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
    pub fn origin(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.origin())
                .unwrap_or(Vec3::ZERO)
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
    pub fn normal(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.normal())
                .unwrap_or(Vec3::Y)
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
    pub fn is_enabled(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.is_enabled())
                .unwrap_or(false)
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
    pub fn draw_plane(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.draw_plane())
                .unwrap_or(false)
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
    pub fn draw_widget(&self) -> bool {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.draw_widget())
                .unwrap_or(false)
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
    pub fn color(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.color())
                .unwrap_or(Vec3::new(0.5, 0.5, 0.5))
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
    pub fn transparency(&self) -> f32 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map(|p| p.transparency())
                .unwrap_or(0.3)
        })
    }
}

// ============================================================================
// Selection and Gizmo API
// ============================================================================

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
pub fn get_selected_structure() -> Option<(String, String)> {
    with_context(|ctx| {
        ctx.selected_structure()
            .map(|(t, n)| (t.to_string(), n.to_string()))
    })
}

/// Returns whether any structure is currently selected.
pub fn has_selection() -> bool {
    with_context(|ctx| ctx.has_selection())
}

/// Sets the gizmo mode (translate, rotate, or scale).
pub fn set_gizmo_mode(mode: GizmoMode) {
    with_context_mut(|ctx| {
        ctx.gizmo_mut().mode = mode;
    });
}

/// Returns the current gizmo mode.
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
pub fn get_selected_transform() -> Mat4 {
    with_context(|ctx| {
        if let Some((type_name, name)) = ctx.selected_structure() {
            ctx.registry
                .get(type_name, name)
                .map(|s| s.transform())
                .unwrap_or(Mat4::IDENTITY)
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

// ============================================================================
// Structure Transform API (for individual structures)
// ============================================================================

/// Sets the transform of a point cloud by name.
pub fn set_point_cloud_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(pc) = ctx.registry.get_mut("PointCloud", name) {
            pc.set_transform(transform);
        }
    });
}

/// Gets the transform of a point cloud by name.
pub fn get_point_cloud_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("PointCloud", name)
            .map(|pc| pc.transform())
    })
}

/// Sets the transform of a surface mesh by name.
pub fn set_surface_mesh_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(mesh) = ctx.registry.get_mut("SurfaceMesh", name) {
            mesh.set_transform(transform);
        }
    });
}

/// Gets the transform of a surface mesh by name.
pub fn get_surface_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| ctx.registry.get("SurfaceMesh", name).map(|m| m.transform()))
}

/// Sets the transform of a curve network by name.
pub fn set_curve_network_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(cn) = ctx.registry.get_mut("CurveNetwork", name) {
            cn.set_transform(transform);
        }
    });
}

/// Gets the transform of a curve network by name.
pub fn get_curve_network_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .map(|cn| cn.transform())
    })
}

/// Sets the transform of a volume mesh by name.
pub fn set_volume_mesh_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(vm) = ctx.registry.get_mut("VolumeMesh", name) {
            vm.set_transform(transform);
        }
    });
}

/// Gets the transform of a volume mesh by name.
pub fn get_volume_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("VolumeMesh", name)
            .map(|vm| vm.transform())
    })
}

// ============================================================================
// Camera and UI Sync Functions
// ============================================================================

/// Syncs CameraSettings from UI to the actual Camera.
pub fn apply_camera_settings(
    camera: &mut polyscope_render::Camera,
    settings: &polyscope_ui::CameraSettings,
) {
    use polyscope_render::{AxisDirection, NavigationStyle, ProjectionMode};

    camera.navigation_style = match settings.navigation_style {
        0 => NavigationStyle::Turntable,
        1 => NavigationStyle::Free,
        2 => NavigationStyle::Planar,
        3 => NavigationStyle::FirstPerson,
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

    camera.front_direction = match settings.front_direction {
        0 => AxisDirection::PosX,
        1 => AxisDirection::NegX,
        2 => AxisDirection::PosY,
        3 => AxisDirection::NegY,
        4 => AxisDirection::PosZ,
        _ => AxisDirection::NegZ,
    };

    camera.set_fov_degrees(settings.fov_degrees);
    camera.set_near(settings.near);
    camera.set_far(settings.far);
    camera.set_move_speed(settings.move_speed);
    camera.set_ortho_scale(settings.ortho_scale);
}

/// Creates CameraSettings from the current Camera state.
pub fn camera_to_settings(camera: &polyscope_render::Camera) -> polyscope_ui::CameraSettings {
    use polyscope_render::{AxisDirection, NavigationStyle, ProjectionMode};

    polyscope_ui::CameraSettings {
        navigation_style: match camera.navigation_style {
            NavigationStyle::Turntable => 0,
            NavigationStyle::Free => 1,
            NavigationStyle::Planar => 2,
            NavigationStyle::FirstPerson => 3,
            NavigationStyle::None => 4,
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
        front_direction: match camera.front_direction {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        },
        fov_degrees: camera.fov_degrees(),
        near: camera.near,
        far: camera.far,
        move_speed: camera.move_speed,
        ortho_scale: camera.ortho_scale,
    }
}

/// Gets scene extents from the global context.
pub fn get_scene_extents() -> polyscope_ui::SceneExtents {
    polyscope_core::state::with_context(|ctx| polyscope_ui::SceneExtents {
        auto_compute: ctx.options.auto_compute_scene_extents,
        length_scale: ctx.length_scale,
        bbox_min: ctx.bounding_box.0.to_array(),
        bbox_max: ctx.bounding_box.1.to_array(),
    })
}

/// Sets auto-compute scene extents option.
pub fn set_auto_compute_extents(auto: bool) {
    polyscope_core::state::with_context_mut(|ctx| {
        ctx.options.auto_compute_scene_extents = auto;
    });
}

// ============================================================================
// Slice Plane UI Sync Functions
// ============================================================================

/// Gets all slice planes as UI settings.
pub fn get_slice_plane_settings() -> Vec<polyscope_ui::SlicePlaneSettings> {
    with_context(|ctx| {
        ctx.slice_planes
            .values()
            .map(|plane| polyscope_ui::SlicePlaneSettings {
                name: plane.name().to_string(),
                enabled: plane.is_enabled(),
                origin: plane.origin().to_array(),
                normal: plane.normal().to_array(),
                draw_plane: plane.draw_plane(),
                draw_widget: plane.draw_widget(),
                color: plane.color().to_array(),
                transparency: plane.transparency(),
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
            current_settings.push(polyscope_ui::SlicePlaneSettings::with_name(&name));
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

    #[test]
    fn test_create_group() {
        setup();
        let name = unique_name("test_group");
        let handle = create_group(&name);
        assert_eq!(handle.name(), name);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_get_group() {
        setup();
        let name = unique_name("get_group");
        create_group(&name);

        let found = get_group(&name);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), name);

        let not_found = get_group("nonexistent_group_xyz");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_group_enable_disable() {
        setup();
        let name = unique_name("enable_group");
        let handle = create_group(&name);

        assert!(handle.is_enabled());
        handle.set_enabled(false);
        assert!(!handle.is_enabled());
        handle.set_enabled(true);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_group_add_structures() {
        setup();
        let group_name = unique_name("struct_group");
        let pc_name = unique_name("pc_in_group");

        // Create point cloud
        register_point_cloud(&pc_name, vec![Vec3::ZERO, Vec3::X]);

        // Create group and add point cloud
        let handle = create_group(&group_name);
        handle.add_point_cloud(&pc_name);

        assert_eq!(handle.num_structures(), 1);
    }

    #[test]
    fn test_group_hierarchy() {
        setup();
        let parent_name = unique_name("parent_group");
        let child_name = unique_name("child_group");

        let parent = create_group(&parent_name);
        let _child = create_group(&child_name);

        parent.add_child_group(&child_name);

        assert_eq!(parent.num_child_groups(), 1);
    }

    #[test]
    fn test_remove_group() {
        setup();
        let name = unique_name("remove_group");
        create_group(&name);

        assert!(get_group(&name).is_some());
        remove_group(&name);
        assert!(get_group(&name).is_none());
    }

    #[test]
    fn test_add_slice_plane() {
        setup();
        let name = unique_name("slice_plane");
        let handle = add_slice_plane(&name);
        assert_eq!(handle.name(), name);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_slice_plane_pose() {
        setup();
        let name = unique_name("slice_pose");
        let handle = add_slice_plane_with_pose(&name, Vec3::new(1.0, 2.0, 3.0), Vec3::X);

        assert_eq!(handle.origin(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(handle.normal(), Vec3::X);
    }

    #[test]
    fn test_slice_plane_setters() {
        setup();
        let name = unique_name("slice_setters");
        let handle = add_slice_plane(&name);

        handle
            .set_origin(Vec3::new(1.0, 0.0, 0.0))
            .set_normal(Vec3::Z)
            .set_color(Vec3::new(1.0, 0.0, 0.0))
            .set_transparency(0.5);

        assert_eq!(handle.origin(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(handle.normal(), Vec3::Z);
        assert_eq!(handle.color(), Vec3::new(1.0, 0.0, 0.0));
        assert!((handle.transparency() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_slice_plane_enable_disable() {
        setup();
        let name = unique_name("slice_enable");
        let handle = add_slice_plane(&name);

        assert!(handle.is_enabled());
        handle.set_enabled(false);
        assert!(!handle.is_enabled());
        handle.set_enabled(true);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_remove_slice_plane() {
        setup();
        let name = unique_name("slice_remove");
        add_slice_plane(&name);

        assert!(get_slice_plane(&name).is_some());
        remove_slice_plane(&name);
        assert!(get_slice_plane(&name).is_none());
    }

    #[test]
    fn test_select_structure() {
        setup();
        let name = unique_name("select_pc");
        register_point_cloud(&name, vec![Vec3::ZERO]);

        assert!(!has_selection());

        select_structure("PointCloud", &name);
        assert!(has_selection());

        let selected = get_selected_structure();
        assert!(selected.is_some());
        let (type_name, struct_name) = selected.unwrap();
        assert_eq!(type_name, "PointCloud");
        assert_eq!(struct_name, name);

        deselect_structure();
        assert!(!has_selection());
    }

    #[test]
    fn test_gizmo_mode() {
        setup();
        set_gizmo_mode(GizmoMode::Translate);
        assert_eq!(get_gizmo_mode(), GizmoMode::Translate);

        set_gizmo_mode(GizmoMode::Rotate);
        assert_eq!(get_gizmo_mode(), GizmoMode::Rotate);

        set_gizmo_mode(GizmoMode::Scale);
        assert_eq!(get_gizmo_mode(), GizmoMode::Scale);
    }

    #[test]
    fn test_gizmo_space() {
        setup();
        set_gizmo_space(GizmoSpace::World);
        assert_eq!(get_gizmo_space(), GizmoSpace::World);

        set_gizmo_space(GizmoSpace::Local);
        assert_eq!(get_gizmo_space(), GizmoSpace::Local);
    }

    #[test]
    fn test_structure_transform() {
        setup();
        let name = unique_name("transform_pc");
        register_point_cloud(&name, vec![Vec3::ZERO, Vec3::X]);

        // Default transform is identity
        let transform = get_point_cloud_transform(&name);
        assert!(transform.is_some());

        // Set a translation transform
        let new_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        set_point_cloud_transform(&name, new_transform);

        let transform = get_point_cloud_transform(&name).unwrap();
        let translation = transform.w_axis.truncate();
        assert!((translation - Vec3::new(1.0, 2.0, 3.0)).length() < 0.001);
    }

    #[test]
    fn test_get_slice_plane_settings() {
        setup();
        let name = unique_name("ui_slice_plane");

        // Add a slice plane
        add_slice_plane_with_pose(&name, Vec3::new(1.0, 2.0, 3.0), Vec3::X);

        // Get settings
        let settings = get_slice_plane_settings();
        let found = settings.iter().find(|s| s.name == name);
        assert!(found.is_some());

        let s = found.unwrap();
        assert_eq!(s.origin, [1.0, 2.0, 3.0]);
        assert_eq!(s.normal, [1.0, 0.0, 0.0]);
        assert!(s.enabled);
    }

    #[test]
    fn test_apply_slice_plane_settings() {
        setup();
        let name = unique_name("apply_slice_plane");

        // Add a slice plane
        add_slice_plane(&name);

        // Create modified settings
        let settings = polyscope_ui::SlicePlaneSettings {
            name: name.clone(),
            enabled: false,
            origin: [5.0, 6.0, 7.0],
            normal: [0.0, 0.0, 1.0],
            draw_plane: false,
            draw_widget: true,
            color: [1.0, 0.0, 0.0],
            transparency: 0.8,
        };

        // Apply settings
        apply_slice_plane_settings(&settings);

        // Verify
        let handle = get_slice_plane(&name).unwrap();
        assert!(!handle.is_enabled());
        assert_eq!(handle.origin(), Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(handle.normal(), Vec3::Z);
        assert!(!handle.draw_plane());
        assert!(handle.draw_widget());
        assert_eq!(handle.color(), Vec3::new(1.0, 0.0, 0.0));
        assert!((handle.transparency() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_handle_slice_plane_action_add() {
        setup();
        let name = unique_name("action_add_plane");
        let mut settings = Vec::new();

        handle_slice_plane_action(
            polyscope_ui::SlicePlanesAction::Add(name.clone()),
            &mut settings,
        );

        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].name, name);
        assert!(get_slice_plane(&name).is_some());
    }

    #[test]
    fn test_handle_slice_plane_action_remove() {
        setup();
        let name = unique_name("action_remove_plane");

        // Add plane
        add_slice_plane(&name);
        let mut settings = vec![polyscope_ui::SlicePlaneSettings::with_name(&name)];

        // Remove via action
        handle_slice_plane_action(
            polyscope_ui::SlicePlanesAction::Remove(0),
            &mut settings,
        );

        assert!(settings.is_empty());
        assert!(get_slice_plane(&name).is_none());
    }
}
