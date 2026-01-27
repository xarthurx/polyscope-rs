#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Surface mesh demonstration using the Stanford Bunny.
//!
//! Run with: cargo run --example `surface_mesh_demo`

use glam::Vec3;

/// Load an OBJ file and return vertices and triangle faces.
fn load_obj(path: &str) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    // Combine all meshes (bunny.obj typically has one mesh)
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    for model in models {
        let mesh = model.mesh;
        let vertex_offset = vertices.len() as u32;

        // Extract vertices (positions come in groups of 3: x, y, z)
        for i in (0..mesh.positions.len()).step_by(3) {
            vertices.push(Vec3::new(
                mesh.positions[i],
                mesh.positions[i + 1],
                mesh.positions[i + 2],
            ));
        }

        // Extract faces (indices come in groups of 3 for triangles)
        for i in (0..mesh.indices.len()).step_by(3) {
            faces.push(glam::UVec3::new(
                mesh.indices[i] + vertex_offset,
                mesh.indices[i + 1] + vertex_offset,
                mesh.indices[i + 2] + vertex_offset,
            ));
        }
    }

    (vertices, faces)
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Load the Stanford Bunny
    let (vertices, faces) = load_obj("assets/bunny.obj");

    println!(
        "Loaded bunny: {} vertices, {} faces",
        vertices.len(),
        faces.len()
    );

    let _mesh = polyscope::register_surface_mesh("bunny", vertices.clone(), faces);

    // Get handle and add quantities via with_mesh
    polyscope::with_surface_mesh("bunny", |mesh| {
        // Add vertex height scalar quantity (Y coordinate)
        let vertex_heights: Vec<f32> = vertices.iter().map(|v| v.y).collect();
        mesh.add_vertex_scalar_quantity("height", vertex_heights);

        // Add vertex colors based on position
        let y_min = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let y_max = vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let vertex_colors: Vec<Vec3> = vertices
            .iter()
            .map(|v| {
                let t = (v.y - y_min) / (y_max - y_min);
                Vec3::new(t, 0.5, 1.0 - t)
            })
            .collect();
        mesh.add_vertex_color_quantity("height_color", vertex_colors);

        // Set a nice surface color
        mesh.set_surface_color(Vec3::new(0.8, 0.6, 0.4));
    });

    println!("Surface mesh demo running...");
    println!("Displaying the Stanford Bunny");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
