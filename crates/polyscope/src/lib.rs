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

// Type casts in visualization code: Conversions between coordinate types (f32, f64)
// and index types (u32, usize) are intentional. Values are bounded by practical
// limits (screen resolution, mesh sizes).
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
// Documentation lints: Detailed error/panic docs will be added as the API stabilizes.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// Function length: Event handling and application logic are legitimately complex.
#![allow(clippy::too_many_lines)]
// Code organization: Local types in event handlers improve readability.
#![allow(clippy::items_after_statements)]
// Function signatures: Some public API functions need many parameters for flexibility.
#![allow(clippy::too_many_arguments)]
// Method design: Some methods take &self for API consistency or future expansion.
#![allow(clippy::unused_self)]
// Argument design: Some functions take ownership for API consistency.
#![allow(clippy::needless_pass_by_value)]
// Variable naming: Short names (x, y, z) are clear in context.
#![allow(clippy::many_single_char_names)]
// Configuration structs may have many boolean fields.
#![allow(clippy::struct_excessive_bools)]

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
    AppearanceSettings, CameraSettings, GizmoAction, GizmoSettings, GroupSettings, GroupsAction,
    SceneExtents, SelectionInfo, SlicePlaneGizmoAction, SlicePlaneSelectionInfo,
    SlicePlaneSettings, SlicePlanesAction, ViewAction,
};

// Re-export structures
pub use polyscope_structures::{
    CameraExtrinsics, CameraIntrinsics, CameraParameters, CameraView, CurveNetwork, PointCloud,
    SurfaceMesh, VolumeCellType, VolumeGrid, VolumeMesh,
};

use glam::UVec3;
use std::sync::Mutex;

/// Global screenshot request storage.
/// This allows `screenshot()` to be called from user code while `show()` is running.
static SCREENSHOT_REQUEST: Mutex<Option<ScreenshotRequest>> = Mutex::new(None);

/// A pending screenshot request.
#[derive(Debug, Clone)]
pub struct ScreenshotRequest {
    /// Filename to save to. None means auto-generate.
    pub filename: Option<String>,
    /// Screenshot options.
    pub options: ScreenshotOptions,
}

/// Requests a screenshot with an auto-generated filename.
///
/// The screenshot will be saved as `screenshot_NNNN.png` in the current directory,
/// where NNNN is an auto-incrementing number.
///
/// This function can be called while `show()` is running.
/// The screenshot will be captured on the next frame.
///
/// # Example
///
/// ```no_run
/// use polyscope::*;
///
/// init().unwrap();
/// // ... register structures ...
///
/// // Request a screenshot (will be saved when show() runs)
/// screenshot();
///
/// show();
/// ```
pub fn screenshot() {
    screenshot_with_options(ScreenshotOptions::default());
}

/// Requests a screenshot with custom options.
///
/// # Arguments
/// * `options` - Screenshot options (e.g., transparent background)
pub fn screenshot_with_options(options: ScreenshotOptions) {
    if let Ok(mut guard) = SCREENSHOT_REQUEST.lock() {
        *guard = Some(ScreenshotRequest {
            filename: None,
            options,
        });
    }
}

/// Requests a screenshot to be saved to a specific file.
///
/// # Arguments
/// * `filename` - The filename to save to (supports .png and .jpg)
///
/// # Example
///
/// ```no_run
/// use polyscope::*;
///
/// init().unwrap();
/// // ... register structures ...
///
/// screenshot_to_file("my_scene.png");
/// show();
/// ```
pub fn screenshot_to_file(filename: impl Into<String>) {
    screenshot_to_file_with_options(filename, ScreenshotOptions::default());
}

/// Requests a screenshot to be saved to a specific file with custom options.
///
/// # Arguments
/// * `filename` - The filename to save to (supports .png and .jpg)
/// * `options` - Screenshot options
pub fn screenshot_to_file_with_options(filename: impl Into<String>, options: ScreenshotOptions) {
    if let Ok(mut guard) = SCREENSHOT_REQUEST.lock() {
        *guard = Some(ScreenshotRequest {
            filename: Some(filename.into()),
            options,
        });
    }
}

