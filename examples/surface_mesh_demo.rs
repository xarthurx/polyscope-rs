#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Surface mesh demonstration using the Stanford Bunny.
//!
//! Run with: cargo run --example `surface_mesh_demo`

use glam::{Vec2, Vec3};

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
    polyscope_rs::init().expect("Failed to initialize polyscope");

    // Load the Stanford Bunny
    let (vertices, faces) = load_obj("assets/bunny.obj");

    println!(
        "Loaded bunny: {} vertices, {} faces",
        vertices.len(),
        faces.len()
    );

    // Compute face normals for face vector quantity (before moving faces)
    let face_normals: Vec<Vec3> = faces
        .iter()
        .map(|f| {
            let v0 = vertices[f.x as usize];
            let v1 = vertices[f.y as usize];
            let v2 = vertices[f.z as usize];
            let e1 = v1 - v0;
            let e2 = v2 - v0;
            e1.cross(e2).normalize_or_zero()
        })
        .collect();

    // Compute per-vertex normals (area-weighted average of incident face normals)
    let mut vertex_normals = vec![Vec3::ZERO; vertices.len()];
    for (fi, f) in faces.iter().enumerate() {
        let normal = face_normals[fi];
        vertex_normals[f.x as usize] += normal;
        vertex_normals[f.y as usize] += normal;
        vertex_normals[f.z as usize] += normal;
    }
    for n in &mut vertex_normals {
        *n = n.normalize_or_zero();
    }

    let _mesh = polyscope_rs::register_surface_mesh("bunny", vertices.clone(), faces);

    // Get handle and add quantities via with_mesh
    polyscope_rs::with_surface_mesh("bunny", |mesh| {
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

        // Add vertex vector quantity (vertex normals)
        mesh.add_vertex_vector_quantity("vertex normals", vertex_normals.clone());

        // Add face vector quantity (face normals)
        mesh.add_face_vector_quantity("face normals", face_normals.clone());

        // Add parameterization quantity (UV from projection)
        let uv_coords: Vec<Vec2> = vertices
            .iter()
            .map(|v| Vec2::new(v.x * 5.0, v.z * 5.0))
            .collect();
        mesh.add_vertex_parameterization_quantity("uv_projection", uv_coords);

        // Add intrinsic vector quantity (tangent field, auto basis)
        let tangent_vecs: Vec<Vec2> = vertices
            .iter()
            .map(|v| Vec2::new(v.y.sin(), v.x.cos()).normalize_or_zero() * 0.02)
            .collect();
        mesh.add_vertex_intrinsic_vector_quantity_auto("tangent_field", tangent_vecs);

        // Add one-form quantity (edge-based flow values)
        let num_edges = mesh.edges().len();
        let edge_values: Vec<f32> = (0..num_edges)
            .map(|i| (i as f32 * 0.1).sin() * 0.02)
            .collect();
        let edge_orientations: Vec<bool> = (0..num_edges).map(|i| i % 2 == 0).collect();
        mesh.add_one_form_quantity("edge_flow", edge_values, edge_orientations);

        // Set a nice surface color
        mesh.set_surface_color(Vec3::new(0.8, 0.6, 0.4));
    });

    println!("Surface mesh demo running...");
    println!("Displaying the Stanford Bunny with quantities:");
    println!("  - height: vertex scalar (Y coordinate)");
    println!("  - height_color: vertex color (position-based)");
    println!("  - vertex normals: vertex vector quantity");
    println!("  - face normals: face vector quantity");
    println!("  - uv_projection: parameterization (checker)");
    println!("  - tangent_field: intrinsic vector (auto basis)");
    println!("  - edge_flow: one-form (edge-based flow)");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope_rs::show();
}
