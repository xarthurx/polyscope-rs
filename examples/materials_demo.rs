#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Materials demo showcasing all 8 matcap materials.
//!
//! This example creates a grid of spherical meshes (icospheres), each assigned
//! a different material, plus a point cloud and curve network to show materials
//! across structure types.
//!
//! Run with: `cargo run --example materials_demo`
//!
//! Controls:
//! - Change each structure's material via the "Material" dropdown in its panel
//! - Left drag: Orbit camera
//! - Right drag: Pan camera
//! - Scroll: Zoom

use glam::{UVec3, Vec3};
use polyscope_rs::Structure;

/// All available matcap material names.
const MATERIALS: &[&str] = &[
    "clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal",
];

/// Generate an icosphere mesh (subdivision of icosahedron).
fn create_icosphere(subdivisions: u32) -> (Vec<Vec3>, Vec<UVec3>) {
    use std::collections::HashMap;

    // Start with icosahedron vertices
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let mut vertices: Vec<Vec3> = vec![
        Vec3::new(-1.0, t, 0.0),
        Vec3::new(1.0, t, 0.0),
        Vec3::new(-1.0, -t, 0.0),
        Vec3::new(1.0, -t, 0.0),
        Vec3::new(0.0, -1.0, t),
        Vec3::new(0.0, 1.0, t),
        Vec3::new(0.0, -1.0, -t),
        Vec3::new(0.0, 1.0, -t),
        Vec3::new(t, 0.0, -1.0),
        Vec3::new(t, 0.0, 1.0),
        Vec3::new(-t, 0.0, -1.0),
        Vec3::new(-t, 0.0, 1.0),
    ];
    // Normalize to unit sphere
    for v in &mut vertices {
        *v = v.normalize();
    }

    let mut faces: Vec<UVec3> = vec![
        UVec3::new(0, 11, 5),
        UVec3::new(0, 5, 1),
        UVec3::new(0, 1, 7),
        UVec3::new(0, 7, 10),
        UVec3::new(0, 10, 11),
        UVec3::new(1, 5, 9),
        UVec3::new(5, 11, 4),
        UVec3::new(11, 10, 2),
        UVec3::new(10, 7, 6),
        UVec3::new(7, 1, 8),
        UVec3::new(3, 9, 4),
        UVec3::new(3, 4, 2),
        UVec3::new(3, 2, 6),
        UVec3::new(3, 6, 8),
        UVec3::new(3, 8, 9),
        UVec3::new(4, 9, 5),
        UVec3::new(2, 4, 11),
        UVec3::new(6, 2, 10),
        UVec3::new(8, 6, 7),
        UVec3::new(9, 8, 1),
    ];

    // Subdivide
    let mut midpoint_cache: HashMap<(u32, u32), u32> = HashMap::new();

    for _ in 0..subdivisions {
        let mut new_faces = Vec::new();
        midpoint_cache.clear();

        for face in &faces {
            let a = get_midpoint(face.x, face.y, &mut vertices, &mut midpoint_cache);
            let b = get_midpoint(face.y, face.z, &mut vertices, &mut midpoint_cache);
            let c = get_midpoint(face.z, face.x, &mut vertices, &mut midpoint_cache);

            new_faces.push(UVec3::new(face.x, a, c));
            new_faces.push(UVec3::new(face.y, b, a));
            new_faces.push(UVec3::new(face.z, c, b));
            new_faces.push(UVec3::new(a, b, c));
        }

        faces = new_faces;
    }

    (vertices, faces)
}

fn get_midpoint(
    i0: u32,
    i1: u32,
    vertices: &mut Vec<Vec3>,
    cache: &mut std::collections::HashMap<(u32, u32), u32>,
) -> u32 {
    let key = if i0 < i1 { (i0, i1) } else { (i1, i0) };
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }
    let mid = (vertices[i0 as usize] + vertices[i1 as usize]).normalize();
    let idx = vertices.len() as u32;
    vertices.push(mid);
    cache.insert(key, idx);
    idx
}

