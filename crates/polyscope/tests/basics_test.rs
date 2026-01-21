//! Basic integration tests for polyscope-rs.
//!
//! Note: Due to polyscope using global state that can only be initialized once
//! per process (OnceLock), all tests are combined into a single test function.
//!
//! Tests that require a window (show()) are marked #[ignore]
//! and should be run manually with: cargo test -- --ignored

use polyscope::*;

/// Main integration test that runs all basic tests in sequence.
///
/// This is structured as a single test because polyscope uses global state
/// that cannot be re-initialized after shutdown within the same process.
#[test]
fn test_basics() {
    // Initialize polyscope
    init().expect("init failed");
    assert!(is_initialized());

    // Test 1: Register point cloud
    {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let _pc = register_point_cloud("test cloud", points);

        assert!(get_point_cloud("test cloud").is_some());
        assert!(get_point_cloud("nonexistent").is_none());
    }

    // Test 2: Register surface mesh
    {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![glam::UVec3::new(0, 1, 2)];
        let _mesh = register_surface_mesh("test mesh", verts, faces);

        assert!(get_surface_mesh("test mesh").is_some());
        assert!(get_surface_mesh("nonexistent").is_none());
    }

    // Test 3: Remove structure
    {
        let points = vec![Vec3::new(0.0, 0.0, 0.0)];
        register_point_cloud("to_remove", points);

        assert!(get_point_cloud("to_remove").is_some());

        remove_structure("to_remove");

        assert!(get_point_cloud("to_remove").is_none());
    }

    // Test 4: Remove all structures
    {
        let points = vec![Vec3::new(0.0, 0.0, 0.0)];
        register_point_cloud("cloud1", points.clone());
        register_point_cloud("cloud2", points);

        remove_all_structures();

        assert!(get_point_cloud("cloud1").is_none());
        assert!(get_point_cloud("cloud2").is_none());
    }

    // Test 5: Point cloud quantities
    {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let pc = register_point_cloud("with_quantities", points);

        pc.add_scalar_quantity("scalars", vec![0.0, 0.5, 1.0]);
        pc.add_vector_quantity("vectors", vec![Vec3::X, Vec3::Y, Vec3::Z]);
        pc.add_color_quantity(
            "colors",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
        );
    }

    // Shutdown
    shutdown();
}

/// This test requires a display and opens a window.
/// Run with: cargo test test_show_window -- --ignored
#[test]
#[ignore]
fn test_show_window() {
    init().expect("init failed");

    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    register_point_cloud("test points", points);

    show();

    shutdown();
}
