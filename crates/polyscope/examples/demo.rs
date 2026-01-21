//! Demo application showing basic polyscope-rs usage.

use polyscope::*;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Create a simple point cloud (cube corners)
    let points = vec![
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    let pc = register_point_cloud("cube corners", points);

    // Add scalar quantity (height)
    pc.add_scalar_quantity("height", vec![-1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0]);

    // Add color quantity
    pc.add_color_quantity("colors", vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(1.0, 1.0, 1.0),
    ]);

    // Create a simple triangle mesh
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
    ];
    let faces = vec![glam::UVec3::new(0, 1, 2)];

    register_surface_mesh("triangle", verts, faces);

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
