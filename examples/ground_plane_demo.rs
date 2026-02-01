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
//! Uses real 3D models (Spot cow, Stanford Bunny) alongside procedural geometry.
//!
//! Run with: cargo run --example `ground_plane_demo`

use glam::Vec3;
use std::f32::consts::PI;

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

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Floating Spot cow (gold)
    let (mut spot_verts, spot_faces) = load_obj("assets/spot.obj");
    normalize_mesh(&mut spot_verts, 1.0);
    let spot_positioned = transform_vertices(&spot_verts, Vec3::new(0.0, 1.5, 0.0), 1.0);
    polyscope::register_surface_mesh("spot_cow", spot_positioned, spot_faces);
    polyscope::with_surface_mesh("spot_cow", |mesh| {
        mesh.set_surface_color(Vec3::new(0.9, 0.7, 0.2));
    });

    // Stanford Bunny on the ground (terracotta)
    let (mut bunny_verts, bunny_faces) = load_obj("assets/bunny.obj");
    normalize_mesh(&mut bunny_verts, 1.5);
    let bunny_positioned = transform_vertices(&bunny_verts, Vec3::new(1.5, 0.75, 0.0), 1.0);
    polyscope::register_surface_mesh("bunny", bunny_positioned, bunny_faces);
    polyscope::with_surface_mesh("bunny", |mesh| {
        mesh.set_surface_color(Vec3::new(0.8, 0.35, 0.25));
    });

    // Torus lying on the ground (blue)
    let (torus_verts, torus_faces) = create_torus(Vec3::new(-1.5, 0.25, 0.0), 0.5, 0.15, 24, 12);
    polyscope::register_surface_mesh("torus", torus_verts, torus_faces);
    polyscope::with_surface_mesh("torus", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.4, 0.9));
    });

    // Small floating sphere (green point cloud)
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
    println!("  - spot_cow: Floating gold Spot cow");
    println!("  - bunny: Terracotta Stanford Bunny");
    println!("  - torus: Blue ring on the ground");
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
