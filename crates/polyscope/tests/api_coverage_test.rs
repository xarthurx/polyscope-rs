//! API coverage integration tests for polyscope-rs.
//!
//! These tests verify the public API for slice planes, groups, transforms,
//! and other features not covered by the basic or headless tests.
//!
//! Note: Due to polyscope using global state that can only be initialized once
//! per process (OnceLock), all tests are combined into a single test function.

use polyscope_rs::*;

/// Main API coverage test that runs all subtests in sequence.
#[test]
fn api_coverage_tests() {
    // Initialize polyscope context
    let _ = init();

    // ========================================================================
    // SLICE PLANE TESTS
    // ========================================================================

    // --- Test: Basic slice plane creation and retrieval ---
    {
        remove_all_slice_planes();
        remove_all_structures();

        // Create a structure so the slice plane has something to slice
        register_point_cloud("sp_test_pc", vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]);

        let plane = add_slice_plane("test_plane");
        assert_eq!(plane.name(), "test_plane");

        // Verify it exists
        assert!(get_slice_plane("test_plane").is_some());
        assert!(get_slice_plane("nonexistent").is_none());

        // Check it's in the list
        let all_planes = get_all_slice_planes();
        assert!(all_planes.contains(&"test_plane".to_string()));
    }

    // --- Test: Slice plane with pose ---
    {
        remove_all_slice_planes();

        let origin = Vec3::new(1.0, 2.0, 3.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let plane = add_slice_plane_with_pose("posed_plane", origin, normal);

        // Verify pose was set
        let retrieved_origin = plane.origin();
        let retrieved_normal = plane.normal();
        assert!((retrieved_origin - origin).length() < 0.001);
        assert!((retrieved_normal - normal).length() < 0.001);
    }

    // --- Test: Slice plane properties ---
    {
        remove_all_slice_planes();

        let plane = add_slice_plane("props_plane");

        // Test enabled
        assert!(plane.is_enabled()); // default is enabled
        plane.set_enabled(false);
        assert!(!plane.is_enabled());
        plane.set_enabled(true);
        assert!(plane.is_enabled());

        // Test draw_plane
        plane.set_draw_plane(false);
        assert!(!plane.draw_plane());
        plane.set_draw_plane(true);
        assert!(plane.draw_plane());

        // Test draw_widget
        plane.set_draw_widget(false);
        assert!(!plane.draw_widget());
        plane.set_draw_widget(true);
        assert!(plane.draw_widget());

        // Test color
        let red = Vec3::new(1.0, 0.0, 0.0);
        plane.set_color(red);
        let color = plane.color();
        assert!((color.x - 1.0).abs() < 0.001);
        assert!(color.y.abs() < 0.001);
        assert!(color.z.abs() < 0.001);

        // Test transparency
        plane.set_transparency(0.7);
        assert!((plane.transparency() - 0.7).abs() < 0.001);

        // Test plane_size
        plane.set_plane_size(2.5);
        assert!((plane.plane_size() - 2.5).abs() < 0.001);
    }

    // --- Test: Slice plane pose methods ---
    {
        remove_all_slice_planes();

        let plane = add_slice_plane("pose_test_plane");

        // Test set_origin
        let new_origin = Vec3::new(5.0, 6.0, 7.0);
        plane.set_origin(new_origin);
        assert!((plane.origin() - new_origin).length() < 0.001);

        // Test set_normal
        let new_normal = Vec3::new(1.0, 0.0, 0.0);
        plane.set_normal(new_normal);
        assert!((plane.normal() - new_normal).length() < 0.001);

        // Test set_pose
        let pose_origin = Vec3::new(10.0, 20.0, 30.0);
        let pose_normal = Vec3::new(0.0, 0.0, 1.0);
        plane.set_pose(pose_origin, pose_normal);
        assert!((plane.origin() - pose_origin).length() < 0.001);
        assert!((plane.normal() - pose_normal).length() < 0.001);
    }

    // --- Test: Remove slice plane ---
    {
        remove_all_slice_planes();

        add_slice_plane("to_remove");
        assert!(get_slice_plane("to_remove").is_some());

        remove_slice_plane("to_remove");
        assert!(get_slice_plane("to_remove").is_none());
    }

    // --- Test: Multiple slice planes ---
    {
        remove_all_slice_planes();

        add_slice_plane("plane_a");
        add_slice_plane("plane_b");
        add_slice_plane("plane_c");

        let all = get_all_slice_planes();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"plane_a".to_string()));
        assert!(all.contains(&"plane_b".to_string()));
        assert!(all.contains(&"plane_c".to_string()));

        remove_all_slice_planes();
        assert!(get_all_slice_planes().is_empty());
    }

    // ========================================================================
    // GROUP TESTS
    // ========================================================================

    // --- Test: Basic group creation and retrieval ---
    {
        remove_all_structures();

        let group = create_group("test_group");
        assert_eq!(group.name(), "test_group");

        assert!(get_group("test_group").is_some());
        assert!(get_group("nonexistent").is_none());

        let all_groups = get_all_groups();
        assert!(all_groups.contains(&"test_group".to_string()));
    }

    // --- Test: Group enabled state ---
    {
        let group = create_group("enabled_test");

        assert!(group.is_enabled()); // default
        group.set_enabled(false);
        assert!(!group.is_enabled());
        group.set_enabled(true);
        assert!(group.is_enabled());
    }

    // --- Test: Adding structures to groups ---
    {
        remove_all_structures();

        // Create structures
        register_point_cloud("group_pc", vec![Vec3::ZERO]);
        register_surface_mesh(
            "group_mesh",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![[0u32, 1, 2]],
        );
        register_curve_network_line("group_cn", vec![Vec3::ZERO, Vec3::X]);

        // Create group and add structures
        let group = create_group("struct_group");
        group.add_point_cloud("group_pc");
        group.add_surface_mesh("group_mesh");
        group.add_curve_network("group_cn");

        assert_eq!(group.num_structures(), 3);

        // Remove a structure from group
        group.remove_structure("PointCloud", "group_pc");
        assert_eq!(group.num_structures(), 2);
    }

    // --- Test: Child groups ---
    {
        let parent = create_group("parent_group");
        let _child1 = create_group("child_group_1");
        let _child2 = create_group("child_group_2");

        parent.add_child_group("child_group_1");
        parent.add_child_group("child_group_2");

        assert_eq!(parent.num_child_groups(), 2);

        parent.remove_child_group("child_group_1");
        assert_eq!(parent.num_child_groups(), 1);

        // Cleanup
        remove_group("parent_group");
        remove_group("child_group_1");
        remove_group("child_group_2");
    }

    // --- Test: Remove group ---
    {
        create_group("to_remove_group");
        assert!(get_group("to_remove_group").is_some());

        remove_group("to_remove_group");
        assert!(get_group("to_remove_group").is_none());
    }

    // ========================================================================
    // TRANSFORM TESTS
    // ========================================================================

    // --- Test: Point cloud transform ---
    {
        remove_all_structures();

        register_point_cloud("transform_pc", vec![Vec3::ZERO, Vec3::X]);

        // Default transform is identity
        let initial = get_point_cloud_transform("transform_pc").unwrap();
        assert!((initial - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 0.001));

        // Set a translation transform
        let translation = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        set_point_cloud_transform("transform_pc", translation);

        let retrieved = get_point_cloud_transform("transform_pc").unwrap();
        assert!((retrieved - translation).abs_diff_eq(Mat4::ZERO, 0.001));
    }

    // --- Test: Surface mesh transform ---
    {
        remove_all_structures();

        register_surface_mesh(
            "transform_mesh",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![[0u32, 1, 2]],
        );

        let rotation = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
        set_surface_mesh_transform("transform_mesh", rotation);

        let retrieved = get_surface_mesh_transform("transform_mesh").unwrap();
        assert!((retrieved - rotation).abs_diff_eq(Mat4::ZERO, 0.001));
    }

    // --- Test: Curve network transform ---
    {
        remove_all_structures();

        register_curve_network_line("transform_cn", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

        let scale = Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0));
        set_curve_network_transform("transform_cn", scale);

        let retrieved = get_curve_network_transform("transform_cn").unwrap();
        assert!((retrieved - scale).abs_diff_eq(Mat4::ZERO, 0.001));
    }

    // --- Test: Volume mesh transform ---
    {
        remove_all_structures();

        register_tet_mesh(
            "transform_vm",
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
                Vec3::new(0.5, 0.5, 1.0),
            ],
            vec![[0, 1, 2, 3]],
        );

        let combined = Mat4::from_scale_rotation_translation(
            Vec3::new(0.5, 0.5, 0.5),
            glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 0.0, 0.0),
        );
        set_volume_mesh_transform("transform_vm", combined);

        let retrieved = get_volume_mesh_transform("transform_vm").unwrap();
        assert!((retrieved - combined).abs_diff_eq(Mat4::ZERO, 0.001));
    }

    // --- Test: Nonexistent structure transform ---
    {
        assert!(get_point_cloud_transform("nonexistent").is_none());
        assert!(get_surface_mesh_transform("nonexistent").is_none());
        assert!(get_curve_network_transform("nonexistent").is_none());
        assert!(get_volume_mesh_transform("nonexistent").is_none());
    }

    // ========================================================================
    // SURFACE MESH HANDLE METHOD TESTS
    // ========================================================================

    // --- Test: Surface mesh appearance methods ---
    {
        remove_all_structures();

        let mesh = register_surface_mesh(
            "appearance_mesh",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::ONE],
            vec![[0u32, 1, 2], [1, 2, 3]],
        );

        // Test chaining
        mesh.set_surface_color(Vec3::new(1.0, 0.0, 0.0))
            .set_edge_color(Vec3::new(0.0, 0.0, 0.0))
            .set_edge_width(2.0)
            .set_show_edges(true)
            .set_backface_color(Vec3::new(0.5, 0.5, 0.5))
            .set_transparency(0.3)
            .set_material("wax");

        // Verify by rendering (just checking no crash)
        // Actual verification would require headless render
    }

    // --- Test: Surface mesh intrinsic vector quantity ---
    {
        remove_all_structures();

        let mesh = register_surface_mesh(
            "intrinsic_mesh",
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
            ],
            vec![[0u32, 1, 2]],
        );

        // Auto-computed basis
        mesh.add_vertex_intrinsic_vector_quantity_auto(
            "auto_intrinsic",
            vec![
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.5, 0.5),
            ],
        );

        // Explicit basis
        let basis_x = vec![Vec3::X, Vec3::X, Vec3::X];
        let basis_y = vec![Vec3::Y, Vec3::Y, Vec3::Y];
        mesh.add_vertex_intrinsic_vector_quantity(
            "explicit_intrinsic",
            vec![
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.5, 0.5),
            ],
            basis_x,
            basis_y,
        );

        // Face intrinsic
        mesh.add_face_intrinsic_vector_quantity_auto("face_intrinsic", vec![Vec2::new(1.0, 0.0)]);
    }

    // --- Test: Surface mesh one-form quantity ---
    {
        remove_all_structures();

        let mesh = register_surface_mesh(
            "oneform_mesh",
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
            ],
            vec![[0u32, 1, 2]],
        );

        // Triangle has 3 edges
        mesh.add_one_form_quantity("one_form", vec![1.0, -0.5, 0.5], vec![true, true, false]);
    }

    // --- Test: Surface mesh RGBA color quantities ---
    {
        remove_all_structures();

        let mesh = register_surface_mesh(
            "rgba_mesh",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![[0u32, 1, 2]],
        );

        // Vertex RGBA
        mesh.add_vertex_color_quantity_with_alpha(
            "vertex_rgba",
            vec![
                Vec4::new(1.0, 0.0, 0.0, 0.5),
                Vec4::new(0.0, 1.0, 0.0, 0.7),
                Vec4::new(0.0, 0.0, 1.0, 0.9),
            ],
        );

        // Face RGBA
        mesh.add_face_color_quantity_with_alpha("face_rgba", vec![Vec4::new(1.0, 1.0, 0.0, 0.5)]);
    }

    // --- Test: Surface mesh parameterization ---
    {
        remove_all_structures();

        let mesh = register_surface_mesh(
            "param_mesh",
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![[0u32, 1, 2]],
        );

        // Vertex parameterization
        mesh.add_vertex_parameterization_quantity(
            "vertex_uv",
            vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
        );

        // Corner parameterization (3 corners for 1 triangle)
        mesh.add_corner_parameterization_quantity(
            "corner_uv",
            vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
        );
    }

    // ========================================================================
    // CURVE NETWORK HANDLE METHOD TESTS
    // ========================================================================

    // --- Test: Curve network handle methods ---
    {
        remove_all_structures();

        let nodes = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let edges = vec![[0, 1], [1, 2], [2, 3]];
        let cn = register_curve_network("cn_handle_test", nodes, edges);

        // Test appearance methods
        cn.set_color(Vec3::new(1.0, 0.0, 0.0))
            .set_radius(0.05, true)
            .set_material("clay");
    }

    // --- Test: Curve network quantities via with_curve_network ---
    {
        remove_all_structures();

        let nodes = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let edges = vec![[0, 1], [1, 2], [2, 3]];
        register_curve_network("cn_quant_test", nodes, edges);

        // Node quantities
        with_curve_network("cn_quant_test", |cn| {
            cn.add_node_scalar_quantity("node_scalar", vec![0.0, 0.33, 0.66, 1.0]);
            cn.add_node_vector_quantity("node_vec", vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE]);
            cn.add_node_color_quantity(
                "node_color",
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(1.0, 1.0, 0.0),
                ],
            );
        });

        // Edge quantities (3 edges)
        with_curve_network("cn_quant_test", |cn| {
            cn.add_edge_scalar_quantity("edge_scalar", vec![0.25, 0.5, 0.75]);
            cn.add_edge_vector_quantity("edge_vec", vec![Vec3::X, Vec3::Y, Vec3::Z]);
            cn.add_edge_color_quantity(
                "edge_color",
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ],
            );
        });
    }

    // ========================================================================
    // VOLUME GRID TESTS
    // ========================================================================

    // --- Test: Volume grid handle methods ---
    {
        remove_all_structures();

        let vg = register_volume_grid(
            "vg_handle_test",
            glam::UVec3::new(3, 3, 3),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        );

        // Node scalar (3x3x3 = 27 values)
        let node_values: Vec<f32> = (0..27).map(|i| i as f32 / 26.0).collect();
        vg.add_node_scalar_quantity("node_data", node_values);

        // Cell scalar (2x2x2 = 8 values)
        let cell_values: Vec<f32> = (0..8).map(|i| i as f32 / 7.0).collect();
        vg.add_cell_scalar_quantity("cell_data", cell_values);
    }

    // ========================================================================
    // VOLUME MESH HANDLE TESTS
    // ========================================================================

    // --- Test: Volume mesh quantities ---
    {
        remove_all_structures();

        let vm = register_tet_mesh(
            "vm_quant_test",
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
                Vec3::new(0.5, 0.5, 1.0),
            ],
            vec![[0, 1, 2, 3]],
        );

        vm.add_vertex_scalar_quantity("vs", vec![0.0, 0.33, 0.66, 1.0]);
        vm.add_vertex_vector_quantity("vv", vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE]);
        vm.add_vertex_color_quantity(
            "vc",
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
        );

        vm.add_cell_scalar_quantity("cs", vec![0.5]);
        vm.add_cell_vector_quantity("cv", vec![Vec3::ONE]);
        vm.add_cell_color_quantity("cc", vec![Vec3::new(0.5, 0.5, 0.5)]);
    }

    // ========================================================================
    // CLEANUP
    // ========================================================================

    remove_all_structures();
    remove_all_slice_planes();
}
