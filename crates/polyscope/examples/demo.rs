//! Demo application showing basic polyscope-rs usage.

use polyscope::*;
use polyscope_structures::PointCloud;

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

    let handle = register_point_cloud("my points", points);

    // Add color based on position (creates a rainbow gradient)
    let colors: Vec<Vec3> = (0..1000)
        .map(|i| {
            let x = (i % 10) as f32 / 9.0;
            let y = ((i / 10) % 10) as f32 / 9.0;
            let z = (i / 100) as f32 / 9.0;
            Vec3::new(x, y, z)
        })
        .collect();
    handle.add_color_quantity("position colors", colors);

    // Enable the color quantity
    with_context_mut(|ctx| {
        if let Some(structure) = ctx.registry.get_mut("PointCloud", "my points") {
            if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                if let Some(q) = pc.get_quantity_mut("position colors") {
                    q.set_enabled(true);
                }
            }
        }
    });

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
