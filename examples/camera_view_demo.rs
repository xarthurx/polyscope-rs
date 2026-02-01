#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Demo showing camera view visualization in polyscope-rs.
//!
//! Loads the Utah Teapot model and surrounds it with camera frustum
//! visualizations from multiple viewpoints.

use polyscope::{self, CameraExtrinsics, CameraIntrinsics, CameraParameters, Vec3};

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

fn main() {
    polyscope::init().expect("Failed to initialize polyscope");

    // Load the teapot as the reference model
    let (mut verts, faces) = load_obj("assets/teapot.obj");
    normalize_mesh(&mut verts, 2.0);
    polyscope::register_surface_mesh("teapot", verts, faces);
    polyscope::with_surface_mesh("teapot", |mesh| {
        mesh.set_surface_color(Vec3::new(0.7, 0.5, 0.3));
    });

    // Camera 1: Looking at the teapot from the front
    let cam1 = polyscope::register_camera_view_look_at(
        "front camera",
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::ZERO,
        Vec3::Y,
        60.0,
        1.5,
    );
    cam1.set_color(Vec3::new(1.0, 0.0, 0.0)); // Red

    // Camera 2: Looking from the side
    let cam2 = polyscope::register_camera_view_look_at(
        "side camera",
        Vec3::new(5.0, 1.0, 0.0),
        Vec3::ZERO,
        Vec3::Y,
        45.0,
        16.0 / 9.0,
    );
    cam2.set_color(Vec3::new(0.0, 1.0, 0.0)); // Green

    // Camera 3: Looking from above
    let cam3 = polyscope::register_camera_view_look_at(
        "top camera",
        Vec3::new(0.0, 5.0, 1.0),
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -1.0),
        50.0,
        1.0,
    );
    cam3.set_color(Vec3::new(0.0, 0.0, 1.0)); // Blue

    // Camera 4: Using raw parameters
    let params = CameraParameters::new(
        CameraIntrinsics::new(70.0, 2.0),
        CameraExtrinsics::new(
            Vec3::new(-4.0, 2.0, 3.0),
            Vec3::new(0.5, -0.2, -0.5).normalize(),
            Vec3::Y,
        ),
    );
    let cam4 = polyscope::register_camera_view("custom camera", params);
    cam4.set_color(Vec3::new(1.0, 1.0, 0.0)); // Yellow

    println!("Camera View Demo");
    println!("=================");
    println!();
    println!("A Utah Teapot surrounded by 4 camera frustum visualizations.");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
