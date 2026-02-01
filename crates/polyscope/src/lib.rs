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
// #[must_use] design: Setter methods intentionally omit #[must_use] since
// the mutation happens in-place; the &Self return is just for chaining convenience.
#![allow(clippy::must_use_candidate)]
// Documentation formatting: Backtick linting is too strict for doc comments.
#![allow(clippy::doc_markdown)]

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
    error::{PolyscopeError, Result},
    gizmo::{GizmoAxis, GizmoConfig, GizmoMode, GizmoSpace, Transform},
    group::Group,
    options::Options,
    pick::{PickResult, Pickable},
    quantity::{ParamCoordsType, ParamVizStyle, Quantity, QuantityKind},
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
pub use polyscope_structures::volume_grid::VolumeGridVizMode;

// Re-export module APIs
pub use camera_view::*;
pub use curve_network::*;
pub use floating::*;
pub use gizmo::*;
pub use groups::*;
pub use init::*;
pub use point_cloud::*;
pub use screenshot::*;
pub use slice_plane::*;
pub use surface_mesh::*;
pub use transform::*;
pub use ui_sync::*;
pub use headless::*;
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

/// Sets a callback that is invoked when files are dropped onto the polyscope window.
///
/// The callback receives a slice of file paths that were dropped.
///
/// # Example
/// ```no_run
/// polyscope::set_file_drop_callback(|paths| {
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
/// polyscope::load_blendable_material("metal", [
///     "assets/metal_r.hdr",
///     "assets/metal_g.hdr",
///     "assets/metal_b.hdr",
///     "assets/metal_k.hdr",
/// ]);
/// ```
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) {
    with_context_mut(|ctx| {
        ctx.material_load_queue.push(
            polyscope_core::state::MaterialLoadRequest::Blendable {
                name: name.to_string(),
                filenames: [
                    filenames[0].to_string(),
                    filenames[1].to_string(),
                    filenames[2].to_string(),
                    filenames[3].to_string(),
                ],
            },
        );
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
/// polyscope::load_static_material("stone", "assets/stone.jpg");
/// ```
pub fn load_static_material(name: &str, filename: &str) {
    with_context_mut(|ctx| {
        ctx.material_load_queue.push(
            polyscope_core::state::MaterialLoadRequest::Static {
                name: name.to_string(),
                path: filename.to_string(),
            },
        );
    });
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
            assert_eq!(cn.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
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
        assert_eq!(color, Some(Vec4::new(0.5, 0.5, 0.5, 1.0)));
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
        assert_eq!(handle.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
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
        assert_eq!(handle.color(), Vec4::new(1.0, 0.0, 0.0, 1.0));
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