/// Transform vertices by translating and scaling.
fn transform_vertices(vertices: &[Vec3], translation: Vec3, scale: f32) -> Vec<Vec3> {
    vertices.iter().map(|v| *v * scale + translation).collect()
}

fn main() {
    env_logger::init();
    polyscope_rs::init().expect("Failed to initialize polyscope");

    // Generate a base icosphere with 2 subdivisions (320 faces - smooth enough)
    let (sphere_verts, sphere_faces) = create_icosphere(2);

    // Colors for each material (chosen to look distinct)
    let colors: &[Vec3] = &[
        Vec3::new(0.55, 0.55, 0.55), // clay - neutral gray
        Vec3::new(0.85, 0.75, 0.55), // wax - warm tan
        Vec3::new(0.90, 0.40, 0.50), // candy - pink
        Vec3::new(0.40, 0.60, 0.85), // flat - blue
        Vec3::new(0.50, 0.35, 0.25), // mud - brown
        Vec3::new(0.80, 0.85, 0.90), // ceramic - white-blue
        Vec3::new(0.25, 0.70, 0.45), // jade - green
        Vec3::new(0.60, 0.50, 0.80), // normal - purple
    ];

    // Create a 4x2 grid of spheres, each with a different material
    let spacing = 2.5_f32;
    for (i, &material) in MATERIALS.iter().enumerate() {
        let col = (i % 4) as f32;
        let row = (i / 4) as f32;
        let pos = Vec3::new(col * spacing - 1.5 * spacing, -row * spacing, 0.0);

        let name = format!("sphere_{material}");
        let verts = transform_vertices(&sphere_verts, pos, 1.0);
        polyscope_rs::register_surface_mesh(&name, verts, sphere_faces.clone());
        polyscope_rs::with_surface_mesh(&name, |mesh| {
            mesh.set_surface_color(colors[i]);
            mesh.set_material(material);
        });
    }

    // Add a point cloud with a non-default material
    let num_pts = 500;
    let mut pc_points = Vec::with_capacity(num_pts);
    for i in 0..num_pts {
        let t = i as f32 / num_pts as f32;
        let theta = t * std::f32::consts::TAU * 5.0;
        let r = 0.8 + 0.3 * (t * 10.0).sin();
        pc_points.push(Vec3::new(
            r * theta.cos() + 12.0,
            r * theta.sin(),
            (t - 0.5) * 4.0,
        ));
    }
    polyscope_rs::register_point_cloud("point_cloud_wax", pc_points);
    polyscope_rs::with_point_cloud("point_cloud_wax", |pc| {
        pc.set_base_color(Vec3::new(0.9, 0.7, 0.3));
        pc.set_point_radius(0.03);
        pc.set_material("wax");
    });

    // Add a curve network with ceramic material
    let num_nodes = 100;
    let mut cn_nodes = Vec::with_capacity(num_nodes);
    for i in 0..num_nodes {
        let t = i as f32 / num_nodes as f32;
        let theta = t * std::f32::consts::TAU * 3.0;
        cn_nodes.push(Vec3::new(
            1.5 * theta.cos() + 12.0,
            1.5 * theta.sin() - 2.5,
            (t - 0.5) * 4.0,
        ));
    }
    polyscope_rs::register_curve_network_line("curve_ceramic", cn_nodes);
    polyscope_rs::with_curve_network("curve_ceramic", |cn| {
        cn.set_color(Vec3::new(0.7, 0.8, 0.9));
        cn.set_material("ceramic");
    });

    println!("Materials Demo");
    println!("==============");
    println!();
    println!("This demo shows all 8 matcap materials on icosphere meshes:");
    println!();
    for (i, &material) in MATERIALS.iter().enumerate() {
        let col = i % 4;
        let row = i / 4;
        println!("  Row {row}, Col {col}: {material}");
    }
    println!();
    println!("Additional structures:");
    println!("  - Point cloud (right side, wax material)");
    println!("  - Curve network (right side below, ceramic material)");
    println!();
    println!("Try switching materials:");
    println!("  - Expand a structure in the left panel");
    println!("  - Use the 'Material' dropdown to change its material");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope_rs::show();
}