/// Takes and returns a pending screenshot request (for internal use by App).
pub(crate) fn take_screenshot_request() -> Option<ScreenshotRequest> {
    SCREENSHOT_REQUEST
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

/// Initializes polyscope with default settings.
///
/// This must be called before any other polyscope functions.
pub fn init() -> Result<()> {
    polyscope_core::state::init_context()?;
    log::info!("polyscope-rs initialized");
    Ok(())
}

/// Returns whether polyscope has been initialized.
#[must_use]
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
#[must_use]
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
#[must_use]
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a scalar quantity to this point cloud.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
#[must_use]
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    #[must_use]
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_color(color);
        });
        self
    }

    /// Sets the radius.
    #[must_use]
    pub fn set_radius(&self, radius: f32, is_relative: bool) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_radius(radius, is_relative);
        });
        self
    }

    /// Sets the material.
    #[must_use]
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
    #[must_use]
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_color(color);
        });
        self
    }

    /// Sets the widget focal length.
    #[must_use]
    pub fn set_widget_focal_length(&self, length: f32, is_relative: bool) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_widget_focal_length(length, is_relative);
        });
        self
    }

    /// Sets the widget thickness.
    #[must_use]
    pub fn set_widget_thickness(&self, thickness: f32) -> &Self {
        with_camera_view(&self.name, |cv| {
            cv.set_widget_thickness(thickness);
        });
        self
    }

    /// Updates the camera parameters.
    #[must_use]
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
#[must_use]
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the edge color.
    #[must_use]
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    #[must_use]
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_width(width);
        });
        self
    }

    /// Adds a node scalar quantity.
    #[must_use]
    pub fn add_node_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.add_node_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a cell scalar quantity.
    #[must_use]
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
/// Cells are stored as 8-index arrays. For tetrahedra, indices 4-7 should be `u32::MAX`.
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
#[must_use]
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    #[must_use]
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_color(color);
        });
        self
    }

    /// Sets the interior color.
    #[must_use]
    pub fn set_interior_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_interior_color(color);
        });
        self
    }

    /// Sets the edge color.
    #[must_use]
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    #[must_use]
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_width(width);
        });
        self
    }

    /// Adds a vertex scalar quantity.
    pub fn add_vertex_scalar_quantity(&self, name: impl Into<String>, values: Vec<f32>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(&self, name: impl Into<String>, values: Vec<f32>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a vertex color quantity.
    pub fn add_vertex_color_quantity(&self, name: impl Into<String>, colors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_color_quantity(name, colors);
        });
        self
    }

    /// Adds a cell color quantity.
    pub fn add_cell_color_quantity(&self, name: impl Into<String>, colors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_color_quantity(name, colors);
        });
        self
    }

    /// Adds a vertex vector quantity.
    pub fn add_vertex_vector_quantity(&self, name: impl Into<String>, vectors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_vector_quantity(name, vectors);
        });
        self
    }

    /// Adds a cell vector quantity.
    pub fn add_cell_vector_quantity(&self, name: impl Into<String>, vectors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_vector_quantity(name, vectors);
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
    #[must_use]
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
    #[must_use]
    pub fn set_show_child_details(&self, show: bool) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.set_show_child_details(show);
            }
        });
        self
    }

    /// Adds a point cloud to this group.
    #[must_use]
    pub fn add_point_cloud(&self, pc_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("PointCloud", pc_name);
            }
        });
        self
    }

    /// Adds a surface mesh to this group.
    #[must_use]
    pub fn add_surface_mesh(&self, mesh_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("SurfaceMesh", mesh_name);
            }
        });
        self
    }

    /// Adds a curve network to this group.
    #[must_use]
    pub fn add_curve_network(&self, cn_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("CurveNetwork", cn_name);
            }
        });
        self
    }

    /// Adds a volume mesh to this group.
    #[must_use]
    pub fn add_volume_mesh(&self, vm_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("VolumeMesh", vm_name);
            }
        });
        self
    }

    /// Adds a volume grid to this group.
    #[must_use]
    pub fn add_volume_grid(&self, vg_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("VolumeGrid", vg_name);
            }
        });
        self
    }

    /// Adds a camera view to this group.
    #[must_use]
    pub fn add_camera_view(&self, cv_name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.add_structure("CameraView", cv_name);
            }
        });
        self
    }

    /// Removes a structure from this group.
    #[must_use]
    pub fn remove_structure(&self, type_name: &str, name: &str) -> &Self {
        with_context_mut(|ctx| {
            if let Some(group) = ctx.get_group_mut(&self.name) {
                group.remove_structure(type_name, name);
            }
        });
        self
    }

    /// Adds a child group to this group.
    #[must_use]
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
    #[must_use]
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

