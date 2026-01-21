//! Demo application showing basic polyscope-rs usage.

use polyscope::*;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Create a grid of points (10x10x10 = 1000 points)
    let mut points = Vec::new();
    for i in 0..10 {
        for j in 0..10 {
            for k in 0..10 {
                points.push(Vec3::new(
                    i as f32 * 0.1 - 0.45,
                    j as f32 * 0.1 - 0.45,
                    k as f32 * 0.1 - 0.45,
                ));
            }
        }
    }

    register_point_cloud("my points", points);

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
