//! Headless rendering integration tests.
//!
//! These tests verify that structures can be rendered without a window.
//! They require a GPU adapter (real or software fallback). On CI without
//! GPU support, they will fail at engine creation and can be skipped
//! with `cargo test -- --skip headless`.

use polyscope_rs::*;

/// Helper: check that a pixel buffer is not all-black and not all-background.
fn has_nontrivial_content(pixels: &[u8], width: u32, height: u32) -> bool {
    let total = (width * height) as usize;
    assert_eq!(pixels.len(), total * 4, "pixel buffer size mismatch");

    // Check not all-black
    let all_black = pixels
        .chunks(4)
        .all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0);

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
        let faces: Vec<Vec<u32>> = vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]];
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
        register_point_cloud("multi_pc", vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]);

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
        register_point_cloud("file_test_pc", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

        let tmp_path = "/tmp/polyscope_headless_test.png";
        render_to_file(tmp_path, 200, 150).expect("render_to_file failed");

        // Verify file exists and is a valid PNG
        let metadata = std::fs::metadata(tmp_path).expect("screenshot file should exist");
        assert!(
            metadata.len() > 100,
            "PNG file should have non-trivial size"
        );

        // Check PNG signature
        let data = std::fs::read(tmp_path).expect("should be able to read screenshot");
        assert_eq!(
            &data[0..4],
            &[0x89, b'P', b'N', b'G'],
            "should be valid PNG"
        );

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

    // --- Test 8: Point cloud quantities ---
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
        let pc = register_point_cloud("pc_quant", points);
        pc.add_scalar_quantity("height", vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.5, 0.3]);
        pc.add_vector_quantity(
            "velocity",
            vec![
                Vec3::X,
                Vec3::Y,
                Vec3::Z,
                Vec3::NEG_X,
                Vec3::NEG_Y,
                Vec3::NEG_Z,
                Vec3::ONE,
                Vec3::ZERO,
            ],
        );
        pc.add_color_quantity(
            "rgb",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(1.0, 1.0, 1.0),
            ],
        );

        let pixels = render_to_image(400, 300).expect("point cloud quantities render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "point cloud with quantities should produce non-trivial output"
        );
    }

    // --- Test 9: Surface mesh scalar + color quantities ---
    {
        remove_all_structures();
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let faces: Vec<Vec<u32>> = vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]];
        let mesh = register_surface_mesh("mesh_sq", vertices, faces);
        mesh.add_vertex_scalar_quantity("vscalar", vec![0.0, 0.5, 1.0, 0.75]);
        mesh.add_face_scalar_quantity("fscalar", vec![0.1, 0.4, 0.7, 1.0]);
        mesh.add_vertex_color_quantity(
            "vcolor",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
        );
        mesh.add_face_color_quantity(
            "fcolor",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
        );

        let pixels = render_to_image(400, 300).expect("mesh scalar+color render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "mesh with scalar+color quantities should produce non-trivial output"
        );
    }

    // --- Test 10: Surface mesh vector + parameterization ---
    {
        remove_all_structures();
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let faces: Vec<Vec<u32>> = vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]];
        let mesh = register_surface_mesh("mesh_vp", vertices, faces);
        mesh.add_vertex_vector_quantity(
            "vvec",
            vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE.normalize()],
        );
        mesh.add_face_vector_quantity("fvec", vec![Vec3::Z, Vec3::Z, Vec3::Y, Vec3::X]);
        mesh.add_vertex_parameterization_quantity(
            "vparam",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.5, 1.0),
                Vec2::new(0.5, 0.5),
            ],
        );

        let pixels = render_to_image(400, 300).expect("mesh vector+param render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "mesh with vector+param quantities should produce non-trivial output"
        );
    }

    // --- Test 11: Surface mesh appearance ---
    {
        remove_all_structures();
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let faces: Vec<Vec<u32>> = vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]];
        let mesh = register_surface_mesh("mesh_appear", vertices, faces);
        mesh.set_show_edges(true);
        mesh.set_transparency(0.5);
        mesh.set_material("wax");

        let pixels = render_to_image(400, 300).expect("mesh appearance render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "mesh with edges+transparency+material should produce non-trivial output"
        );
    }

    // --- Test 12: Volume mesh + quantities ---
    {
        remove_all_structures();
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        let vm = register_tet_mesh("vm_test", vertices, tets);
        vm.add_vertex_scalar_quantity("vs", vec![0.0, 0.5, 1.0, 0.75]);
        vm.add_cell_scalar_quantity("cs", vec![0.5]);

        let pixels = render_to_image(400, 300).expect("volume mesh render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "volume mesh with quantities should produce non-trivial output"
        );
    }

    // --- Test 13: Volume grid + scalar quantity ---
    {
        remove_all_structures();
        let dim = glam::UVec3::new(4, 4, 4);
        let vg = register_volume_grid(
            "vg_test",
            dim,
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        // 4x4x4 = 64 node values
        let values: Vec<f32> = (0..64).map(|i| i as f32 / 63.0).collect();
        vg.add_node_scalar_quantity("ns", values);

        let pixels = render_to_image(400, 300).expect("volume grid render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "volume grid with scalar quantity should produce non-trivial output"
        );
    }

    // --- Test 14: Camera view ---
    {
        remove_all_structures();
        register_camera_view_look_at(
            "cam_test",
            Vec3::new(3.0, 3.0, 3.0), // position
            Vec3::ZERO,               // target
            Vec3::Y,                  // up
            45.0,                     // fov degrees
            1.33,                     // aspect ratio
        );

        let pixels = render_to_image(400, 300).expect("camera view render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        // Camera view is a thin wireframe frustum; just verify no crash
    }

    // --- Test 15: Curve network variants ---
    {
        remove_all_structures();
        let line_nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        register_curve_network_line("cn_line", line_nodes);

        let loop_nodes = vec![
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 3.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];
        register_curve_network_loop("cn_loop", loop_nodes);

        let pixels = render_to_image(400, 300).expect("curve network variants render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "curve network line+loop should produce non-trivial output"
        );
    }

    // --- Test 16: Floating quantities ---
    {
        remove_all_structures();
        remove_all_floating_quantities();

        // 16x16 scalar image
        let scalar_values: Vec<f32> = (0..16 * 16).map(|i| i as f32 / 255.0).collect();
        register_floating_scalar_image("fsi_test", 16, 16, scalar_values);

        // 16x16 color image
        let color_values: Vec<Vec3> = (0..16 * 16)
            .map(|i| {
                let t = i as f32 / 255.0;
                Vec3::new(t, 1.0 - t, 0.5)
            })
            .collect();
        register_floating_color_image("fci_test", 16, 16, color_values);

        let pixels = render_to_image(400, 300).expect("floating quantities render failed");
        assert_eq!(pixels.len(), 400 * 300 * 4);
        // Floating images are screen-space overlays; just verify no crash

        remove_all_floating_quantities();
    }

    // --- Test 17: Polygon mesh (mixed tri + quad) ---
    {
        remove_all_structures();
        // A simple shape with 1 triangle and 1 quad face
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
            Vec3::new(0.5, 1.5, 0.0), // 4
        ];
        let faces: Vec<Vec<u32>> = vec![
            vec![0, 1, 2, 3], // quad
            vec![3, 2, 4],    // triangle
        ];
        let mesh = register_surface_mesh("poly_mesh", vertices, faces);
        mesh.add_face_scalar_quantity("fs", vec![0.3, 0.8]);

        let pixels = render_to_image(400, 300).expect("polygon mesh render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "polygon mesh with face scalar should produce non-trivial output"
        );
    }

    // --- Test 18: Multi-structure with quantities ---
    {
        remove_all_structures();

        // Point cloud + scalar
        let pc = register_point_cloud("multi_pc_q", vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]);
        pc.add_scalar_quantity("s", vec![0.0, 0.33, 0.66, 1.0]);

        // Surface mesh + vertex color
        let mesh = register_surface_mesh(
            "multi_mesh_q",
            vec![
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(2.5, 1.0, 0.0),
            ],
            vec![vec![0u32, 1, 2]],
        );
        mesh.add_vertex_color_quantity(
            "vc",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
        );

        // Curve network
        register_curve_network(
            "multi_cn_q",
            vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-1.0, 1.0, 0.0)],
            vec![[0, 1]],
        );

        // Volume mesh + cell scalar
        let vm = register_tet_mesh(
            "multi_vm_q",
            vec![
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(1.0, 0.0, 2.0),
                Vec3::new(0.5, 1.0, 2.0),
                Vec3::new(0.5, 0.5, 3.0),
            ],
            vec![[0, 1, 2, 3]],
        );
        vm.add_cell_scalar_quantity("cs", vec![0.7]);

        let pixels =
            render_to_image(400, 300).expect("multi-structure with quantities render failed");
        assert!(
            has_nontrivial_content(&pixels, 400, 300),
            "multi-structure with quantities should produce non-trivial output"
        );
    }

    // Clean up
    remove_all_structures();
}