// ============================================================================
// Slice Planes API
// ============================================================================

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
    #[must_use]
    pub fn set_pose(&self, origin: Vec3, normal: Vec3) -> &Self {
        with_context_mut(|ctx| {
            if let Some(plane) = ctx.get_slice_plane_mut(&self.name) {
                plane.set_pose(origin, normal);
            }
        });
        self
    }

    /// Sets the origin point of the plane.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    pub fn color(&self) -> Vec3 {
        with_context(|ctx| {
            ctx.get_slice_plane(&self.name)
                .map_or(Vec3::new(0.5, 0.5, 0.5), polyscope_core::SlicePlane::color)
        })
    }

    /// Sets the transparency of the plane visualization.
    #[must_use]
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
    #[must_use]
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
#[must_use]
pub fn get_point_cloud_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("PointCloud", name)
            .map(polyscope_core::Structure::transform)
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
#[must_use]
pub fn get_surface_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("SurfaceMesh", name)
            .map(polyscope_core::Structure::transform)
    })
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
#[must_use]
pub fn get_curve_network_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .map(polyscope_core::Structure::transform)
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
#[must_use]
pub fn get_volume_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("VolumeMesh", name)
            .map(polyscope_core::Structure::transform)
    })
}

// ============================================================================
// Camera and UI Sync Functions
// ============================================================================

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
pub fn set_auto_compute_extents(auto: bool) {
    polyscope_core::state::with_context_mut(|ctx| {
        ctx.options.auto_compute_scene_extents = auto;
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
                color: plane.color().to_array(),
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
                        color: plane.color().to_array(),
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
    current_settings: &mut Vec<polyscope_ui::GroupSettings>,
) {
    match action {
        polyscope_ui::GroupsAction::None => {}
        polyscope_ui::GroupsAction::Create(name) => {
            create_group(&name);
            current_settings.push(polyscope_ui::GroupSettings::with_name(&name));
        }
        polyscope_ui::GroupsAction::Remove(idx) => {
            if idx < current_settings.len() {
                let name = &current_settings[idx].name;
                remove_group(name);
                current_settings.remove(idx);
            }
        }
        polyscope_ui::GroupsAction::ToggleEnabled(idx)
        | polyscope_ui::GroupsAction::ToggleDetails(idx) => {
            if idx < current_settings.len() {
                apply_group_settings(&current_settings[idx]);
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
        // Use ok() to handle race conditions in parallel tests
        let _ = init();
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
    fn test_slice_plane_gizmo_selection() {
        setup();
        let name = unique_name("slice_gizmo");
        add_slice_plane(&name);

        // Initially no slice plane selected
        let info = get_slice_plane_selection_info();
        assert!(!info.has_selection);

        // Select slice plane
        select_slice_plane_for_gizmo(&name);
        let info = get_slice_plane_selection_info();
        assert!(info.has_selection);
        assert_eq!(info.name, name);

        // Deselect slice plane
        deselect_slice_plane_gizmo();
        let info = get_slice_plane_selection_info();
        assert!(!info.has_selection);
    }

    #[test]
    fn test_slice_plane_structure_mutual_exclusion() {
        setup();
        let pc_name = unique_name("mutual_pc");
        let plane_name = unique_name("mutual_plane");

        register_point_cloud(&pc_name, vec![Vec3::ZERO]);
        add_slice_plane(&plane_name);

        // Select structure
        select_structure("PointCloud", &pc_name);
        assert!(has_selection());

        // Select slice plane - should deselect structure
        select_slice_plane_for_gizmo(&plane_name);
        assert!(!has_selection()); // Structure should be deselected
        let info = get_slice_plane_selection_info();
        assert!(info.has_selection);

        // Select structure again - should deselect slice plane
        select_structure("PointCloud", &pc_name);
        assert!(has_selection());
        let info = get_slice_plane_selection_info();
        assert!(!info.has_selection); // Slice plane should be deselected
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
            plane_size: 0.2,
            is_selected: false,
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
        handle_slice_plane_action(polyscope_ui::SlicePlanesAction::Remove(0), &mut settings);

        assert!(settings.is_empty());
        assert!(get_slice_plane(&name).is_none());
    }

    #[test]
    fn test_get_group_settings() {
        setup();
        let name = unique_name("ui_group");
        let pc_name = unique_name("pc_in_ui_group");

        // Create group and add a structure
        let handle = create_group(&name);
        register_point_cloud(&pc_name, vec![Vec3::ZERO]);
        handle.add_point_cloud(&pc_name);

        // Get settings
        let settings = get_group_settings();
        let found = settings.iter().find(|s| s.name == name);
        assert!(found.is_some());

        let s = found.unwrap();
        assert!(s.enabled);
        assert!(s.show_child_details);
        assert_eq!(s.child_structures.len(), 1);
        assert_eq!(s.child_structures[0], ("PointCloud".to_string(), pc_name));
    }

    #[test]
    fn test_apply_group_settings() {
        setup();
        let name = unique_name("apply_group");

        // Create group
        create_group(&name);

        // Create modified settings
        let settings = polyscope_ui::GroupSettings {
            name: name.clone(),
            enabled: false,
            show_child_details: false,
            parent_group: None,
            child_structures: Vec::new(),
            child_groups: Vec::new(),
        };

        // Apply settings
        apply_group_settings(&settings);

        // Verify
        let handle = get_group(&name).unwrap();
        assert!(!handle.is_enabled());
    }

    #[test]
    fn test_handle_group_action_create() {
        setup();
        let name = unique_name("action_create_group");
        let mut settings = Vec::new();

        handle_group_action(
            polyscope_ui::GroupsAction::Create(name.clone()),
            &mut settings,
        );

        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].name, name);
        assert!(get_group(&name).is_some());
    }

    #[test]
    fn test_handle_group_action_remove() {
        setup();
        let name = unique_name("action_remove_group");

        // Create group
        create_group(&name);
        let mut settings = vec![polyscope_ui::GroupSettings::with_name(&name)];

        // Remove via action
        handle_group_action(polyscope_ui::GroupsAction::Remove(0), &mut settings);

        assert!(settings.is_empty());
        assert!(get_group(&name).is_none());
    }

    #[test]
    fn test_get_gizmo_settings() {
        setup();

        // Set known values
        set_gizmo_space(GizmoSpace::Local);
        set_gizmo_visible(false);
        set_gizmo_snap_translate(0.5);
        set_gizmo_snap_rotate(15.0);
        set_gizmo_snap_scale(0.1);

        let settings = get_gizmo_settings();
        assert!(settings.local_space); // Local
        assert!(!settings.visible);
        assert!((settings.snap_translate - 0.5).abs() < 0.001);
        assert!((settings.snap_rotate - 15.0).abs() < 0.001);
        assert!((settings.snap_scale - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_apply_gizmo_settings() {
        setup();

        let settings = polyscope_ui::GizmoSettings {
            local_space: false, // World
            visible: true,
            snap_translate: 1.0,
            snap_rotate: 45.0,
            snap_scale: 0.25,
        };

        apply_gizmo_settings(&settings);

        assert_eq!(get_gizmo_space(), GizmoSpace::World);
        assert!(is_gizmo_visible());
    }

    #[test]
    fn test_get_selection_info_no_selection() {
        setup();
        deselect_structure();

        let info = get_selection_info();
        assert!(!info.has_selection);
    }

    #[test]
    fn test_get_selection_info_with_selection() {
        setup();
        let name = unique_name("gizmo_select_pc");

        register_point_cloud(&name, vec![Vec3::ZERO]);
        select_structure("PointCloud", &name);

        let info = get_selection_info();
        assert!(info.has_selection);
        assert_eq!(info.type_name, "PointCloud");
        assert_eq!(info.name, name);

        deselect_structure();
    }

    #[test]
    fn test_apply_selection_transform() {
        setup();
        let name = unique_name("gizmo_transform_pc");

        register_point_cloud(&name, vec![Vec3::ZERO]);
        select_structure("PointCloud", &name);

        let selection = polyscope_ui::SelectionInfo {
            has_selection: true,
            type_name: "PointCloud".to_string(),
            name: name.clone(),
            translation: [1.0, 2.0, 3.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            centroid: [1.0, 2.0, 3.0],
        };

        apply_selection_transform(&selection);

        let transform = get_point_cloud_transform(&name).unwrap();
        let translation = transform.w_axis.truncate();
        assert!((translation - Vec3::new(1.0, 2.0, 3.0)).length() < 0.001);

        deselect_structure();
    }
}
