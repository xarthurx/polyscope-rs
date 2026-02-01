#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Materials demo showcasing all 8 matcap materials.
//!
//! This example creates a grid of real 3D models (Spot cow, Utah Teapot,
//! Stanford Bunny, Armadillo), each assigned a different material, plus a
//! point cloud and curve network to show materials across structure types.
//!
//! Run with: `cargo run --example materials_demo`
//!
//! Controls:
//! - Change each structure's material via the "Material" dropdown in its panel
//! - Left drag: Orbit camera
//! - Right drag: Pan camera
//! - Scroll: Zoom

use glam::Vec3;
use polyscope::Structure;

/// All available matcap material names.
const MATERIALS: &[&str] = &[
    "clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal",
];

/// OBJ model paths, cycled for each material slot.
const MODEL_PATHS: &[&str] = &[
    "assets/spot.obj",
    "assets/teapot.obj",
    "assets/bunny.obj",
    "assets/armadillo.obj",
];

const MODEL_NAMES: &[&str] = &["spot", "teapot", "bunny", "armadillo"];

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

    // Load and normalize all 4 models
    let base_models: Vec<(Vec<Vec3>, Vec<glam::UVec3>)> = MODEL_PATHS
        .iter()
        .map(|path| {
            let (mut verts, faces) = load_obj(path);
            normalize_mesh(&mut verts, 1.8);
            (verts, faces)
        })
        .collect();

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

    // Create a 4x2 grid, each with a different material and model
    let spacing = 2.5_f32;
    for (i, &material) in MATERIALS.iter().enumerate() {
        let col = (i % 4) as f32;
        let row = (i / 4) as f32;
        let pos = Vec3::new(col * spacing - 1.5 * spacing, -row * spacing, 0.0);

        let model_idx = i % 4;
        let (ref base_verts, ref base_faces) = base_models[model_idx];

        let name = format!("{}_{}", MODEL_NAMES[model_idx], material);
        let verts = transform_vertices(base_verts, pos, 1.0);
        polyscope::register_surface_mesh(&name, verts, base_faces.clone());
        polyscope::with_surface_mesh(&name, |mesh| {
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
    polyscope::register_point_cloud("point_cloud_wax", pc_points);
    polyscope::with_point_cloud("point_cloud_wax", |pc| {
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
    polyscope::register_curve_network_line("curve_ceramic", cn_nodes);
    polyscope::with_curve_network("curve_ceramic", |cn| {
        cn.set_color(Vec3::new(0.7, 0.8, 0.9));
        cn.set_material("ceramic");
    });

    println!("Materials Demo");
    println!("==============");
    println!();
    println!("This demo shows all 8 matcap materials on real 3D models:");
    println!();
    for (i, &material) in MATERIALS.iter().enumerate() {
        let col = i % 4;
        let row = i / 4;
        println!(
            "  Row {row}, Col {col}: {material} ({})",
            MODEL_NAMES[i % 4]
        );
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

    polyscope::show();
}
