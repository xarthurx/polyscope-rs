#![allow(clippy::cast_precision_loss)]
//! Demonstrates the view save/restore API.
//!
//! Run with: `cargo run --release --example view_state_demo`
//!
//! In the window: orbit the camera, then use the Save View / Load View
//! buttons in the right-panel View section to write/read a JSON file
//! that captures the camera pose + render-look state.

use polyscope_rs::{Result, Vec3, init, register_point_cloud, show};

fn main() -> Result<()> {
    env_logger::init();
    init()?;

    // A small 3D grid of points to look at
    let mut pts = Vec::new();
    for i in -3..=3 {
        for j in -3..=3 {
            for k in -3..=3 {
                pts.push(Vec3::new(i as f32, j as f32, k as f32) * 0.5);
            }
        }
    }
    let _ = register_point_cloud("grid", pts);

    println!("Use the 'Save View…' / 'Load View…' buttons in the View section");
    println!("of the right panel to save the current camera pose to JSON and");
    println!("restore it later.");
    show();
    Ok(())
}
