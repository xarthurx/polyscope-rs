#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Ground Plane and Shadows demonstration.
//!
//! This demo shows:
//! - Ground plane rendering with different modes
//! - Shadow casting onto the ground plane
//! - Ground plane reflections
//! - Scene presentation settings
//!
//! Run with: cargo run --example `ground_plane_demo`

use glam::Vec3;
use std::f32::consts::PI;

/// Generate a simple icosahedron mesh.
fn create_icosahedron(center: Vec3, radius: f32) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let a = radius / (1.0 + phi * phi).sqrt();
    let b = a * phi;

    let vertices = vec![
        center + Vec3::new(-a, b, 0.0),
        center + Vec3::new(a, b, 0.0),
        center + Vec3::new(-a, -b, 0.0),
        center + Vec3::new(a, -b, 0.0),
        center + Vec3::new(0.0, -a, b),
        center + Vec3::new(0.0, a, b),
        center + Vec3::new(0.0, -a, -b),
        center + Vec3::new(0.0, a, -b),
        center + Vec3::new(b, 0.0, -a),
        center + Vec3::new(b, 0.0, a),
        center + Vec3::new(-b, 0.0, -a),
        center + Vec3::new(-b, 0.0, a),
    ];

    let faces = vec![
        glam::UVec3::new(0, 11, 5),
        glam::UVec3::new(0, 5, 1),
        glam::UVec3::new(0, 1, 7),
        glam::UVec3::new(0, 7, 10),
        glam::UVec3::new(0, 10, 11),
        glam::UVec3::new(1, 5, 9),
        glam::UVec3::new(5, 11, 4),
        glam::UVec3::new(11, 10, 2),
        glam::UVec3::new(10, 7, 6),
        glam::UVec3::new(7, 1, 8),
        glam::UVec3::new(3, 9, 4),
        glam::UVec3::new(3, 4, 2),
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 8),
        glam::UVec3::new(3, 8, 9),
        glam::UVec3::new(4, 9, 5),
        glam::UVec3::new(2, 4, 11),
        glam::UVec3::new(6, 2, 10),
        glam::UVec3::new(8, 6, 7),
        glam::UVec3::new(9, 8, 1),
    ];

    (vertices, faces)
}

/// Generate a torus mesh by creating a grid of vertices.
fn create_torus(
    center: Vec3,
    major_radius: f32,
    minor_radius: f32,
    major_segments: usize,
    minor_segments: usize,
) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    // Generate vertices
    for i in 0..major_segments {
        let theta = 2.0 * PI * i as f32 / major_segments as f32;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        for j in 0..minor_segments {
            let phi = 2.0 * PI * j as f32 / minor_segments as f32;
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let x = (major_radius + minor_radius * cos_phi) * cos_theta;
            let y = minor_radius * sin_phi;
            let z = (major_radius + minor_radius * cos_phi) * sin_theta;

            vertices.push(center + Vec3::new(x, y, z));
        }
    }

    // Generate faces
    for i in 0..major_segments {
        let next_i = (i + 1) % major_segments;
        for j in 0..minor_segments {
            let next_j = (j + 1) % minor_segments;

            let v0 = (i * minor_segments + j) as u32;
            let v1 = (next_i * minor_segments + j) as u32;
            let v2 = (next_i * minor_segments + next_j) as u32;
            let v3 = (i * minor_segments + next_j) as u32;

            faces.push(glam::UVec3::new(v0, v1, v2));
            faces.push(glam::UVec3::new(v0, v2, v3));
        }
    }

    (vertices, faces)
}

