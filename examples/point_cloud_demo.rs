#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
//! Point cloud demonstration showcasing all point cloud features.
//!
//! This demo shows:
//! - Basic point cloud registration
//! - Scalar quantities (height, distance)
//! - Vector quantities (normals, gradients)
//! - Color quantities (position-based colors)
//! - Point radius and material settings
//!
//! Run with: cargo run --example `point_cloud_demo`

use glam::Vec3;
use std::f32::consts::PI;

/// Generate points on a torus surface.
fn generate_torus(
    major_radius: f32,
    minor_radius: f32,
    n_major: usize,
    n_minor: usize,
) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(n_major * n_minor);
    for i in 0..n_major {
        let theta = 2.0 * PI * i as f32 / n_major as f32;
        for j in 0..n_minor {
            let phi = 2.0 * PI * j as f32 / n_minor as f32;
            let x = (major_radius + minor_radius * phi.cos()) * theta.cos();
            let y = (major_radius + minor_radius * phi.cos()) * theta.sin();
            let z = minor_radius * phi.sin();
            points.push(Vec3::new(x, y, z));
        }
    }
    points
}

/// Generate random points inside a sphere using rejection sampling.
fn generate_sphere_volume(radius: f32, count: usize, seed: u64) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(count);
    let mut rng_state = seed;

    while points.len() < count {
        // Simple LCG random number generator
        const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
        rng_state = rng_state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(1);
        let x = ((rng_state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;
        rng_state = rng_state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(1);
        let y = ((rng_state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;
        rng_state = rng_state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(1);
        let z = ((rng_state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;

        let p = Vec3::new(x, y, z);
        if p.length() <= 1.0 {
            points.push(p * radius);
        }
    }
    points
}

fn main() {
    env_logger::init();
    polyscope_rs::init().expect("Failed to initialize polyscope");

    // === Point Cloud 1: Torus with surface quantities ===
    let torus_points = generate_torus(1.0, 0.3, 40, 20);
    let num_torus = torus_points.len();

    let torus = polyscope_rs::register_point_cloud("torus", torus_points.clone());

    // Scalar quantity: height (Z coordinate)
    let heights: Vec<f32> = torus_points.iter().map(|p| p.z).collect();
    torus.add_scalar_quantity("height", heights);

    // Scalar quantity: distance from Y axis
    let dist_from_axis: Vec<f32> = torus_points
        .iter()
        .map(|p| (p.x * p.x + p.z * p.z).sqrt())
        .collect();
    torus.add_scalar_quantity("radius", dist_from_axis);

    // Vector quantity: approximate normals (pointing outward from minor circle)
    let normals: Vec<Vec3> = torus_points
        .iter()
        .map(|p| {
            // For a torus, the normal points from the center of the tube to the surface
            let theta = p.y.atan2(p.x);
            let tube_center = Vec3::new(theta.cos(), theta.sin(), 0.0);
            (*p - tube_center).normalize_or_zero()
        })
        .collect();
    torus.add_vector_quantity("normals", normals);

    // Color quantity: position-based RGB
    let colors: Vec<Vec3> = torus_points
        .iter()
        .map(|p| {
            Vec3::new(
                (p.x + 1.3) / 2.6, // Map [-1.3, 1.3] to [0, 1]
                (p.y + 1.3) / 2.6,
                (p.z + 0.3) / 0.6, // Map [-0.3, 0.3] to [0, 1]
            )
        })
        .collect();
    torus.add_color_quantity("position_color", colors);

    // === Point Cloud 2: Sphere volume with distance field ===
    let sphere_center = Vec3::new(3.0, 0.0, 0.0);
    let sphere_points: Vec<Vec3> = generate_sphere_volume(0.8, 500, 12345)
        .into_iter()
        .map(|p| p + sphere_center)
        .collect();
    let num_sphere = sphere_points.len();

    let sphere = polyscope_rs::register_point_cloud("sphere_volume", sphere_points.clone());

    // Scalar quantity: distance from sphere center
    let center_dist: Vec<f32> = sphere_points
        .iter()
        .map(|p| (*p - sphere_center).length())
        .collect();
    sphere.add_scalar_quantity("distance_from_center", center_dist);

    // Vector quantity: gradient (pointing outward from center)
    let gradients: Vec<Vec3> = sphere_points
        .iter()
        .map(|p| (*p - sphere_center).normalize_or_zero() * 0.2)
        .collect();
    sphere.add_vector_quantity("gradient", gradients);

    // Color quantity: shell coloring (inner=blue, outer=red)
    let shell_colors: Vec<Vec3> = sphere_points
        .iter()
        .map(|p| {
            let t = (*p - sphere_center).length() / 0.8;
            Vec3::new(t, 0.2, 1.0 - t)
        })
        .collect();
    sphere.add_color_quantity("shell_color", shell_colors);

    // === Point Cloud 3: Grid sampling for visualization ===
    let grid_origin = Vec3::new(-3.0, 0.0, 0.0);
    let mut grid_points = Vec::new();
    let grid_size = 8;
    let spacing = 0.15;

    for i in 0..grid_size {
        for j in 0..grid_size {
            for k in 0..grid_size {
                grid_points.push(
                    grid_origin
                        + Vec3::new(i as f32 * spacing, j as f32 * spacing, k as f32 * spacing),
                );
            }
        }
    }
    let num_grid = grid_points.len();

    let grid = polyscope_rs::register_point_cloud("grid", grid_points.clone());

    // Scalar quantity: sinusoidal field
    let field_values: Vec<f32> = grid_points
        .iter()
        .map(|p| {
            let local = *p - grid_origin;
            (local.x * 5.0).sin() * (local.y * 5.0).cos() * (local.z * 5.0).sin()
        })
        .collect();
    grid.add_scalar_quantity("wave_field", field_values);

    // Vector quantity: curl-like field
    let curl_field: Vec<Vec3> = grid_points
        .iter()
        .map(|p| {
            let local = *p - grid_origin;
            Vec3::new(
                -local.y.sin(),
                local.x.sin(),
                (local.x + local.y).cos() * 0.5,
            )
            .normalize_or_zero()
                * 0.05
        })
        .collect();
    grid.add_vector_quantity("curl_field", curl_field);

    println!("Point Cloud Demo");
    println!("================");
    println!();
    println!("Structures:");
    println!("  - torus: {num_torus} points (surface sampling)");
    println!("  - sphere_volume: {num_sphere} points (volume sampling)");
    println!("  - grid: {num_grid} points (regular grid)");
    println!();
    println!("Quantities on torus:");
    println!("  - height: Z coordinate (scalar)");
    println!("  - radius: distance from Y axis (scalar)");
    println!("  - normals: surface normals (vector)");
    println!("  - position_color: RGB from XYZ (color)");
    println!();
    println!("Quantities on sphere_volume:");
    println!("  - distance_from_center: radial distance (scalar)");
    println!("  - gradient: outward direction (vector)");
    println!("  - shell_color: blue (inner) to red (outer)");
    println!();
    println!("Quantities on grid:");
    println!("  - wave_field: sinusoidal function (scalar)");
    println!("  - curl_field: rotational vector field (vector)");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - Click structures in UI to see quantities");
    println!("  - ESC: Exit");

    polyscope_rs::show();
}
