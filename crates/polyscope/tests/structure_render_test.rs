//! Headless render tests for structure quantities, materials and edge cases.
//!
//! All checks share one `#[test]` function because polyscope keeps its state
//! in a global context that a process can initialize only once.

use polyscope_rs::*;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

/// Returns the number of pixels that differ between two RGBA buffers.
fn count_changed_pixels(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len(), "pixel buffer sizes differ");
    a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count()
}

fn render() -> Vec<u8> {
    render_to_image(WIDTH, HEIGHT).expect("headless render failed")
}

/// Sets the enabled state of one quantity on a curve network.
fn set_curve_quantity_enabled(structure: &str, quantity: &str, enabled: bool) {
    with_curve_network(structure, |cn| {
        cn.get_quantity_mut(quantity)
            .expect("quantity not found")
            .set_enabled(enabled);
    });
}

/// Checks that node and edge vector quantities on a curve network are drawn.
fn check_curve_network_vectors() {
    remove_all_structures();
    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    register_curve_network("cn_vectors", nodes, vec![[0, 1], [1, 2], [2, 3], [3, 0]]);
    with_curve_network("cn_vectors", |cn| {
        cn.add_node_vector_quantity("node_vecs", vec![Vec3::new(0.3, 0.3, 0.5); 4]);
        cn.add_edge_vector_quantity("edge_vecs", vec![Vec3::new(-0.3, 0.3, 0.5); 4]);
    });

    let disabled = render();

    set_curve_quantity_enabled("cn_vectors", "node_vecs", true);
    let node_enabled = render();
    assert!(
        count_changed_pixels(&disabled, &node_enabled) > 0,
        "an enabled curve node vector quantity must change the image"
    );
    // A second render uses a new device, so the GPU data must be rebuilt.
    let node_rerender = render();
    assert!(
        count_changed_pixels(&disabled, &node_rerender) > 0,
        "curve node vectors must survive a headless re-render"
    );

    set_curve_quantity_enabled("cn_vectors", "node_vecs", false);
    set_curve_quantity_enabled("cn_vectors", "edge_vecs", true);
    let edge_enabled = render();
    assert!(
        count_changed_pixels(&disabled, &edge_enabled) > 0,
        "an enabled curve edge vector quantity must change the image"
    );
    assert!(
        count_changed_pixels(&node_enabled, &edge_enabled) > 0,
        "node and edge vectors start at different points"
    );
}

/// Returns the eight corners of the unit cube in hex-cell order.
fn unit_cube_vertices() -> Vec<Vec3> {
    vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ]
}

/// Sets the enabled state of one quantity on a volume mesh.
fn set_volume_quantity_enabled(structure: &str, quantity: &str, enabled: bool) {
    with_volume_mesh(structure, |vm| {
        vm.get_quantity_mut(quantity)
            .expect("quantity not found")
            .set_enabled(enabled);
    });
}

/// Checks that vertex and cell vector quantities on a volume mesh are drawn.
fn check_volume_mesh_vectors() {
    remove_all_structures();
    // Two hex cells side by side.
    let mut vertices = unit_cube_vertices();
    vertices.extend(unit_cube_vertices().iter().map(|v| *v + Vec3::X));
    let vm = register_hex_mesh(
        "vm_vectors",
        vertices,
        vec![[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]],
    );
    vm.add_vertex_vector_quantity("vertex_vecs", vec![Vec3::new(0.3, 0.3, 0.0); 16]);
    vm.add_cell_vector_quantity("cell_vecs", vec![Vec3::new(0.0, 0.0, 1.0); 2]);

    let disabled = render();

    set_volume_quantity_enabled("vm_vectors", "vertex_vecs", true);
    let vertex_enabled = render();
    assert!(
        count_changed_pixels(&disabled, &vertex_enabled) > 0,
        "an enabled volume mesh vertex vector quantity must change the image"
    );
    let vertex_rerender = render();
    assert!(
        count_changed_pixels(&disabled, &vertex_rerender) > 0,
        "volume mesh vertex vectors must survive a headless re-render"
    );

    set_volume_quantity_enabled("vm_vectors", "vertex_vecs", false);
    set_volume_quantity_enabled("vm_vectors", "cell_vecs", true);
    let cell_enabled = render();
    assert!(
        count_changed_pixels(&disabled, &cell_enabled) > 0,
        "an enabled volume mesh cell vector quantity must change the image"
    );
    assert!(
        count_changed_pixels(&vertex_enabled, &cell_enabled) > 0,
        "vertex and cell vectors start at different points"
    );
}

