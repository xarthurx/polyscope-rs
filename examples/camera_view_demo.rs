//! Demo showing camera view visualization in polyscope-rs.

use polyscope::{self, CameraExtrinsics, CameraIntrinsics, CameraParameters, Vec3};

fn main() {
    // Initialize polyscope
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a simple cube mesh for reference
    let vertices = vec![
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
    ];
    let faces = vec![
        glam::UVec3::new(0, 1, 2),
        glam::UVec3::new(0, 2, 3),
        glam::UVec3::new(1, 5, 6),
        glam::UVec3::new(1, 6, 2),
        glam::UVec3::new(5, 4, 7),
        glam::UVec3::new(5, 7, 6),
        glam::UVec3::new(4, 0, 3),
        glam::UVec3::new(4, 3, 7),
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 7),
        glam::UVec3::new(4, 5, 1),
        glam::UVec3::new(4, 1, 0),
    ];
    polyscope::register_surface_mesh("cube", vertices, faces);

    // Register multiple camera views from different angles

    // Camera 1: Looking at the cube from the front
    let cam1 = polyscope::register_camera_view_look_at(
        "front camera",
        Vec3::new(0.0, 0.0, 3.0), // position
        Vec3::ZERO,               // target
        Vec3::Y,                  // up
        60.0,                     // vertical FoV degrees
        1.5,                      // aspect ratio
    );
    cam1.set_color(Vec3::new(1.0, 0.0, 0.0)); // Red

    // Camera 2: Looking from the side
    let cam2 = polyscope::register_camera_view_look_at(
        "side camera",
        Vec3::new(3.0, 0.5, 0.0),
        Vec3::ZERO,
        Vec3::Y,
        45.0,
        16.0 / 9.0,
    );
    cam2.set_color(Vec3::new(0.0, 1.0, 0.0)); // Green

    // Camera 3: Looking from above
    let cam3 = polyscope::register_camera_view_look_at(
        "top camera",
        Vec3::new(0.0, 3.0, 0.5),
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -1.0), // Up is -Z when looking down
        50.0,
        1.0,
    );
    cam3.set_color(Vec3::new(0.0, 0.0, 1.0)); // Blue

    // Camera 4: Using raw parameters
    let params = CameraParameters::new(
        CameraIntrinsics::new(70.0, 2.0),
        CameraExtrinsics::new(
            Vec3::new(-2.0, 1.0, 2.0),
            Vec3::new(0.5, -0.2, -0.5).normalize(),
            Vec3::Y,
        ),
    );
    let cam4 = polyscope::register_camera_view("custom camera", params);
    cam4.set_color(Vec3::new(1.0, 1.0, 0.0)); // Yellow

    println!("Camera view demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    // Show the viewer
    polyscope::show();
}
