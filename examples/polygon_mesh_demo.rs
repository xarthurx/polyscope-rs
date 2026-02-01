#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Polygon mesh demo showing support for arbitrary n-gon faces.
//!
//! This example registers meshes with quad, pentagon, and mixed polygon faces
//! to demonstrate that polyscope-rs handles arbitrary polygons via internal
//! fan triangulation while preserving original face structure for quantities
//! and wireframe display.
//!
//! Run with: `cargo run --example polygon_mesh_demo`
//!
//! Controls:
//! - Left drag: Orbit camera
//! - Right drag: Pan camera
//! - Scroll: Zoom
//! - ESC: Exit

use std::f32::consts::PI;

use glam::Vec3;

/// Generate a cube with quad faces (6 quads, no triangulation).
fn create_quad_cube() -> (Vec<Vec3>, Vec<Vec<u32>>) {
    let vertices = vec![
        Vec3::new(-0.5, -0.5, 0.5),  // 0: front-bottom-left
        Vec3::new(0.5, -0.5, 0.5),   // 1: front-bottom-right
        Vec3::new(0.5, 0.5, 0.5),    // 2: front-top-right
        Vec3::new(-0.5, 0.5, 0.5),   // 3: front-top-left
        Vec3::new(-0.5, -0.5, -0.5), // 4: back-bottom-left
        Vec3::new(0.5, -0.5, -0.5),  // 5: back-bottom-right
        Vec3::new(0.5, 0.5, -0.5),   // 6: back-top-right
        Vec3::new(-0.5, 0.5, -0.5),  // 7: back-top-left
    ];

    let faces: Vec<Vec<u32>> = vec![
        vec![0, 1, 2, 3],    // front
        vec![5, 4, 7, 6],    // back
        vec![3, 2, 6, 7],    // top
        vec![4, 5, 1, 0],    // bottom
        vec![1, 5, 6, 2],    // right
        vec![4, 0, 3, 7],    // left
    ];

    (vertices, faces)
}

/// Generate a regular polygon prism (top and bottom are n-gons, sides are quads).
fn create_polygon_prism(n: usize, height: f32, radius: f32) -> (Vec<Vec3>, Vec<Vec<u32>>) {
    let mut vertices = Vec::with_capacity(n * 2);
    let half_h = height / 2.0;

    // Bottom ring
    for i in 0..n {
        let angle = 2.0 * PI * (i as f32) / (n as f32);
        vertices.push(Vec3::new(radius * angle.cos(), -half_h, radius * angle.sin()));
    }
    // Top ring
    for i in 0..n {
        let angle = 2.0 * PI * (i as f32) / (n as f32);
        vertices.push(Vec3::new(radius * angle.cos(), half_h, radius * angle.sin()));
    }

    let mut faces = Vec::new();

    // Bottom face (n-gon, wound clockwise when viewed from below)
    let bottom: Vec<u32> = (0..n as u32).rev().collect();
    faces.push(bottom);

    // Top face (n-gon)
    let top: Vec<u32> = (n as u32..2 * n as u32).collect();
    faces.push(top);

    // Side quads
    for i in 0..n {
        let next = (i + 1) % n;
        faces.push(vec![
            i as u32,
            next as u32,
            (next + n) as u32,
            (i + n) as u32,
        ]);
    }

    (vertices, faces)
}

/// Generate a mixed-polygon mesh: a truncated pyramid with a pentagon top,
/// pentagon bottom, and trapezoidal (quad) sides.
fn create_truncated_pyramid() -> (Vec<Vec3>, Vec<Vec<u32>>) {
    let n = 5;
    let mut vertices = Vec::new();

    // Bottom pentagon (larger)
    for i in 0..n {
        let angle = 2.0 * PI * (i as f32) / (n as f32) - PI / 2.0;
        vertices.push(Vec3::new(1.0 * angle.cos(), -0.5, 1.0 * angle.sin()));
    }
    // Top pentagon (smaller)
    for i in 0..n {
        let angle = 2.0 * PI * (i as f32) / (n as f32) - PI / 2.0;
        vertices.push(Vec3::new(0.5 * angle.cos(), 0.5, 0.5 * angle.sin()));
    }

    let mut faces = Vec::new();

    // Bottom face (pentagon, reversed winding)
    let bottom: Vec<u32> = (0..n as u32).rev().collect();
    faces.push(bottom);

    // Top face (pentagon)
    let top: Vec<u32> = (n as u32..2 * n as u32).collect();
    faces.push(top);

    // Side quads
    for i in 0..n {
        let next = (i + 1) % n;
        faces.push(vec![
            i as u32,
            next as u32,
            (next + n) as u32,
            (i + n) as u32,
        ]);
    }

    (vertices, faces)
}

