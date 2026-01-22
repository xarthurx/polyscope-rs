//! Surface mesh demonstration.
//!
//! Run with: cargo run --example surface_mesh_demo

use glam::Vec3;

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a simple box mesh
    let vertices = vec![
        // Front face
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        // Back face
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
    ];

    let faces = vec![
        glam::UVec3::new(0, 1, 2), // Front tri 1
        glam::UVec3::new(0, 2, 3), // Front tri 2
        glam::UVec3::new(5, 4, 7), // Back tri 1
        glam::UVec3::new(5, 7, 6), // Back tri 2
        glam::UVec3::new(4, 0, 3), // Left tri 1
        glam::UVec3::new(4, 3, 7), // Left tri 2
        glam::UVec3::new(1, 5, 6), // Right tri 1
        glam::UVec3::new(1, 6, 2), // Right tri 2
        glam::UVec3::new(3, 2, 6), // Top tri 1
        glam::UVec3::new(3, 6, 7), // Top tri 2
        glam::UVec3::new(4, 5, 1), // Bottom tri 1
        glam::UVec3::new(4, 1, 0), // Bottom tri 2
    ];

    let _mesh = polyscope::register_surface_mesh("box", vertices.clone(), faces);

    // Get handle and add quantities via with_mesh
    polyscope::with_surface_mesh("box", |mesh| {
        // Add vertex height scalar quantity
        let vertex_heights: Vec<f32> = vertices.iter().map(|v| v.y).collect();
        mesh.add_vertex_scalar_quantity("height", vertex_heights);

        // Add vertex colors
        let vertex_colors: Vec<Vec3> = vertices
            .iter()
            .map(|v| Vec3::new((v.x + 1.0) / 2.0, (v.y + 1.0) / 2.0, (v.z + 1.0) / 2.0))
            .collect();
        mesh.add_vertex_color_quantity("position_color", vertex_colors);

        // Enable wireframe
        mesh.set_show_edges(true);
        mesh.set_edge_width(2.0);
    });

    println!("Surface mesh demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
