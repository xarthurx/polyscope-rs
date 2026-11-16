//! polyscope-rs: A Rust-native 3D visualization library for geometric data.
//!
//! Polyscope is a viewer and user interface for 3D data such as meshes and point clouds.
//! It allows you to register your data and quickly generate informative visualizations.
//!
//! # Quick Start
//!
//! ```no_run
//! use polyscope_rs::*;
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
// #[must_use] design: Setter methods intentionally omit #[must_use] since
// the mutation happens in-place; the &Self return is just for chaining convenience.
#![allow(clippy::must_use_candidate)]
// Documentation formatting: Backtick linting is too strict for doc comments.
#![allow(clippy::doc_markdown)]

/// Generates `get_<prefix>`, `with_<prefix>`, and `with_<prefix>_ref` accessor functions
/// for a structure type registered in the global context.
macro_rules! impl_structure_accessors {
    (
        get_fn = $get_fn:ident,
        with_fn = $with_fn:ident,
        with_ref_fn = $with_ref_fn:ident,
        handle = $handle:ident,
        type_name = $type_name:expr,
        rust_type = $rust_type:ty,
        doc_name = $doc_name:expr
    ) => {
        #[doc = concat!("Gets a registered ", $doc_name, " by name.")]
        #[must_use]
        pub fn $get_fn(name: &str) -> Option<$handle> {
            crate::with_context(|ctx| {
                if ctx.registry.contains($type_name, name) {
                    Some($handle {
                        name: name.to_string(),
                    })
                } else {
                    None
                }
            })
        }

        #[doc = concat!("Executes a closure with mutable access to a registered ", $doc_name, ".\n\nReturns `None` if the ", $doc_name, " does not exist.")]
        pub fn $with_fn<F, R>(name: &str, f: F) -> Option<R>
        where
            F: FnOnce(&mut $rust_type) -> R,
        {
            crate::with_context_mut(|ctx| {
                ctx.registry
                    .get_mut($type_name, name)
                    .and_then(|s| s.as_any_mut().downcast_mut::<$rust_type>())
                    .map(f)
            })
        }

        #[doc = concat!("Executes a closure with immutable access to a registered ", $doc_name, ".\n\nReturns `None` if the ", $doc_name, " does not exist.")]
        pub fn $with_ref_fn<F, R>(name: &str, f: F) -> Option<R>
        where
            F: FnOnce(&$rust_type) -> R,
        {
            crate::with_context(|ctx| {
                ctx.registry
                    .get($type_name, name)
                    .and_then(|s| s.as_any().downcast_ref::<$rust_type>())
                    .map(f)
            })
        }
    };
}

mod app;
mod camera_view;
mod curve_network;
mod floating;
mod gizmo;
mod groups;
mod headless;
mod init;
mod point_cloud;
mod screenshot;
mod slice_plane;
mod surface_mesh;
mod transform;
mod ui_sync;
mod volume_grid;
mod volume_mesh;

// Re-export core types
pub use polyscope_core::{
    Mat4, Vec2, Vec3, Vec4,
    error::{PolyscopeError, Result},
    gizmo::{GizmoAxis, GizmoConfig, GizmoMode, GizmoSpace, Transform},
    ground_plane::{GroundPlaneConfig, GroundPlaneMode},
    group::Group,
    options::{Options, TransparencyMode},
    pick::{PickResult, Pickable},
    quantity::{ParamCoordsType, ParamVizStyle, Quantity, QuantityKind},
    registry::Registry,
    slice_plane::{MAX_SLICE_PLANES, SlicePlane, SlicePlaneUniforms},
    ssao::SsaoConfig,
    state::{Context, with_context, with_context_mut},
    structure::{HasQuantities, Structure},
    view_state::{
        CameraStateOwned, GroundPlaneState, RenderState, TransparencyState, ViewState,
        ViewTransition, apply_view_state, current_view_state, load_view_from_file,
        load_view_from_json, save_view_to_file, save_view_to_json,
    },
};

// Re-export render types
pub use polyscope_render::{
    AxisDirection, BUILTIN_COLOR_MAP_NAMES, Camera, ColorMap, ColorMapRegistry, Material,
    MaterialRegistry, NavigationStyle, PickElementType, ProjectionMode, RenderContext,
    RenderEngine, ScreenshotError, ScreenshotOptions,
};

// Re-export UI types
pub use polyscope_ui::{
    AppearanceSettings, CameraSettings, GizmoAction, GizmoSettings, GroupSettings, GroupsAction,
    SceneExtents, SelectionInfo, SlicePlaneGizmoAction, SlicePlaneSelectionInfo,
    SlicePlaneSettings, SlicePlanesAction, ViewAction,
};