/// Returns whether a pixel in a square window around the image center differs.
fn center_window_changed(a: &[u8], b: &[u8], half_size: u32) -> bool {
    let (cx, cy) = (WIDTH / 2, HEIGHT / 2);
    (cy - half_size..=cy + half_size).any(|y| {
        (cx - half_size..=cx + half_size).any(|x| {
            let i = ((y * WIDTH + x) * 4) as usize;
            a[i..i + 4] != b[i..i + 4]
        })
    })
}

/// Renders a scene without and with edges and returns whether the image center changed.
fn edges_change_center(name: &str, setup: impl Fn(&str, bool)) -> bool {
    remove_all_structures();
    setup(name, false);
    let plain = render();
    remove_all_structures();
    setup(name, true);
    let edges = render();
    center_window_changed(&plain, &edges, 3)
}

/// Registers a hex mesh of `count` unit cubes in a row along the x axis.
fn register_hex_row(name: &str, count: u32, show_edges: bool) {
    let vertices: Vec<Vec3> = (0..count)
        .flat_map(|i| {
            unit_cube_vertices()
                .into_iter()
                .map(move |v| v + Vec3::X * i as f32)
        })
        .collect();
    let cells: Vec<[u32; 8]> = (0..count)
        .map(|i| std::array::from_fn(|k| i * 8 + k as u32))
        .collect();
    let vm = register_hex_mesh(name, vertices, cells);
    vm.set_edge_width(if show_edges { 1.0 } else { 0.0 });
}

/// Registers a surface mesh of `count` unit quads in a row along the x axis.
fn register_quad_row(name: &str, count: u32, show_edges: bool) {
    let vertices: Vec<Vec3> = (0..=count)
        .flat_map(|i| [Vec3::new(i as f32, 0.0, 0.0), Vec3::new(i as f32, 1.0, 0.0)])
        .collect();
    let faces: Vec<Vec<u32>> = (0..count)
        .map(|i| vec![2 * i, 2 * i + 2, 2 * i + 3, 2 * i + 1])
        .collect();
    let mesh = register_surface_mesh(name, vertices, faces);
    mesh.set_show_edges(show_edges);
}

/// Checks that wireframes skip the triangulation diagonal of quad faces.
///
/// The camera looks at the front faces of a row of cells or quads. With two
/// elements, their shared real edge runs through the image center, so the
/// center must change when edges are on. With one element, only the diagonals
/// of its quad face cross the image center, so the center must not change.
fn check_quad_wireframes() {
    let hex_row = |name: &str, show_edges: bool| register_hex_row(name, 2, show_edges);
    let hex = |name: &str, show_edges: bool| register_hex_row(name, 1, show_edges);
    assert!(
        edges_change_center("hex_pair", hex_row),
        "the edge between two hex cells must be drawn"
    );
    assert!(
        !edges_change_center("hex_single", hex),
        "the diagonal of a hex face must not be drawn"
    );

    let quad_row = |name: &str, show_edges: bool| register_quad_row(name, 2, show_edges);
    let quad = |name: &str, show_edges: bool| register_quad_row(name, 1, show_edges);
    assert!(
        edges_change_center("quad_pair", quad_row),
        "the edge between two surface mesh quads must be drawn"
    );
    assert!(
        !edges_change_center("quad_single", quad),
        "the diagonal of a surface mesh quad must not be drawn"
    );
}

/// Checks that the material of a volume mesh changes its shading.
fn check_volume_mesh_material() {
    remove_all_structures();
    let vm = register_hex_mesh(
        "vm_material",
        unit_cube_vertices(),
        vec![[0, 1, 2, 3, 4, 5, 6, 7]],
    );
    let clay = render();
    vm.set_material("candy");
    let candy = render();
    assert!(
        count_changed_pixels(&clay, &candy) > 0,
        "a different volume mesh material must change the image"
    );
}

