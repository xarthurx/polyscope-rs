//! Demo application showing basic polyscope-rs usage.
//!
//! This demo creates a sphere of points with scalar, color, and vector quantities.

use polyscope::*;
use polyscope_structures::PointCloud;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Create a sphere of points
    let mut points = Vec::new();
    let n = 20;
    for i in 0..n {
        for j in 0..n {
            let theta = std::f32::consts::PI * i as f32 / (n - 1) as f32;
            let phi = 2.0 * std::f32::consts::PI * j as f32 / n as f32;
            let r = 0.5;
            points.push(Vec3::new(
                r * theta.sin() * phi.cos(),
                r * theta.sin() * phi.sin(),
                r * theta.cos(),
            ));
        }
    }

    let num_points = points.len();
    let handle = register_point_cloud("sphere", points.clone());

    // Add scalar quantity (latitude = z coordinate)
    let scalars: Vec<f32> = points.iter().map(|p| p.z).collect();
    handle.add_scalar_quantity("latitude", scalars);

    // Add color quantity (position-based colors)
    let colors: Vec<Vec3> = points
        .iter()
        .map(|p| Vec3::new(p.x + 0.5, p.y + 0.5, p.z + 0.5))
        .collect();
    handle.add_color_quantity("position", colors);

    // Add vector quantity (normal vectors pointing outward)
    let vectors: Vec<Vec3> = points.iter().map(|p| p.normalize() * 0.1).collect();
    handle.add_vector_quantity("normals", vectors);

    // Enable scalar quantity by default (will use viridis colormap)
    with_context_mut(|ctx| {
        if let Some(structure) = ctx.registry.get_mut("PointCloud", "sphere") {
            if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                if let Some(q) = pc.get_quantity_mut("latitude") {
                    q.set_enabled(true);
                }
            }
        }
    });

    println!("Created sphere with {} points", num_points);
    println!("Quantities: latitude (scalar), position (color), normals (vector)");
    println!("Press Escape to close the viewer");

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
