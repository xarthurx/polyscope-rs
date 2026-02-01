#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Transparency demo demonstrating depth-peeled transparency rendering.
//!
//! This example loads real 3D models (Spot cow, Utah Teapot, Stanford Bunny)
//! and renders them with varying transparency to show how the depth peeling
//! algorithm handles overlapping transparent surfaces correctly.
//!
//! Run with: cargo run --example `transparency_demo`
//!
//! Controls:
//! - Change transparency mode in Appearance settings (`Pretty` recommended)
//! - Adjust mesh transparency via the Opacity slider in each mesh's settings

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

    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    for model in models {
        let mesh = model.mesh;
        let vertex_offset = vertices.len() as u32;

        for i in (0..mesh.positions.len()).step_by(3) {
            vertices.push(Vec3::new(
                mesh.positions[i],
                mesh.positions[i + 1],
                mesh.positions[i + 2],
            ));
        }

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

/// Normalize a mesh to be centered at origin with a given target size.
fn normalize_mesh(vertices: &mut [Vec3], target_size: f32) {
    if vertices.is_empty() {
        return;
    }
    let min = vertices.iter().copied().reduce(Vec3::min).unwrap();
    let max = vertices.iter().copied().reduce(Vec3::max).unwrap();
    let center = (min + max) * 0.5;
    let extent = max - min;
    let max_extent = extent.x.max(extent.y).max(extent.z);
    let scale = target_size / max_extent;
    for v in vertices.iter_mut() {
        *v = (*v - center) * scale;
    }
}

/// Transform vertices by translating and scaling.
fn transform_vertices(vertices: &[Vec3], translation: Vec3, scale: f32) -> Vec<Vec3> {
    vertices.iter().map(|v| *v * scale + translation).collect()
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Load and normalize all three models to a common size
    let (mut spot_verts, spot_faces) = load_obj("assets/spot.obj");
    normalize_mesh(&mut spot_verts, 1.5);

    let (mut teapot_verts, teapot_faces) = load_obj("assets/teapot.obj");
    normalize_mesh(&mut teapot_verts, 1.5);

    let (mut bunny_verts, bunny_faces) = load_obj("assets/bunny.obj");
    normalize_mesh(&mut bunny_verts, 1.5);

    // Position the three transparent models with slight overlap
    let spot_positioned = transform_vertices(&spot_verts, Vec3::new(-0.4, 0.0, 0.0), 1.0);
    polyscope::register_surface_mesh("spot_cow", spot_positioned, spot_faces.clone());
    polyscope::with_surface_mesh("spot_cow", |mesh| {
        mesh.set_surface_color(Vec3::new(1.0, 0.6, 0.2)); // warm orange
        mesh.set_transparency(0.5);
    });

    let teapot_positioned = transform_vertices(&teapot_verts, Vec3::new(0.0, 0.0, 0.2), 1.0);
    polyscope::register_surface_mesh("teapot", teapot_positioned, teapot_faces.clone());
    polyscope::with_surface_mesh("teapot", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.5, 1.0)); // cool blue
        mesh.set_transparency(0.5);
    });

    let bunny_positioned = transform_vertices(&bunny_verts, Vec3::new(0.4, 0.0, -0.2), 1.0);
    polyscope::register_surface_mesh("bunny", bunny_positioned, bunny_faces.clone());
    polyscope::with_surface_mesh("bunny", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.9, 0.3)); // green
        mesh.set_transparency(0.5);
    });

    // Opaque reference mesh behind the transparent ones
    let spot_opaque = transform_vertices(&spot_verts, Vec3::new(0.0, 0.0, -1.5), 1.8);
    polyscope::register_surface_mesh("opaque_spot", spot_opaque, spot_faces);
    polyscope::with_surface_mesh("opaque_spot", |mesh| {
        mesh.set_surface_color(Vec3::new(0.5, 0.5, 0.5));
    });

    println!("Transparency Demo");
    println!("==================");
    println!();
    println!("This demo shows depth-peeled transparency (Pretty mode).");
    println!("Three overlapping transparent models (Spot, Teapot, Bunny) are rendered");
    println!("in front of an opaque gray Spot cow.");
    println!();
    println!("To see the difference between transparency modes:");
    println!("  1. Open the Appearance settings in the left panel");
    println!("  2. Change 'Transparency' from 'Simple' to 'Pretty'");
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