/// Checks that empty structures register and render without a panic.
///
/// wgpu rejects zero-size storage buffer bindings, so empty data must not
/// reach the GPU. Enabled vector quantities on empty data are included.
fn check_empty_structures() {
    let cases: [(&str, fn()); 5] = [
        ("a point cloud with 0 points", || {
            register_point_cloud("empty_pc", vec![]);
            with_point_cloud("empty_pc", |pc| {
                pc.add_vector_quantity("vecs", vec![]);
                pc.get_quantity_mut("vecs").unwrap().set_enabled(true);
            });
        }),
        ("a curve network with 0 edges", || {
            register_curve_network("no_edges", vec![Vec3::ZERO, Vec3::X], vec![]);
            with_curve_network("no_edges", |cn| {
                cn.add_edge_vector_quantity("vecs", vec![]);
                cn.get_quantity_mut("vecs").unwrap().set_enabled(true);
            });
        }),
        ("a curve network with 0 nodes", || {
            register_curve_network("no_nodes", vec![], vec![]);
        }),
        ("a surface mesh with 0 faces", || {
            register_surface_mesh(
                "no_faces",
                vec![Vec3::ZERO, Vec3::X, Vec3::Y],
                Vec::<Vec<u32>>::new(),
            );
        }),
        ("a volume mesh with 0 cells", || {
            let vm = register_volume_mesh("no_cells", vec![Vec3::ZERO, Vec3::X], vec![]);
            vm.add_cell_vector_quantity("vecs", vec![]);
            set_volume_quantity_enabled("no_cells", "vecs", true);
        }),
    ];
    for (label, register) in cases {
        remove_all_structures();
        register();
        let pixels = render_to_image(WIDTH, HEIGHT)
            .unwrap_or_else(|e| panic!("rendering {label} failed: {e}"));
        assert_eq!(pixels.len(), (WIDTH * HEIGHT * 4) as usize);
    }
}

/// Returns the RGB value of one pixel of an RGBA buffer.
fn pixel(image: &[u8], x: u32, y: u32) -> [i32; 3] {
    let i = ((y * WIDTH + x) * 4) as usize;
    [image[i], image[i + 1], image[i + 2]].map(i32::from)
}

/// Returns the largest channel difference between two pixels.
fn pixel_distance(a: [i32; 3], b: [i32; 3]) -> i32 {
    (0..3).map(|c| (a[c] - b[c]).abs()).max().unwrap_or(0)
}

/// Returns the image positions of the left and right cell of a hex pair.
///
/// The cells face the camera. The function scans the middle row of `image` for
/// pixels that differ from the background and splits that span into two halves.
fn hex_pair_cell_centers(image: &[u8]) -> [(u32, u32); 2] {
    let y = HEIGHT / 2;
    let background = pixel(image, 0, 0);
    let covered: Vec<u32> = (0..WIDTH)
        .filter(|&x| pixel(image, x, y) != background)
        .collect();
    let (first, last) = (covered[0], covered[covered.len() - 1]);
    let quarter = (last - first) / 4;
    [(first + quarter, y), (last - quarter, y)]
}

/// Checks that scalar and color quantities color a volume mesh.
fn check_volume_mesh_color_quantities() {
    remove_all_structures();
    register_hex_row("vm_colors", 2, false);
    let vm = get_volume_mesh("vm_colors").expect("volume mesh not registered");
    vm.add_vertex_scalar_quantity("vertex_scalar", (0..16).map(|i| i as f32).collect())
        .add_cell_scalar_quantity("cell_scalar", vec![0.0, 1.0])
        .add_vertex_color_quantity(
            "vertex_color",
            (0..16)
                .map(|i| Vec3::new(i as f32 / 15.0, 0.3, 1.0 - i as f32 / 15.0))
                .collect(),
        )
        .add_cell_color_quantity("cell_color", vec![Vec3::X, Vec3::Z]);

    let plain = render();
    for name in ["vertex_scalar", "cell_scalar", "vertex_color", "cell_color"] {
        vm.set_quantity_enabled(name, true);
        let colored = render();
        assert!(
            count_changed_pixels(&plain, &colored) > 0,
            "an enabled volume mesh {name} quantity must change the image"
        );
    }

    // Enabling the cell color quantity disabled the other color quantities.
    let enabled: Vec<bool> = with_volume_mesh_ref("vm_colors", |vm| {
        vm.quantities().iter().map(|q| q.is_enabled()).collect()
    })
    .unwrap();
    assert_eq!(enabled, [false, false, false, true]);

    // Removing the active quantity restores the base color.
    with_volume_mesh("vm_colors", |vm| vm.remove_quantity("cell_color"));
    assert_eq!(
        count_changed_pixels(&plain, &render()),
        0,
        "without an enabled quantity the mesh must use its base color"
    );
}

