//! Demo application showcasing Phase 4 UI integration.
//!
//! This demo creates a sphere of points with scalar, color, and vector quantities
//! to demonstrate all Phase 4 features including:
//! - Structure tree UI with collapsible headers
//! - Structure-specific controls (color picker, radius slider)
//! - Quantity controls (enable/disable, colormap, range)
//! - Selection panel (left-click to select, right-click to clear)
//! - Camera controls (orbit, pan, zoom)

use polyscope::*;
use polyscope_structures::PointCloud;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Create a sphere of points
    let mut points = Vec::new();
    let n = 15; // 15x15 = 225 points for good visualization
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
    // This will color the sphere from bottom to top using a colormap
    let scalars: Vec<f32> = points.iter().map(|p| p.z).collect();
    handle.add_scalar_quantity("latitude", scalars);

    // Add color quantity (position-based RGB colors)
    // Maps x,y,z position to RGB, creating a smooth gradient
    let colors: Vec<Vec3> = points
        .iter()
        .map(|p| Vec3::new(p.x + 0.5, p.y + 0.5, p.z + 0.5))
        .collect();
    handle.add_color_quantity("position_color", colors);

    // Add vector quantity (surface normals pointing outward)
    // Visualizes the outward-facing normals as arrows
    let vectors: Vec<Vec3> = points.iter().map(|p| p.normalize() * 0.1).collect();
    handle.add_vector_quantity("normals", vectors);

    // Enable the scalar quantity by default to show colormap visualization
    with_context_mut(|ctx| {
        if let Some(structure) = ctx.registry.get_mut("PointCloud", "sphere") {
            if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                if let Some(q) = pc.get_quantity_mut("latitude") {
                    q.set_enabled(true);
                }
            }
        }
    });

    // Print usage instructions
    println!("=== polyscope-rs Phase 4 Demo ===");
    println!();
    println!("Created sphere with {} points", num_points);
    println!();
    println!("Quantities available:");
    println!("  - latitude (scalar): z-coordinate mapped to colormap");
    println!("  - position_color (color): position mapped to RGB");
    println!("  - normals (vector): outward-facing surface normals");
    println!();
    println!("=== UI Features to Test ===");
    println!();
    println!("LEFT PANEL:");
    println!("  [View] - Background color picker, Reset View button");
    println!("  [Structures] - Structure tree with 'sphere' point cloud");
    println!("    - Click header to expand/collapse structure UI");
    println!("    - Enabled checkbox to toggle visibility");
    println!("    - Color picker for base color");
    println!("    - Radius slider for point size");
    println!("    - Quantities section with checkboxes to enable/disable");
    println!("      - Scalar: colormap selector and range controls");
    println!("      - Color: simple enable/disable");
    println!("      - Vector: length, radius, and color controls");
    println!();
    println!("SELECTION (right panel):");
    println!("  - Left-click on points to select (shows selection panel)");
    println!("  - Right-click to clear selection");
    println!();
    println!("CAMERA CONTROLS:");
    println!("  - Left-drag: Orbit around target");
    println!("  - Right-drag: Pan");
    println!("  - Scroll: Zoom in/out");
    println!();
    println!("Press Escape to close the viewer");
    println!();

    // Show the viewer (blocks until closed)
    show();

    // Cleanup
    shutdown();

    Ok(())
}