/// Generate a simple box mesh.
fn create_box(center: Vec3, size: Vec3) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let half = size / 2.0;
    let vertices = vec![
        center + Vec3::new(-half.x, -half.y, -half.z),
        center + Vec3::new(half.x, -half.y, -half.z),
        center + Vec3::new(half.x, half.y, -half.z),
        center + Vec3::new(-half.x, half.y, -half.z),
        center + Vec3::new(-half.x, -half.y, half.z),
        center + Vec3::new(half.x, -half.y, half.z),
        center + Vec3::new(half.x, half.y, half.z),
        center + Vec3::new(-half.x, half.y, half.z),
    ];

    let faces = vec![
        glam::UVec3::new(0, 1, 2),
        glam::UVec3::new(0, 2, 3),
        glam::UVec3::new(5, 4, 7),
        glam::UVec3::new(5, 7, 6),
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 7),
        glam::UVec3::new(4, 5, 1),
        glam::UVec3::new(4, 1, 0),
        glam::UVec3::new(1, 5, 6),
        glam::UVec3::new(1, 6, 2),
        glam::UVec3::new(4, 0, 3),
        glam::UVec3::new(4, 3, 7),
    ];

    (vertices, faces)
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create several objects at different heights to show shadows

    // Floating icosahedron (gold)
    let (ico_verts, ico_faces) = create_icosahedron(Vec3::new(0.0, 1.5, 0.0), 0.5);
    polyscope::register_surface_mesh("icosahedron", ico_verts, ico_faces);
    polyscope::with_surface_mesh("icosahedron", |mesh| {
        mesh.set_surface_color(Vec3::new(0.9, 0.7, 0.2));
    });

    // Torus lying on the ground (blue)
    let (torus_verts, torus_faces) = create_torus(Vec3::new(-1.5, 0.25, 0.0), 0.5, 0.15, 24, 12);
    polyscope::register_surface_mesh("torus", torus_verts, torus_faces);
    polyscope::with_surface_mesh("torus", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.4, 0.9));
    });

    // Tall box (red)
    let (box_verts, box_faces) = create_box(Vec3::new(1.5, 0.75, 0.0), Vec3::new(0.5, 1.5, 0.5));
    polyscope::register_surface_mesh("tall_box", box_verts, box_faces);
    polyscope::with_surface_mesh("tall_box", |mesh| {
        mesh.set_surface_color(Vec3::new(0.9, 0.2, 0.2));
    });

    // Small floating sphere (green)
    let sphere_pts: Vec<Vec3> = (0..500)
        .map(|i| {
            let phi = PI * (3.0 - 5.0_f32.sqrt()) * i as f32;
            let y = 1.0 - (i as f32 / 499.0) * 2.0;
            let r = (1.0 - y * y).sqrt();
            Vec3::new(0.0, 2.5, -1.2) + Vec3::new(r * phi.cos(), y, r * phi.sin()) * 0.3
        })
        .collect();
    let sphere = polyscope::register_point_cloud("floating_sphere", sphere_pts);
    sphere.add_color_quantity(
        "green",
        (0..500).map(|_| Vec3::new(0.2, 0.9, 0.3)).collect(),
    );

    // A curve network (helix) - also casts shadow
    let helix_pts: Vec<Vec3> = (0..100)
        .map(|i| {
            let t = i as f32 / 99.0;
            let angle = 4.0 * PI * t;
            Vec3::new(
                -1.5 + 0.3 * angle.cos(),
                1.0 + t * 1.5,
                1.2 + 0.3 * angle.sin(),
            )
        })
        .collect();
    polyscope::register_curve_network_line("helix", helix_pts);
    polyscope::with_curve_network("helix", |cn| {
        cn.set_color(Vec3::new(0.9, 0.5, 0.1));
        cn.set_radius(0.03, true);
    });

    println!("Ground Plane and Shadows Demo");
    println!("=============================");
    println!();
    println!("This demo showcases ground plane rendering with shadows.");
    println!();
    println!("Structures:");
    println!("  - icosahedron: Floating gold shape");
    println!("  - torus: Blue ring on the ground");
    println!("  - tall_box: Red vertical box");
    println!("  - floating_sphere: Green point cloud");
    println!("  - helix: Orange spiral curve");
    println!();
    println!("Ground Plane Controls (in Appearance panel):");
    println!("  - Mode: None / Tile / Shadow Only / Tile + Shadow");
    println!("  - Height: Y position of the ground plane");
    println!("  - Tile Color: Color of the ground tiles");
    println!("  - Shadow Darkness: Intensity of shadows");
    println!("  - Reflection: Enable mirror-like reflections");
    println!();
    println!("Note: Shadows are cast from a default light position.");
    println!("Move structures with gizmos to see shadow changes.");
    println!();
    println!("Camera Controls:");
    println!("  - Left drag: Orbit");
    println!("  - Right drag: Pan");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
