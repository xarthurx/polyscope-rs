//! Headless rendering integration tests.
//!
//! These tests verify that structures can be rendered without a window.
//! They require a GPU adapter (real or software fallback). On CI without
//! GPU support, they will fail at engine creation and can be skipped
//! with `cargo test -- --skip headless`.

use polyscope::*;

/// Helper: check that a pixel buffer is not all-black and not all-background.
fn has_nontrivial_content(pixels: &[u8], width: u32, height: u32) -> bool {
    let total = (width * height) as usize;
    assert_eq!(pixels.len(), total * 4, "pixel buffer size mismatch");

    // Check not all-black
    let all_black = pixels.chunks(4).all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0);

    // Check not uniform (all same color)
    let first = &pixels[0..4];
    let all_uniform = pixels.chunks(4).all(|px| px == first);

    !all_black && !all_uniform
}

/// All headless render tests are combined into a single test function
/// because polyscope uses OnceLock<RwLock<Context>> for global state,
/// which can only be initialized once per process.
#[test]
fn headless_render_tests() {
    // Initialize polyscope context
    let _ = init();

    // --- Test 1: Empty scene ---
    {
        remove_all_structures();
        let result = render_to_image(200, 150);
        match result {
            Ok(pixels) => {
                assert_eq!(pixels.len(), 200 * 150 * 4);
                // Empty scene should be uniform background color
                let first = &pixels[0..4];
                let all_same = pixels.chunks(4).all(|px| px == first);
                assert!(all_same, "empty scene should be uniform background color");
            }
            Err(e) => {
                // GPU not available — skip remaining tests
                eprintln!("Skipping headless tests: no GPU adapter available ({e})");
                return;
            }
        }
    }

    // --- Test 2: Point cloud ---
    {
        remove_all_structures();
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        register_point_cloud("test_pc", points);

        let pixels = render_to_image(400, 300).expect("point cloud render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "point cloud render should produce non-trivial output"
        );
    }

    // --- Test 3: Surface mesh ---
    {
        remove_all_structures();
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let faces: Vec<Vec<u32>> = vec![
            vec![0, 1, 2],
            vec![0, 1, 3],
            vec![1, 2, 3],
            vec![0, 2, 3],
        ];
        register_surface_mesh("test_mesh", vertices, faces);

        let pixels = render_to_image(400, 300).expect("surface mesh render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "surface mesh render should produce non-trivial output"
        );
    }

    // --- Test 4: Curve network ---
    {
        remove_all_structures();
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let edges = vec![[0, 1], [1, 2], [2, 3], [3, 0]];
        register_curve_network("test_cn", nodes, edges);

        let pixels = render_to_image(400, 300).expect("curve network render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "curve network render should produce non-trivial output"
        );
    }

    // --- Test 5: Multiple structures ---
    {
        remove_all_structures();

        // Point cloud
        register_point_cloud(
            "multi_pc",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        );

        // Surface mesh
        register_surface_mesh(
            "multi_mesh",
            vec![
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(2.5, 1.0, 0.0),
            ],
            vec![vec![0, 1, 2]],
        );

        // Curve network
        register_curve_network(
            "multi_cn",
            vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-1.0, 1.0, 0.0)],
            vec![[0, 1]],
        );

        let pixels = render_to_image(400, 300).expect("multi-structure render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "multi-structure render should produce non-trivial output"
        );
    }

    // --- Test 6: render_to_file ---
    {
        remove_all_structures();
        register_point_cloud(
            "file_test_pc",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        );

        let tmp_path = "/tmp/polyscope_headless_test.png";
        render_to_file(tmp_path, 200, 150).expect("render_to_file failed");

        // Verify file exists and is a valid PNG
        let metadata = std::fs::metadata(tmp_path).expect("screenshot file should exist");
        assert!(metadata.len() > 100, "PNG file should have non-trivial size");

        // Check PNG signature
        let data = std::fs::read(tmp_path).expect("should be able to read screenshot");
        assert_eq!(&data[0..4], &[0x89, b'P', b'N', b'G'], "should be valid PNG");

        // Clean up
        let _ = std::fs::remove_file(tmp_path);
    }

    // --- Test 7: Different resolutions ---
    {
        remove_all_structures();
        register_point_cloud("res_test", vec![Vec3::ZERO, Vec3::X]);

        // Small
        let pixels = render_to_image(64, 64).expect("small render failed");
        assert_eq!(pixels.len(), 64 * 64 * 4);

        // Large
        let pixels = render_to_image(1024, 768).expect("large render failed");
        assert_eq!(pixels.len(), 1024 * 768 * 4);
    }

    // Clean up
    remove_all_structures();
}
