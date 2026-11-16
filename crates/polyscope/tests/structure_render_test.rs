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

    remove_all_structures();
}