/// Translate all vertices by an offset.
fn translate(vertices: &[Vec3], offset: Vec3) -> Vec<Vec3> {
    vertices.iter().map(|v| *v + offset).collect()
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // 1. Quad cube — all faces are quads (4-gons)
    let (verts, faces) = create_quad_cube();
    let verts = translate(&verts, Vec3::new(-2.5, 0.0, 0.0));
    polyscope::register_surface_mesh("quad_cube", verts, faces.clone());
    polyscope::with_surface_mesh("quad_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.6, 1.0));
        mesh.set_show_edges(true);
    });
    // Face scalar: one value per original quad face
    let face_scalars: Vec<f32> = (0..faces.len()).map(|i| i as f32 / faces.len() as f32).collect();
    polyscope::with_surface_mesh("quad_cube", |mesh| {
        mesh.add_face_scalar_quantity("face_id", face_scalars);
    });

    // 2. Hexagonal prism — top and bottom are hexagons, sides are quads
    let (verts, faces) = create_polygon_prism(6, 1.0, 0.8);
    let verts = translate(&verts, Vec3::new(0.0, 0.0, 0.0));
    polyscope::register_surface_mesh("hex_prism", verts, faces.clone());
    polyscope::with_surface_mesh("hex_prism", |mesh| {
        mesh.set_surface_color(Vec3::new(1.0, 0.6, 0.2));
        mesh.set_show_edges(true);
    });
    let face_scalars: Vec<f32> = (0..faces.len()).map(|i| i as f32 / faces.len() as f32).collect();
    polyscope::with_surface_mesh("hex_prism", |mesh| {
        mesh.add_face_scalar_quantity("face_id", face_scalars);
    });

    // 3. Octagonal prism — top and bottom are octagons
    let (verts, faces) = create_polygon_prism(8, 0.8, 0.7);
    let verts = translate(&verts, Vec3::new(2.5, 0.0, 0.0));
    polyscope::register_surface_mesh("oct_prism", verts, faces.clone());
    polyscope::with_surface_mesh("oct_prism", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 1.0, 0.4));
        mesh.set_show_edges(true);
    });
    let face_scalars: Vec<f32> = (0..faces.len()).map(|i| i as f32 / faces.len() as f32).collect();
    polyscope::with_surface_mesh("oct_prism", |mesh| {
        mesh.add_face_scalar_quantity("face_id", face_scalars);
    });

    // 4. Truncated pyramid — pentagons + quads mixed
    let (verts, faces) = create_truncated_pyramid();
    let verts = translate(&verts, Vec3::new(0.0, 0.0, 2.5));
    polyscope::register_surface_mesh("truncated_pyramid", verts, faces.clone());
    polyscope::with_surface_mesh("truncated_pyramid", |mesh| {
        mesh.set_surface_color(Vec3::new(0.9, 0.3, 0.7));
        mesh.set_show_edges(true);
    });
    let face_scalars: Vec<f32> = (0..faces.len()).map(|i| i as f32 / faces.len() as f32).collect();
    polyscope::with_surface_mesh("truncated_pyramid", |mesh| {
        mesh.add_face_scalar_quantity("face_id", face_scalars);
    });

    println!("Polygon Mesh Demo");
    println!("=================");
    println!();
    println!("This demo shows polygon mesh support (quads, hexagons, octagons, etc.).");
    println!("All faces are registered as arbitrary polygons (Vec<Vec<u32>>),");
    println!("fan-triangulated internally for rendering.");
    println!();
    println!("Meshes shown:");
    println!("  - quad_cube: Cube with 6 quad faces (left)");
    println!("  - hex_prism: Hexagonal prism with 6-gon caps (center)");
    println!("  - oct_prism: Octagonal prism with 8-gon caps (right)");
    println!("  - truncated_pyramid: Pentagon caps + quad sides (back)");
    println!();
    println!("Note: Wireframe shows original polygon edges, not internal");
    println!("triangulation edges. Face scalar quantities use one value per");
    println!("original polygon face.");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
