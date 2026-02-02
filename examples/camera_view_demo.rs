#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Demo showing camera view visualization in polyscope-rs.
//!
//! Loads the Utah Teapot model and surrounds it with camera frustum
//! visualizations from multiple viewpoints (4 fixed + 5 random).

use polyscope::{self, CameraExtrinsics, CameraIntrinsics, CameraParameters, Vec3};
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

/// Simple deterministic pseudo-random number generator (xorshift32).
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Returns a float in [0, 1).
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }

    /// Returns a float in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

fn main() {
    polyscope::init().expect("Failed to initialize polyscope");

    // Load the teapot as the reference model (larger size)
    let (mut verts, faces) = load_obj("assets/teapot.obj");
    normalize_mesh(&mut verts, 4.0);
    polyscope::register_surface_mesh("teapot", verts, faces);
    polyscope::with_surface_mesh("teapot", |mesh| {
        mesh.set_surface_color(Vec3::new(0.7, 0.5, 0.3));
    });

    // --- Fixed cameras ---

    // Camera 1: Looking at the teapot from the front
    let cam1 = polyscope::register_camera_view_look_at(
        "front camera",
        Vec3::new(0.0, 0.0, 7.0),
        Vec3::ZERO,
        Vec3::Y,
        60.0,
        1.5,
    );
    cam1.set_color(Vec3::new(1.0, 0.0, 0.0)); // Red

    // Camera 2: Looking from the side
    let cam2 = polyscope::register_camera_view_look_at(
        "side camera",
        Vec3::new(7.0, 1.0, 0.0),
        Vec3::ZERO,
        Vec3::Y,
        45.0,
        16.0 / 9.0,
    );
    cam2.set_color(Vec3::new(0.0, 1.0, 0.0)); // Green

    // Camera 3: Looking from above
    let cam3 = polyscope::register_camera_view_look_at(
        "top camera",
        Vec3::new(0.0, 7.0, 1.0),
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
            Vec3::new(-5.0, 2.5, 4.0),
            Vec3::new(0.5, -0.2, -0.5).normalize(),
            Vec3::Y,
        ),
    );
    let cam4 = polyscope::register_camera_view("custom camera", params);
    cam4.set_color(Vec3::new(1.0, 1.0, 0.0)); // Yellow

    // --- 5 randomly placed cameras around the teapot ---

    let mut rng = Rng::new(42);

    // Distinct colors for random cameras
    let random_colors = [
        Vec3::new(1.0, 0.5, 0.0), // Orange
        Vec3::new(0.5, 0.0, 1.0), // Purple
        Vec3::new(0.0, 1.0, 1.0), // Cyan
        Vec3::new(1.0, 0.0, 1.0), // Magenta
        Vec3::new(0.5, 1.0, 0.0), // Lime
    ];

    for (i, &color) in random_colors.iter().enumerate() {
        // Random position on a sphere of radius 5–8 around the origin
        let radius = rng.range(5.0, 8.0);
        let theta = rng.range(0.0, 2.0 * PI); // azimuth
        let phi = rng.range(0.3, PI - 0.3); // elevation (avoid poles)

        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.cos();
        let z = radius * phi.sin() * theta.sin();
        let position = Vec3::new(x, y, z);

        // Random FoV and aspect ratio
        let fov = rng.range(40.0, 75.0);
        let aspect = rng.range(1.0, 2.0);

        let cam = polyscope::register_camera_view_look_at(
            format!("random camera {}", i + 1),
            position,
            Vec3::ZERO,
            Vec3::Y,
            fov,
            aspect,
        );
        cam.set_color(color);
    }

    println!("Camera View Demo");
    println!("=================");
    println!();
    println!("A Utah Teapot surrounded by 9 camera frustum visualizations");
    println!("(4 fixed + 5 randomly placed).");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
