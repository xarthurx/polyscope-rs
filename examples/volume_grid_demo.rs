#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Demo showing volume grid visualization in polyscope-rs.
//!
//! Demonstrates:
//! - Gridcube mode: colored cubes at each grid node
//! - Isosurface mode: marching cubes isosurface extraction
//! - Cell scalar quantities

use polyscope::{self, Vec3, VolumeGridVizMode};

fn main() {
    // Initialize polyscope
    polyscope::init().expect("Failed to initialize polyscope");

    // --- Grid 1: Sphere SDF with isosurface ---
    let nx = 30u32;
    let ny = 30u32;
    let nz = 30u32;
    let grid = polyscope::register_volume_grid(
        "sphere SDF",
        glam::UVec3::new(nx, ny, nz),
        Vec3::new(-1.5, -1.5, -1.5),
        Vec3::new(1.5, 1.5, 1.5),
    );
    grid.set_edge_color(Vec3::new(0.3, 0.3, 0.3));
    grid.set_cube_size_factor(0.8);

    // Signed distance field for a sphere of radius 1
    let mut sdf_values = Vec::new();
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32 / (nx - 1) as f32) * 3.0 - 1.5;
                let y = (j as f32 / (ny - 1) as f32) * 3.0 - 1.5;
                let z = (k as f32 / (nz - 1) as f32) * 3.0 - 1.5;
                let dist = (x * x + y * y + z * z).sqrt() - 1.0;
                sdf_values.push(dist);
            }
        }
    }
    grid.add_node_scalar_quantity("SDF", sdf_values);

    // Enable the quantity and set isosurface mode
    grid.set_quantity_enabled("SDF", true);
    grid.set_node_scalar_viz_mode("SDF", VolumeGridVizMode::Isosurface);
    grid.set_isosurface_level("SDF", 0.0);
    grid.set_isosurface_color("SDF", Vec3::new(0.047, 0.451, 0.690));

    // --- Grid 2: Gridcube visualization ---
    let grid2 = polyscope::register_volume_grid(
        "density field",
        glam::UVec3::new(10, 10, 10),
        Vec3::new(3.0, -1.0, -1.0),
        Vec3::new(5.0, 1.0, 1.0),
    );
    grid2.set_edge_color(Vec3::new(0.2, 0.2, 0.2));
    grid2.set_cube_size_factor(0.7);

    // Distance from center scalar field
    let mut values = Vec::new();
    for k in 0..10 {
        for j in 0..10 {
            for i in 0..10 {
                let x = (i as f32 / 9.0) * 2.0 - 1.0;
                let y = (j as f32 / 9.0) * 2.0 - 1.0;
                let z = (k as f32 / 9.0) * 2.0 - 1.0;
                let dist = (x * x + y * y + z * z).sqrt();
                values.push(dist);
            }
        }
    }
    grid2.add_node_scalar_quantity("distance", values);
    grid2.set_quantity_enabled("distance", true);
    grid2.set_node_scalar_viz_mode("distance", VolumeGridVizMode::Gridcube);

    // --- Grid 3: Cell scalar quantity ---
    let grid3 = polyscope::register_volume_grid(
        "temperature",
        glam::UVec3::new(6, 6, 6),
        Vec3::new(-4.0, -1.0, -1.0),
        Vec3::new(-2.0, 1.0, 1.0),
    );
    grid3.set_edge_color(Vec3::new(0.8, 0.2, 0.2));
    grid3.set_cube_size_factor(0.6);

    // Cell scalar: sinusoidal field (5x5x5 cells for a 6x6x6 node grid)
    let mut cell_values = Vec::new();
    for k in 0..5 {
        for j in 0..5 {
            for i in 0..5 {
                let x = (i as f32 + 0.5) / 5.0;
                let y = (j as f32 + 0.5) / 5.0;
                let z = (k as f32 + 0.5) / 5.0;
                let val = (x * std::f32::consts::PI * 2.0).sin()
                    * (y * std::f32::consts::PI).cos()
                    + z;
                cell_values.push(val);
            }
        }
    }
    grid3.add_cell_scalar_quantity("heat", cell_values);
    grid3.set_quantity_enabled("heat", true);
    grid3.set_color_map("heat", "coolwarm");

    println!("Volume grid demo running...");
    println!("Features:");
    println!("  - Left: Sphere SDF with isosurface extraction (isoval=0)");
    println!("  - Right: Distance field with gridcube visualization");
    println!("  - Far left: Cell scalar with coolwarm colormap");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    // Show the viewer
    polyscope::show();
}
