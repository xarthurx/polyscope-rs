//! Demo showing volume mesh visualization in polyscope-rs.

use polyscope::{self, Vec3};

fn main() {
    // Initialize polyscope
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a simple tetrahedron
    let tet_vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tets = vec![[0, 1, 2, 3]];
    let tet_mesh = polyscope::register_tet_mesh("single tet", tet_vertices, tets);
    tet_mesh.set_color(Vec3::new(0.8, 0.3, 0.3));

    // Create a stack of tetrahedra
    let mut vertices = Vec::new();
    let mut tets = Vec::new();
    for i in 0..3 {
        let offset = Vec3::new(2.0, 0.0, i as f32 * 1.0);
        let base_idx = (i * 4) as u32;
        vertices.push(Vec3::new(0.0, 0.0, 0.0) + offset);
        vertices.push(Vec3::new(1.0, 0.0, 0.0) + offset);
        vertices.push(Vec3::new(0.5, 1.0, 0.0) + offset);
        vertices.push(Vec3::new(0.5, 0.5, 0.8) + offset);
        tets.push([base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);
    }
    let tet_stack = polyscope::register_tet_mesh("tet stack", vertices, tets);
    tet_stack.set_color(Vec3::new(0.3, 0.6, 0.3));

    // Create a single hexahedron (cube)
    let hex_vertices = vec![
        // Bottom face
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(5.0, 1.0, 0.0),
        Vec3::new(4.0, 1.0, 0.0),
        // Top face
        Vec3::new(4.0, 0.0, 1.0),
        Vec3::new(5.0, 0.0, 1.0),
        Vec3::new(5.0, 1.0, 1.0),
        Vec3::new(4.0, 1.0, 1.0),
    ];
    let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
    let hex_mesh = polyscope::register_hex_mesh("single hex", hex_vertices, hexes);
    hex_mesh.set_color(Vec3::new(0.3, 0.3, 0.8));

    // Create a 2x2x2 grid of hexahedra
    let mut vertices = Vec::new();
    let mut hexes = Vec::new();

    // Generate vertices for 3x3x3 grid
    for k in 0..3 {
        for j in 0..3 {
            for i in 0..3 {
                vertices.push(Vec3::new(
                    6.5 + i as f32 * 0.5,
                    j as f32 * 0.5,
                    k as f32 * 0.5,
                ));
            }
        }
    }

    // Generate hexahedra cells
    for k in 0..2 {
        for j in 0..2 {
            for i in 0..2 {
                let v0 = (k * 9 + j * 3 + i) as u32;
                let v1 = v0 + 1;
                let v2 = v0 + 4;
                let v3 = v0 + 3;
                let v4 = v0 + 9;
                let v5 = v0 + 10;
                let v6 = v0 + 13;
                let v7 = v0 + 12;
                hexes.push([v0, v1, v2, v3, v4, v5, v6, v7]);
            }
        }
    }

    let hex_grid = polyscope::register_hex_mesh("hex grid", vertices, hexes);
    hex_grid.set_color(Vec3::new(0.6, 0.4, 0.8));

    println!("Volume mesh demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    // Show the viewer
    polyscope::show();
}
