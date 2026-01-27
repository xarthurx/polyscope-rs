#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Curve network demonstration.
//!
//! Run with: cargo run --example `curve_network_demo`

use glam::Vec3;
use std::f32::consts::PI;

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a helix curve
    let helix_nodes: Vec<Vec3> = (0..100)
        .map(|i| {
            let t = i as f32 * 0.1;
            Vec3::new(t.cos(), t * 0.1, t.sin())
        })
        .collect();

    // Compute tangent vectors at each node (derivative of the helix parameterization)
    let helix_tangents: Vec<Vec3> = (0..100)
        .map(|i| {
            let t = i as f32 * 0.1;
            Vec3::new(-t.sin(), 0.1, t.cos()).normalize()
        })
        .collect();

    let _helix = polyscope::register_curve_network_line("helix", helix_nodes.clone());

    polyscope::with_curve_network("helix", |c| {
        c.set_radius(0.02, true);
        c.set_color(Vec3::new(0.2, 0.8, 0.4));

        // Add node vector quantity (tangent vectors)
        c.add_node_vector_quantity("tangents", helix_tangents);

        // Add edge vector quantity (edge directions)
        let num_edges = c.num_edges();
        let nodes = c.nodes().to_vec();
        let tails = c.edge_tail_inds().to_vec();
        let tips = c.edge_tip_inds().to_vec();
        let edge_dirs: Vec<Vec3> = (0..num_edges)
            .map(|i| {
                let tail = nodes[tails[i] as usize];
                let tip = nodes[tips[i] as usize];
                (tip - tail).normalize_or_zero()
            })
            .collect();
        c.add_edge_vector_quantity("edge directions", edge_dirs);

        // Add node scalar quantity (height along helix)
        let node_heights: Vec<f32> = nodes.iter().map(|n| n.y).collect();
        c.add_node_scalar_quantity("height", node_heights);
    });

    // Create a circle as a loop
    let circle_nodes: Vec<Vec3> = (0..32)
        .map(|i| {
            let angle = 2.0 * PI * i as f32 / 32.0;
            Vec3::new(2.0 + 0.5 * angle.cos(), 0.0, 0.5 * angle.sin())
        })
        .collect();

    let _circle = polyscope::register_curve_network_loop("circle", circle_nodes);

    polyscope::with_curve_network("circle", |c| {
        c.set_radius(0.015, true);
        c.set_color(Vec3::new(0.8, 0.2, 0.4));
    });

    // Create a grid as explicit edges
    let mut grid_nodes = Vec::new();
    let mut grid_edges = Vec::new();

    // Create a 5x5 grid
    for i in 0..5 {
        for j in 0..5 {
            grid_nodes.push(Vec3::new(-2.0 + i as f32 * 0.3, 1.5, -0.6 + j as f32 * 0.3));
        }
    }

    // Horizontal edges
    for i in 0..5 {
        for j in 0..4 {
            let idx = i * 5 + j;
            grid_edges.push([idx as u32, (idx + 1) as u32]);
        }
    }

    // Vertical edges
    for i in 0..4 {
        for j in 0..5 {
            let idx = i * 5 + j;
            grid_edges.push([idx as u32, (idx + 5) as u32]);
        }
    }

    let _grid = polyscope::register_curve_network("grid", grid_nodes, grid_edges);

    polyscope::with_curve_network("grid", |c| {
        c.set_radius(0.01, true);
        c.set_color(Vec3::new(0.9, 0.7, 0.2));
    });

    // Create separate line segments
    let segment_nodes = vec![
        Vec3::new(-2.0, -0.5, 0.0),
        Vec3::new(-1.5, -0.3, 0.0),
        Vec3::new(-1.0, -0.5, 0.0),
        Vec3::new(-0.5, -0.3, 0.0),
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(0.5, -0.3, 0.0),
    ];

    let _segments = polyscope::register_curve_network_segments("segments", segment_nodes);

    polyscope::with_curve_network("segments", |c| {
        c.set_radius(0.025, true);
        c.set_color(Vec3::new(0.4, 0.4, 0.8));
    });

    println!("Curve network demo running...");
    println!("Displaying:");
    println!("  - Helix (green): Connected line curve");
    println!("    - tangents: node vector quantity");
    println!("    - edge directions: edge vector quantity");
    println!("    - height: node scalar quantity");
    println!("  - Circle (red): Closed loop");
    println!("  - Grid (yellow): Explicit edges");
    println!("  - Segments (blue): Separate line segments");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