/// Checks that cell scalars 0 and 1 give the two end colors of the color map.
fn check_volume_mesh_cell_scalar_colors() {
    remove_all_structures();
    register_hex_row("vm_cells", 2, false);
    let vm = get_volume_mesh("vm_cells").expect("volume mesh not registered");
    let viridis = ColorMapRegistry::new()
        .get("viridis")
        .expect("viridis missing")
        .clone();
    vm.add_cell_scalar_quantity("t", vec![0.0, 1.0])
        .add_cell_color_quantity("ends", vec![viridis.sample(0.0), viridis.sample(1.0)]);

    vm.set_quantity_enabled("t", true);
    let scalar_image = render();
    let [left, right] = hex_pair_cell_centers(&scalar_image);
    let left_color = pixel(&scalar_image, left.0, left.1);
    let right_color = pixel(&scalar_image, right.0, right.1);

    // Viridis starts dark purple (blue above green) and ends yellow (blue low).
    assert!(
        pixel_distance(left_color, right_color) > 60,
        "the two cells must get clearly different colors: {left_color:?} {right_color:?}"
    );
    assert!(
        left_color[2] > left_color[1],
        "cell 0 must be purple: {left_color:?}"
    );
    assert!(
        right_color[0] > right_color[2] + 50 && right_color[1] > right_color[2] + 50,
        "cell 1 must be yellow: {right_color:?}"
    );

    // The same mesh colored directly with the two end colors looks the same.
    vm.set_quantity_enabled("ends", true);
    let ends_image = render();
    for (x, y) in [left, right] {
        assert!(pixel_distance(pixel(&scalar_image, x, y), pixel(&ends_image, x, y)) <= 2);
    }

    // A new color map and a new range change the colors at once.
    vm.set_quantity_enabled("t", true);
    with_volume_mesh("vm_cells", |vm| {
        let q = vm.get_quantity_mut("t").unwrap();
        let q = q.as_any_mut();
        let q = q
            .downcast_mut::<polyscope_structures::volume_mesh::VolumeMeshCellScalarQuantity>()
            .unwrap();
        q.set_color_map("coolwarm");
    });
    let coolwarm_image = render();
    assert!(pixel_distance(pixel(&coolwarm_image, left.0, left.1), left_color) > 20);
    with_volume_mesh("vm_cells", |vm| {
        let q = vm.get_quantity_mut("t").unwrap();
        let q = q.as_any_mut();
        let q = q
            .downcast_mut::<polyscope_structures::volume_mesh::VolumeMeshCellScalarQuantity>()
            .unwrap();
        q.set_color_map("viridis").set_data_range(0.0, 2.0);
    });
    let range_image = render();
    assert!(pixel_distance(pixel(&range_image, right.0, right.1), right_color) > 20);
    with_volume_mesh("vm_cells", |vm| {
        let q = vm.get_quantity_mut("t").unwrap();
        let q = q.as_any_mut();
        let q = q
            .downcast_mut::<polyscope_structures::volume_mesh::VolumeMeshCellScalarQuantity>()
            .unwrap();
        q.set_data_range(0.0, 1.0);
    });

    // After a transform the quantity still colors the mesh.
    with_volume_mesh("vm_cells", |vm| {
        vm.set_transform(Mat4::from_rotation_y(0.4))
    });
    let moved_colored = render();
    vm.set_quantity_enabled("t", false);
    let moved_plain = render();
    assert!(count_changed_pixels(&moved_colored, &moved_plain) > 0);
    vm.set_quantity_enabled("t", true);
    with_volume_mesh("vm_cells", |vm| vm.set_transform(Mat4::IDENTITY));

    // A slice plane culls the left cell. The right cell, now first in the
    // render data, keeps its color.
    add_slice_plane_with_pose("cull_left", Vec3::new(1.0, 0.0, 0.0), Vec3::X);
    let culled_image = render();
    remove_all_slice_planes();
    assert!(pixel_distance(pixel(&culled_image, right.0, right.1), right_color) <= 2);
    // The background replaces the culled cell.
    assert!(pixel_distance(pixel(&culled_image, left.0, left.1), left_color) > 20);
}

#[test]
fn structure_render_tests() {
    let _ = init();

    // Skip all checks if no GPU adapter is available.
    remove_all_structures();
    if let Err(e) = render_to_image(64, 64) {
        eprintln!("Skipping structure render tests: no GPU adapter available ({e})");
        return;
    }

    check_curve_network_vectors();
    check_volume_mesh_vectors();
    check_quad_wireframes();
    check_volume_mesh_material();
    check_empty_structures();
    check_volume_mesh_color_quantities();
    check_volume_mesh_cell_scalar_colors();

    remove_all_structures();
}
