//! Surface mesh demonstration.
//!
//! Run with: cargo run --example surface_mesh_demo

use glam::Vec3;
use std::f32::consts::PI;

/// Generate a UV sphere mesh.
fn generate_sphere(radius: f32, lat_segments: u32, lon_segments: u32) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    // Generate vertices
    for lat in 0..=lat_segments {
        let theta = PI * lat as f32 / lat_segments as f32; // 0 to PI
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = 2.0 * PI * lon as f32 / lon_segments as f32; // 0 to 2*PI
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            vertices.push(Vec3::new(x, y, z));
        }
    }

    // Generate faces (triangles)
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let first = lat * (lon_segments + 1) + lon;
            let second = first + lon_segments + 1;

            // Two triangles per quad (except at poles)
            if lat != 0 {
                faces.push(glam::UVec3::new(first, second, first + 1));
            }
            if lat != lat_segments - 1 {
                faces.push(glam::UVec3::new(second, second + 1, first + 1));
            }
        }
    }

    (vertices, faces)
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a sphere mesh
    let (vertices, faces) = generate_sphere(1.0, 32, 32);

    let _mesh = polyscope::register_surface_mesh("sphere", vertices.clone(), faces);

    // Get handle and add quantities via with_mesh
    polyscope::with_surface_mesh("sphere", |mesh| {
        // Add vertex height scalar quantity (Y coordinate)
        let vertex_heights: Vec<f32> = vertices.iter().map(|v| v.y).collect();
        mesh.add_vertex_scalar_quantity("height", vertex_heights);

        // Add vertex colors based on position (normalized to 0-1 range)
        let vertex_colors: Vec<Vec3> = vertices
            .iter()
            .map(|v| Vec3::new((v.x + 1.0) / 2.0, (v.y + 1.0) / 2.0, (v.z + 1.0) / 2.0))
            .collect();
        mesh.add_vertex_color_quantity("position_color", vertex_colors);

        // Set a nice surface color
        mesh.set_surface_color(Vec3::new(0.2, 0.5, 0.8));
    });

    println!("Surface mesh demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