// Re-export structures
pub use polyscope_structures::volume_grid::VolumeGridVizMode;
pub use polyscope_structures::{
    CameraExtrinsics, CameraIntrinsics, CameraParameters, CameraView, CurveNetwork, PointCloud,
    SurfaceMesh, VolumeCellType, VolumeGrid, VolumeMesh,
};

// Re-export module APIs
pub use camera_view::*;
pub use curve_network::*;
pub use floating::*;
pub use gizmo::*;
pub use groups::*;
pub use headless::*;
pub use init::*;
pub use point_cloud::*;
pub use screenshot::*;
pub use slice_plane::*;
pub use surface_mesh::*;
pub use transform::*;
pub use ui_sync::*;
pub use volume_grid::*;
pub use volume_mesh::*;

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
        // A removed structure does not stay selected.
        let selection_is_gone = ctx
            .selected_structure()
            .is_some_and(|(type_name, selected)| !ctx.registry.contains(type_name, selected));
        if selection_is_gone {
            ctx.deselect_structure();
        }
        ctx.update_extents();
    });
}

/// Removes all structures.
pub fn remove_all_structures() {
    with_context_mut(|ctx| {
        ctx.registry.clear();
        ctx.deselect_structure();
        ctx.update_extents();
    });
}

/// Removes all structures, groups, slice planes, floating quantities,
/// and clears callbacks.
///
/// Unlike shutting down, this preserves render engine state and options.
/// Useful for resetting the scene while keeping the app running.
/// Matches C++ Polyscope's `removeEverything()` (commit f34f403).
pub fn remove_everything() {
    remove_all_structures();
    remove_all_groups();
    remove_all_slice_planes();
    remove_all_floating_quantities();
    clear_file_drop_callback();
}

/// Sets a callback that is invoked when files are dropped onto the polyscope window.
///
/// The callback receives a slice of file paths that were dropped.
///
/// # Example
/// ```no_run
/// polyscope_rs::set_file_drop_callback(|paths| {
///     for path in paths {
///         println!("Dropped: {}", path.display());
///     }
/// });
/// ```
pub fn set_file_drop_callback(callback: impl FnMut(&[std::path::PathBuf]) + Send + Sync + 'static) {
    with_context_mut(|ctx| {
        ctx.file_drop_callback = Some(Box::new(callback));
    });
}

/// Clears the file drop callback.
pub fn clear_file_drop_callback() {
    with_context_mut(|ctx| {
        ctx.file_drop_callback = None;
    });
}

/// Loads a blendable (4-channel, RGB-tintable) matcap material from disk.
///
/// Takes a name and 4 image file paths for R, G, B, K matcap channels.
/// The material becomes available in the UI material selector on the next frame.
///
/// Supports HDR, JPEG, PNG, EXR, and other image formats.
///
/// # Example
/// ```no_run
/// polyscope_rs::load_blendable_material("metal", [
///     "assets/metal_r.hdr",
///     "assets/metal_g.hdr",
///     "assets/metal_b.hdr",
///     "assets/metal_k.hdr",
/// ]);
/// ```
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) {
    with_context_mut(|ctx| {
        ctx.material_load_queue
            .push(polyscope_core::state::MaterialLoadRequest::Blendable {
                name: name.to_string(),
                filenames: [
                    filenames[0].to_string(),
                    filenames[1].to_string(),
                    filenames[2].to_string(),
                    filenames[3].to_string(),
                ],
            });
    });
}

/// Loads a blendable material using a base path and extension.
///
/// Automatically expands to 4 filenames by appending `_r`, `_g`, `_b`, `_k`
/// before the extension. For example:
/// `load_blendable_material_ext("metal", "assets/metal", ".hdr")`
/// loads `assets/metal_r.hdr`, `assets/metal_g.hdr`, `assets/metal_b.hdr`, `assets/metal_k.hdr`.
pub fn load_blendable_material_ext(name: &str, base: &str, ext: &str) {
    load_blendable_material(
        name,
        [
            &format!("{base}_r{ext}"),
            &format!("{base}_g{ext}"),
            &format!("{base}_b{ext}"),
            &format!("{base}_k{ext}"),
        ],
    );
}

