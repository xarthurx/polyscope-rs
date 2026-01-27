#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Demo showing volume grid visualization in polyscope-rs.

use polyscope::{self, Vec3};

fn main() {
    // Initialize polyscope
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a 3D grid
    let grid = polyscope::register_volume_grid(
        "density field",
        glam::UVec3::new(10, 10, 10),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    );
    grid.set_edge_color(Vec3::new(0.3, 0.3, 0.3));

    // Add a scalar quantity (distance from center)
    let mut values = Vec::new();
    for k in 0..10 {
        for j in 0..10 {
            for i in 0..10 {
                // Compute position in [-1, 1]^3
                let x = (i as f32 / 9.0) * 2.0 - 1.0;
                let y = (j as f32 / 9.0) * 2.0 - 1.0;
                let z = (k as f32 / 9.0) * 2.0 - 1.0;
                let dist = (x * x + y * y + z * z).sqrt();
                values.push(dist);
            }
        }
    }
    grid.add_node_scalar_quantity("distance", values);

    // Create a second grid with different dimensions
    let grid2 = polyscope::register_volume_grid(
        "temperature",
        glam::UVec3::new(5, 8, 12),
        Vec3::new(2.0, -1.0, -1.5),
        Vec3::new(4.0, 1.0, 1.5),
    );
    grid2.set_edge_color(Vec3::new(0.8, 0.2, 0.2));

    // Add a sinusoidal scalar field
    let mut temp_values = Vec::new();
    for k in 0..12 {
        for j in 0..8 {
            for i in 0..5 {
                let x = i as f32 / 4.0;
                let y = j as f32 / 7.0;
                let z = k as f32 / 11.0;
                let val = (x * std::f32::consts::PI * 2.0).sin()
                    * (y * std::f32::consts::PI).cos()
                    * (z * std::f32::consts::PI * 1.5).sin();
                temp_values.push(val);
            }
        }
    }
    grid2.add_node_scalar_quantity("temperature", temp_values);

    // Create a uniform grid
    let _grid3 = polyscope::register_volume_grid_uniform(
        "uniform grid",
        8,
        Vec3::new(-3.0, -1.0, -1.0),
        Vec3::new(-1.5, 1.0, 1.0),
    );

    println!("Volume grid demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    // Show the viewer
    polyscope::show();
}
