//! Transparency demo demonstrating order-independent transparency rendering.
//!
//! This example creates overlapping transparent meshes to show how the
//! Weighted Blended OIT algorithm renders transparent surfaces correctly
//! regardless of render order.
//!
//! Run with: cargo run --example transparency_demo
//!
//! Controls:
//! - Change transparency mode in Appearance settings (WeightedBlended recommended)
//! - Adjust mesh transparency via the Opacity slider in each mesh's settings

use glam::Vec3;

/// Generate a unit cube mesh centered at origin.
fn create_cube() -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let vertices = vec![
        // Front face
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
        // Back face
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
    ];

    let faces = vec![
        // Front
        glam::UVec3::new(0, 1, 2),
        glam::UVec3::new(0, 2, 3),
        // Back
        glam::UVec3::new(5, 4, 7),
        glam::UVec3::new(5, 7, 6),
        // Top
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 7),
        // Bottom
        glam::UVec3::new(4, 5, 1),
        glam::UVec3::new(4, 1, 0),
        // Right
        glam::UVec3::new(1, 5, 6),
        glam::UVec3::new(1, 6, 2),
        // Left
        glam::UVec3::new(4, 0, 3),
        glam::UVec3::new(4, 3, 7),
    ];

    (vertices, faces)
}

/// Transform vertices by translating and scaling.
fn transform_vertices(vertices: &[Vec3], translation: Vec3, scale: f32) -> Vec<Vec3> {
    vertices
        .iter()
        .map(|v| *v * scale + translation)
        .collect()
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create base cube geometry
    let (cube_verts, cube_faces) = create_cube();

    // Create three overlapping transparent cubes with different colors
    // Positioned to overlap so we can see the OIT effect

    // Red cube (center-left)
    let red_verts = transform_vertices(&cube_verts, Vec3::new(-0.3, 0.0, 0.0), 1.0);
    polyscope::register_surface_mesh("red_cube", red_verts, cube_faces.clone());
    polyscope::with_surface_mesh("red_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(1.0, 0.2, 0.2));
        mesh.set_transparency(0.5); // 50% transparent
    });

    // Green cube (center)
    let green_verts = transform_vertices(&cube_verts, Vec3::new(0.0, 0.0, 0.0), 1.0);
    polyscope::register_surface_mesh("green_cube", green_verts, cube_faces.clone());
    polyscope::with_surface_mesh("green_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 1.0, 0.2));
        mesh.set_transparency(0.5); // 50% transparent
    });

    // Blue cube (center-right)
    let blue_verts = transform_vertices(&cube_verts, Vec3::new(0.3, 0.0, 0.0), 1.0);
    polyscope::register_surface_mesh("blue_cube", blue_verts, cube_faces.clone());
    polyscope::with_surface_mesh("blue_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.2, 1.0));
        mesh.set_transparency(0.5); // 50% transparent
    });

    // Also add an opaque reference cube behind the others
    let gray_verts = transform_vertices(&cube_verts, Vec3::new(0.0, 0.0, -1.0), 1.5);
    polyscope::register_surface_mesh("opaque_cube", gray_verts, cube_faces);
    polyscope::with_surface_mesh("opaque_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.5, 0.5, 0.5));
        // Keep opaque (transparency = 0.0)
    });

    println!("Transparency Demo");
    println!("==================");
    println!();
    println!("This demo shows order-independent transparency (OIT) rendering.");
    println!("Three overlapping transparent cubes (red, green, blue) are rendered");
    println!("in front of a gray opaque cube.");
    println!();
    println!("To see the difference between transparency modes:");
    println!("  1. Open the Appearance settings in the left panel");
    println!("  2. Change 'Transparency' from 'Simple' to 'Weighted Blended'");
    println!("  3. Notice how the overlapping transparent surfaces blend correctly");
    println!();
    println!("Try adjusting individual mesh transparency:");
    println!("  - Expand each mesh in the Structures panel");
    println!("  - Use the 'Opacity' slider (0 = transparent, 1 = opaque)");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