/// Loads a static (single-texture, non-RGB-tintable) matcap material from disk.
///
/// The same texture is used for all 4 matcap channels. Static materials
/// cannot be tinted with per-surface RGB colors.
///
/// # Example
/// ```no_run
/// polyscope_rs::load_static_material("stone", "assets/stone.jpg");
/// ```
pub fn load_static_material(name: &str, filename: &str) {
    with_context_mut(|ctx| {
        ctx.material_load_queue
            .push(polyscope_core::state::MaterialLoadRequest::Static {
                name: name.to_string(),
                path: filename.to_string(),
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    // Counter for unique test names to avoid race conditions
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    // All tests share one global context. Some tests clear it
    // (`remove_everything`, `remove_all_structures`, `remove_all_groups`).
    // This lock makes the tests run one at a time.
    static CONTEXT_LOCK: Mutex<()> = Mutex::new(());

    fn unique_name(prefix: &str) -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{prefix}_{n}")
    }

    /// Initializes the context and returns the guard that serializes the tests.
    ///
    /// Keep the guard alive for the full test (`let _guard = setup();`).
    fn setup() -> MutexGuard<'static, ()> {
        // A failed test poisons the lock. The other tests must still run.
        let guard = CONTEXT_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        // `init` fails after the first call. That is expected here.
        let _ = init();
        // Each test starts from a clean scene. An earlier test can leave
        // structures, slice planes, a selection or changed options.
        remove_everything();
        with_context_mut(|ctx| {
            ctx.options = Options::default();
            ctx.gizmo_config = GizmoConfig::default();
            ctx.take_pending_view_apply();
        });
        guard
    }

    #[test]
    fn test_removed_structure_does_not_stay_selected() {
        let _guard = setup();
        let kept = unique_name("selected_kept");
        let removed = unique_name("selected_removed");
        register_point_cloud(&kept, vec![Vec3::ZERO]);
        register_point_cloud(&removed, vec![Vec3::X]);

        // Removal of another structure keeps the selection.
        select_structure("PointCloud", &kept);
        remove_structure(&removed);
        assert_eq!(
            get_selected_structure(),
            Some(("PointCloud".to_string(), kept.clone()))
        );

        // Removal of the selected structure clears the selection.
        remove_structure(&kept);
        assert!(!has_selection());

        // Removal of all structures clears the selection.
        register_point_cloud(&kept, vec![Vec3::ZERO]);
        select_structure("PointCloud", &kept);
        remove_all_structures();
        assert!(!has_selection());
    }

    #[test]
    fn test_removed_slice_plane_does_not_stay_selected() {
        let _guard = setup();
        let name = unique_name("selected_plane");

        add_slice_plane(&name);
        with_context_mut(|ctx| ctx.select_slice_plane(&name));
        remove_slice_plane(&name);
        assert!(with_context(|ctx| ctx.selected_slice_plane().is_none()));

        add_slice_plane(&name);
        with_context_mut(|ctx| ctx.select_slice_plane(&name));
        remove_all_slice_planes();
        assert!(with_context(|ctx| ctx.selected_slice_plane().is_none()));
    }

    #[test]
    fn test_register_curve_network() {
        let _guard = setup();
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
        let _guard = setup();
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
    fn test_volume_structure_set_material() {
        let _guard = setup();

        let vm_name = unique_name("vm_material");
        let vm = register_tet_mesh(
            &vm_name,
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
            vec![[0, 1, 2, 3]],
        );
        let vm_material = || with_volume_mesh_ref(&vm_name, |m| m.material().to_string());
        assert_eq!(vm_material().as_deref(), Some("clay"));
        vm.set_material("wax");
        assert_eq!(vm_material().as_deref(), Some("wax"));

        let vg_name = unique_name("vg_material");
        let vg = register_volume_grid(&vg_name, glam::UVec3::splat(3), Vec3::ZERO, Vec3::ONE);
        let vg_material = || with_volume_grid_ref(&vg_name, |g| g.material().to_string());
        assert_eq!(vg_material().as_deref(), Some("clay"));
        vg.set_material("candy");
        assert_eq!(vg_material().as_deref(), Some("candy"));
    }

    #[test]
    fn test_point_cloud_handle_adds_quantities() {
        let _guard = setup();
        let name = unique_name("pc_quantities");
        register_point_cloud(&name, vec![Vec3::ZERO, Vec3::X])
            .add_scalar_quantity("scalar", vec![0.0, 1.0])
            .add_vector_quantity("vector", vec![Vec3::X, Vec3::Y])
            .add_color_quantity("color", vec![Vec3::ONE, Vec3::ZERO]);
        let num_quantities = with_point_cloud_ref(&name, |pc| pc.num_quantities());
        assert_eq!(num_quantities, Some(3));
    }

    /// Renders frames with one long-lived headless engine.
    ///
    /// `render_to_image` clears all GPU data on every call, so it cannot show
    /// stale GPU data. This renderer keeps the GPU data between frames.
    struct LiveRenderer {
        app: app::App,
    }

    impl LiveRenderer {
        fn new() -> Option<Self> {
            use pollster::FutureExt;
            let engine = RenderEngine::new_headless(160, 120).block_on().ok()?;
            let mut app = app::App::new();
            app.engine = Some(engine);
            Some(Self { app })
        }

        fn frame(&mut self) -> Vec<u8> {
            self.app.render_frame_headless(false);
            self.app.capture_to_buffer().expect("capture failed")
        }
    }

    /// Checks that `update` shows in the next frame of a long-lived engine.
    ///
    /// The reference image comes from the same engine and camera after all
    /// GPU data is cleared and built again from the updated structures.
    fn assert_update_reaches_gpu(label: &str, update: impl FnOnce()) {
        let Some(mut renderer) = LiveRenderer::new() else {
            eprintln!("Skipping {label}: no GPU adapter available");
            return;
        };
        let before = renderer.frame();
        update();
        let updated = renderer.frame();
        with_context_mut(|ctx| {
            for structure in ctx.registry.iter_mut() {
                structure.clear_gpu_resources();
            }
        });
        let rebuilt = renderer.frame();
        assert!(
            before != rebuilt,
            "{label}: the update must change the image"
        );
        let stale_pixels = updated
            .chunks(4)
            .zip(rebuilt.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            stale_pixels, 0,
            "{label}: the next frame shows old GPU data"
        );
    }

    /// Enables one quantity of a structure through the `HasQuantities` trait.
    fn enable_quantity(structure: &mut dyn HasQuantities, quantity: &str) {
        structure
            .get_quantity_mut(quantity)
            .expect("quantity not found")
            .set_enabled(true);
    }

    #[test]
    fn test_position_updates_reach_the_gpu() {
        let _guard = setup();
        let moved = |points: &[Vec3]| -> Vec<Vec3> {
            points
                .iter()
                .map(|p| *p * 0.6 + Vec3::new(0.3, 0.2, 0.0))
                .collect()
        };
        let square = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        remove_all_structures();
        register_point_cloud("live_pc", square.clone());
        with_point_cloud("live_pc", |pc| {
            pc.add_vector_quantity("vecs", vec![Vec3::new(0.2, 0.1, 0.0); 4]);
            enable_quantity(pc, "vecs");
        });
        assert_update_reaches_gpu("point cloud with vectors", || {
            with_point_cloud("live_pc", |pc| pc.update_points(moved(&square)));
        });

        remove_all_structures();
        register_point_cloud("live_pc_count", square.clone());
        assert_update_reaches_gpu("point cloud with a new point count", || {
            let mut points = moved(&square);
            points.push(Vec3::new(0.5, 0.5, 0.0));
            with_point_cloud("live_pc_count", |pc| pc.update_points(points));
        });

        remove_all_structures();
        register_surface_mesh("live_mesh", square.clone(), vec![vec![0u32, 1, 2, 3]]);
        with_surface_mesh("live_mesh", |mesh| {
            mesh.add_vertex_vector_quantity("vertex_vecs", vec![Vec3::new(0.2, 0.1, 0.3); 4]);
            mesh.add_face_vector_quantity("face_vecs", vec![Vec3::new(-0.2, 0.1, 0.3)]);
            enable_quantity(mesh, "vertex_vecs");
            enable_quantity(mesh, "face_vecs");
        });
        assert_update_reaches_gpu("surface mesh with vectors", || {
            with_surface_mesh("live_mesh", |mesh| mesh.update_vertices(moved(&square)));
        });

        remove_all_structures();
        register_curve_network_line("live_cn", square.clone());
        with_curve_network("live_cn", |cn| {
            cn.add_node_vector_quantity("node_vecs", vec![Vec3::new(0.2, 0.1, 0.0); 4]);
            cn.add_edge_vector_quantity("edge_vecs", vec![Vec3::new(-0.2, 0.1, 0.0); 3]);
            enable_quantity(cn, "node_vecs");
            enable_quantity(cn, "edge_vecs");
        });
        assert_update_reaches_gpu("curve network with vectors", || {
            with_curve_network("live_cn", |cn| cn.update_node_positions(moved(&square)));
        });

        remove_all_structures();
        register_curve_network_line("live_cn_count", square.clone());
        assert_update_reaches_gpu("curve network with a new node count", || {
            let mut nodes = moved(&square);
            nodes.push(Vec3::new(0.5, 0.5, 0.0));
            with_curve_network("live_cn_count", |cn| cn.update_node_positions(nodes));
        });

        remove_all_structures();
        register_point_cloud("live_frame", vec![Vec3::splat(-2.0), Vec3::splat(2.0)]);
        let cv = register_camera_view_look_at("live_cv", Vec3::ZERO, Vec3::Z, Vec3::Y, 45.0, 1.5);
        assert_update_reaches_gpu("camera view", || {
            cv.set_params(CameraParameters::look_at(
                Vec3::new(0.5, 0.5, 0.0),
                Vec3::X,
                Vec3::Y,
                60.0,
                1.5,
            ));
        });

        remove_all_structures();
    }

    #[test]
    fn test_register_curve_network_loop() {
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
            assert_eq!(cn.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
            assert_eq!(cn.radius(), 0.1);
            assert!(!cn.radius_is_relative());
            assert_eq!(cn.material(), "clay");
        });
    }

    #[test]
    fn test_with_curve_network() {
        let _guard = setup();
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
        assert_eq!(color, Some(Vec4::new(0.5, 0.5, 0.5, 1.0)));
    }

    #[test]
    fn test_create_group() {
        let _guard = setup();
        let name = unique_name("test_group");
        let handle = create_group(&name);
        assert_eq!(handle.name(), name);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_get_group() {
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
        let parent_name = unique_name("parent_group");
        let child_name = unique_name("child_group");

        let parent = create_group(&parent_name);
        let _child = create_group(&child_name);

        parent.add_child_group(&child_name);

        assert_eq!(parent.num_child_groups(), 1);
    }

    #[test]
    fn test_remove_group() {
        let _guard = setup();
        let name = unique_name("remove_group");
        create_group(&name);

        assert!(get_group(&name).is_some());
        remove_group(&name);
        assert!(get_group(&name).is_none());
    }

    #[test]
    fn test_add_slice_plane() {
        let _guard = setup();
        let name = unique_name("slice_plane");
        let handle = add_slice_plane(&name);
        assert_eq!(handle.name(), name);
        assert!(handle.is_enabled());
    }

    #[test]
    fn test_slice_plane_pose() {
        let _guard = setup();
        let name = unique_name("slice_pose");
        let handle = add_slice_plane_with_pose(&name, Vec3::new(1.0, 2.0, 3.0), Vec3::X);

        assert_eq!(handle.origin(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(handle.normal(), Vec3::X);
    }

    #[test]
    fn test_slice_plane_setters() {
        let _guard = setup();
        let name = unique_name("slice_setters");
        let handle = add_slice_plane(&name);

        handle
            .set_origin(Vec3::new(1.0, 0.0, 0.0))
            .set_normal(Vec3::Z)
            .set_color(Vec3::new(1.0, 0.0, 0.0))
            .set_transparency(0.5);

        assert_eq!(handle.origin(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(handle.normal(), Vec3::Z);
        assert_eq!(handle.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert!((handle.transparency() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_slice_plane_enable_disable() {
        let _guard = setup();
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
        let _guard = setup();
        let name = unique_name("slice_remove");
        add_slice_plane(&name);

        assert!(get_slice_plane(&name).is_some());
        remove_slice_plane(&name);
        assert!(get_slice_plane(&name).is_none());
    }

    #[test]
    fn test_select_structure() {
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
    fn test_structure_transform() {
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
        assert_eq!(handle.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert!((handle.transparency() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_handle_slice_plane_action_add() {
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
        let _guard = setup();
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
    fn test_get_gizmo_settings() {
        let _guard = setup();

        // Set known values
        set_gizmo_mode(GizmoMode::Rotate);
        set_gizmo_space(GizmoSpace::Local);
        set_gizmo_visible(false);
        set_gizmo_snap_translate(0.5);
        set_gizmo_snap_rotate(15.0);
        set_gizmo_snap_scale(0.1);

        let settings = get_gizmo_settings();
        assert_eq!(settings.mode, GizmoMode::Rotate);
        assert!(settings.local_space); // Local
        assert!(!settings.visible);
        assert!((settings.snap_translate - 0.5).abs() < 0.001);
        assert!((settings.snap_rotate - 15.0).abs() < 0.001);
        assert!((settings.snap_scale - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_apply_gizmo_settings() {
        let _guard = setup();

        let settings = polyscope_ui::GizmoSettings {
            mode: GizmoMode::Scale,
            local_space: false, // World
            visible: true,
            snap_translate: 1.0,
            snap_rotate: 45.0,
            snap_scale: 0.25,
        };

        apply_gizmo_settings(&settings);

        assert_eq!(get_gizmo_mode(), GizmoMode::Scale);
        assert_eq!(get_gizmo_space(), GizmoSpace::World);
        assert!(is_gizmo_visible());
        let applied = get_gizmo_settings();
        assert!((applied.snap_translate - 1.0).abs() < 0.001);
        assert!((applied.snap_rotate - 45.0).abs() < 0.001);
        assert!((applied.snap_scale - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_get_selection_info_with_selection() {
        let _guard = setup();
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
        let _guard = setup();
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

    #[test]
    fn test_remove_all_groups() {
        let _guard = setup();
        let g1 = unique_name("rag_group1");
        let g2 = unique_name("rag_group2");
        create_group(&g1);
        create_group(&g2);

        assert!(get_group(&g1).is_some());
        assert!(get_group(&g2).is_some());

        remove_all_groups();

        assert!(get_group(&g1).is_none());
        assert!(get_group(&g2).is_none());
    }

    #[test]
    fn test_remove_everything() {
        let _guard = setup();
        let pc_name = unique_name("re_pc");
        let group_name = unique_name("re_group");
        let sp_name = unique_name("re_slice");

        register_point_cloud(&pc_name, vec![Vec3::ZERO]);
        create_group(&group_name);
        add_slice_plane(&sp_name);

        remove_everything();

        assert!(get_point_cloud(&pc_name).is_none());
        assert!(get_group(&group_name).is_none());
        assert!(get_all_slice_planes().is_empty());
    }

    #[test]
    fn test_degenerate_bounding_box() {
        let _guard = setup();
        // Clear all structures so only our degenerate point cloud contributes
        remove_all_structures();
        let name = unique_name("degen_bbox");
        // Register a point cloud where all points are at the same location
        register_point_cloud(&name, vec![Vec3::ONE, Vec3::ONE, Vec3::ONE]);

        let (bb_min, bb_max) = with_context(|ctx| ctx.bounding_box);
        // Bounding box should be perturbed so min != max
        assert!(
            bb_max.x > bb_min.x,
            "degenerate bbox not perturbed: min={bb_min}, max={bb_max}"
        );
        assert!(bb_max.y > bb_min.y);
        assert!(bb_max.z > bb_min.z);
    }

    /// A single click changes the selection only after the double-click time,
    /// and a double-click cancels it (C++ commit 022deea).
    #[test]
    fn test_click_pick_waits_for_double_click() {
        use crate::app::{App, ClickHit};
        use std::time::{Duration, Instant};

        let _guard = setup();
        deselect_structure();
        let name = unique_name("click_pick_pc");
        register_point_cloud(&name, vec![Vec3::ZERO, Vec3::X]);
        let hit = || {
            Some(ClickHit::Structure {
                type_name: "PointCloud".to_string(),
                name: name.clone(),
                element_index: 1,
            })
        };
        let screen = glam::Vec2::new(10.0, 20.0);
        let later = || Instant::now() + Duration::from_secs(1);

        let mut app = App::new();

        // The first click does not select at once.
        app.queue_click_pick(hit(), screen);
        app.update_pending_pick(Instant::now());
        assert!(app.selection.is_none());
        assert!(with_context(|ctx| ctx.selected_structure().is_none()));

        // The second click of a double-click cancels the pick.
        app.cancel_pending_pick();
        app.update_pending_pick(later());
        assert!(app.selection.is_none());

        // A single click selects after the delay.
        app.queue_click_pick(hit(), screen);
        app.update_pending_pick(later());
        let selection = app.selection.as_ref().expect("selection after the delay");
        assert_eq!(selection.structure_name, name);
        assert_eq!(selection.element_index, 1);
        assert_eq!(
            with_context(|ctx| ctx
                .selected_structure()
                .map(|(t, n)| (t.to_owned(), n.to_owned()))),
            Some(("PointCloud".to_owned(), name.clone()))
        );

        // With a selection, a click applies at once, as in C++.
        app.queue_click_pick(None, screen);
        app.update_pending_pick(Instant::now());
        assert!(app.selection.is_none());

        // A structure removed during the delay is not selected.
        app.queue_click_pick(hit(), screen);
        remove_structure(&name);
        app.update_pending_pick(later());
        assert!(app.selection.is_none());
        assert!(with_context(|ctx| ctx.selected_structure().is_none()));
    }

    /// The pick waits for the Dear `ImGui` double-click time (0.30 s) plus a
    /// 50 ms margin, as C++ Polyscope does.
    #[test]
    fn test_click_pick_delay_matches_imgui_double_click_time() {
        use crate::app::{App, ClickHit};
        use std::time::{Duration, Instant};

        let _guard = setup();
        deselect_structure();
        let name = unique_name("click_delay_pc");
        register_point_cloud(&name, vec![Vec3::ZERO]);
        let mut app = App::new();

        let before = Instant::now();
        app.queue_click_pick(
            Some(ClickHit::Structure {
                type_name: "PointCloud".to_string(),
                name: name.clone(),
                element_index: 0,
            }),
            glam::Vec2::ZERO,
        );
        app.update_pending_pick(before + Duration::from_millis(340));
        assert!(app.selection.is_none(), "no selection before 0.35 s");
        app.update_pending_pick(Instant::now() + Duration::from_millis(360));
        assert!(app.selection.is_some(), "selection after 0.35 s");
        remove_structure(&name);
    }

    /// Mouse releases act as in C++ `processInputEvents` (polyscope.cpp 540-590).
    #[test]
    fn test_release_actions_follow_upstream_click_logic() {
        use crate::app::{ReleaseAction, release_action};
        use winit::event::MouseButton::{Left, Right};
        use winit::keyboard::ModifiersState as Mods;

        let _guard = setup();
        let ctrl = if cfg!(target_os = "macos") {
            Mods::SUPER
        } else {
            Mods::CONTROL
        };
        let none = Mods::empty();

        // Left button: a plain click picks, the end of a drag does nothing.
        assert_eq!(
            release_action(Left, true, false, none),
            ReleaseAction::QueuePick
        );
        assert_eq!(
            release_action(Left, false, false, none),
            ReleaseAction::Nothing
        );
        // A held modifier key suppresses the pick.
        for mods in [Mods::SHIFT, ctrl, Mods::ALT] {
            assert_eq!(
                release_action(Left, true, false, mods),
                ReleaseAction::Nothing
            );
        }
        // A double-click or a Ctrl+Shift click sets the view center.
        let set_center = ReleaseAction::SetViewCenter;
        assert_eq!(release_action(Left, true, true, none), set_center);
        assert_eq!(release_action(Left, true, true, Mods::ALT), set_center);
        assert_eq!(
            release_action(Left, true, false, Mods::SHIFT | ctrl),
            set_center
        );
        assert_eq!(
            release_action(Left, false, true, none),
            ReleaseAction::Nothing
        );

        // Right button: a click clears the selection, a drag cancels a queued pick.
        assert_eq!(
            release_action(Right, true, false, none),
            ReleaseAction::ClearSelection
        );
        assert_eq!(
            release_action(Right, false, false, none),
            ReleaseAction::CancelPick
        );
        assert_eq!(
            release_action(Right, true, false, Mods::SHIFT),
            ReleaseAction::Nothing
        );
    }

    /// The drag threshold follows C++ Polyscope: a drag starts 6 pixels from the
    /// press position, and a release is a click while the drag adds less than
    /// 1% of the window size.
    #[test]
    fn test_drag_threshold_follows_upstream() {
        use crate::app::App;

        let _guard = setup();
        let mut app = App::new();
        let size = (1000, 500);
        let move_by = |app: &mut App, dx: f64| {
            app.mouse_pos.0 += dx;
            app.track_mouse_drag((dx, 0.0), size);
        };

        // Jitter below 6 pixels from the press position is a click.
        app.mouse_pos = (100.0, 100.0);
        app.begin_mouse_press();
        for dx in [2.0, 2.0, -3.0, 4.0, -5.0] {
            move_by(&mut app, dx);
        }
        assert!(app.release_is_click());

        // A short drag past 6 pixels is still a click (7 / 1000 < 0.01).
        app.begin_mouse_press();
        move_by(&mut app, 7.0);
        assert!(app.mouse_dragging);
        assert!(app.release_is_click());

        // A long drag is not a click, also when the cursor comes back.
        move_by(&mut app, 20.0);
        move_by(&mut app, -27.0);
        assert!(!app.release_is_click());

        // The next press starts a new click.
        app.begin_mouse_press();
        assert!(app.release_is_click());

        // A right click clears the selection and a queued pick.
        app.queue_click_pick(None, glam::Vec2::ZERO);
        app.clear_selection();
        assert!(app.pending_pick.is_none());
        assert!(app.selection.is_none());
    }

    #[test]
    fn test_single_point_scene_has_nonzero_length_scale() {
        let _guard = setup();
        // Clear all structures so only the one-point cloud contributes.
        remove_all_structures();
        let name = unique_name("single_point_scale");
        register_point_cloud(&name, vec![Vec3::new(1.0, 2.0, 3.0)]);

        let (length_scale, (bb_min, bb_max)) =
            with_context(|ctx| (ctx.length_scale, ctx.bounding_box));
        assert!(
            length_scale > 0.0,
            "one-point scene must have a positive length scale, got {length_scale}"
        );
        // C++ uses the diagonal of the perturbed box (1e-3 per axis).
        let expected = (bb_max - bb_min).length();
        assert!((length_scale - expected).abs() < 1e-9);
        assert!((length_scale - 3.0_f32.sqrt() * 1e-3).abs() < 1e-6);
    }

    /// Registers structures that hold GPU resources in structures and quantities.
    fn register_gpu_scene() {
        let square = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        register_point_cloud("gpu_pc", square.clone());
        with_point_cloud("gpu_pc", |pc| {
            pc.add_vector_quantity("vecs", vec![Vec3::new(0.2, 0.1, 0.0); 4]);
            enable_quantity(pc, "vecs");
        });
        register_surface_mesh("gpu_mesh", square.clone(), vec![vec![0u32, 1, 2, 3]]);
        register_curve_network_loop("gpu_cn", square);
        with_curve_network("gpu_cn", |cn| {
            cn.set_render_mode(1);
            cn.set_radius(0.05, false);
        });
        let grid = register_volume_grid(
            "gpu_grid",
            glam::UVec3::splat(3),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 1.0, 0.0),
        );
        grid.add_node_scalar_quantity("s", (0..27u8).map(|i| f32::from(i) / 26.0).collect());
        with_volume_grid("gpu_grid", |vg| enable_quantity(vg, "s"));
    }

    /// Returns whether any registered structure holds GPU render data.
    fn any_structure_has_render_data() -> bool {
        let pc = with_point_cloud_ref("gpu_pc", |pc| pc.render_data().is_some());
        let mesh = with_surface_mesh_ref("gpu_mesh", |m| m.render_data().is_some());
        let cn = with_curve_network_ref("gpu_cn", |cn| cn.render_data().is_some());
        let grid = with_volume_grid_ref("gpu_grid", |vg| vg.render_data().is_some());
        [pc, mesh, cn, grid].into_iter().any(|r| r == Some(true))
    }

    #[test]
    fn test_headless_render_drops_its_gpu_resources() {
        let _guard = setup();
        register_gpu_scene();
        let Ok(first) = render_to_image(96, 72) else {
            eprintln!("Skipping headless GPU hygiene test: no GPU adapter available");
            return;
        };
        assert!(
            !any_structure_has_render_data(),
            "a headless render must not leave GPU resources of its device"
        );
        let second = render_to_image(96, 72).expect("second headless render");
        assert_eq!(first, second, "a second headless render must match");
    }

    #[test]
    fn test_new_engine_rebuilds_gpu_resources() {
        let _guard = setup();
        register_gpu_scene();
        let Some(mut first_engine) = LiveRenderer::new() else {
            eprintln!("Skipping engine switch test: no GPU adapter available");
            return;
        };
        let first = first_engine.frame();
        assert!(any_structure_has_render_data());
        drop(first_engine);

        // A new engine has a new device. The window clears the GPU resources
        // when it creates its engine; this test does the same.
        app::clear_structure_gpu_resources();
        let mut second_engine = LiveRenderer::new().expect("second engine");
        let second = second_engine.frame();
        assert_eq!(first, second, "the new engine must draw the same frame");
    }

    #[test]
    fn test_curve_pick_radius_follows_the_radius() {
        let _guard = setup();
        register_curve_network(
            "pick_radius_cn",
            vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            vec![[0, 1]],
        );
        with_curve_network("pick_radius_cn", |cn| {
            cn.set_render_mode(1);
            cn.set_radius(0.01, false);
        });
        let Some(mut renderer) = LiveRenderer::new() else {
            eprintln!("Skipping curve pick radius test: no GPU adapter available");
            return;
        };
        let app = &mut renderer.app;
        let engine = app.engine.as_mut().expect("engine");
        let (width, height) = (engine.width, engine.height);
        engine.init_pick_buffers(width, height);

        // The first frames create the pick resources with the small radius.
        app.render_pick_frame();
        app.render_pick_frame();
        // A point 0.15 away from the tube axis, in the image plane.
        let pixel_of = |engine: &RenderEngine, p: Vec3| {
            let clip = engine.camera.view_projection_matrix() * p.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            let x = f32::midpoint(ndc.x, 1.0) * engine.width as f32;
            let y = f32::midpoint(-ndc.y, 1.0) * engine.height as f32;
            (x as u32, y as u32)
        };
        let engine = app.engine.as_ref().expect("engine");
        let up = engine.camera.up_direction.to_vec3();
        let (x, y) = pixel_of(engine, up * 0.15);
        let hit = |app: &app::App| {
            let engine = app.engine.as_ref().expect("engine");
            engine.pick_at(x, y).is_some_and(|index| index > 0)
        };
        assert!(!hit(app), "the thin tube must not cover the test pixel");

        // The pick pass of a frame uses the tube geometry of the frame before.
        // So two frames after the change, the pick must use the new radius.
        with_curve_network("pick_radius_cn", |cn| {
            cn.set_radius(0.3, false);
        });
        app.render_pick_frame();
        app.render_pick_frame();
        assert!(hit(app), "the pick pass must use the new tube radius");
    }
}
