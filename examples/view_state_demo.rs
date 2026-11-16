#![allow(clippy::cast_precision_loss)]
//! Demonstrates the view save/restore API.
//!
//! Run with: `cargo run --release --example view_state_demo`
//!
//! In the window: orbit the camera, then use the Save View / Load View
//! buttons in the View section to write/read a JSON file that captures the
//! camera pose + render-look state.
//!
//! After you close the window, the demo uses the programmatic API: it saves
//! the final view to a JSON file in the system temp directory, loads it back,
//! and renders the restored view headless to a PNG file.

use polyscope_rs::{
    Result, Vec3, ViewTransition, current_view_state, init, load_view_from_file,
    register_point_cloud, render_to_file, save_view_to_file, show, with_context_mut,
};

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
    println!("of the left panel to save the current camera pose to JSON and");
    println!("restore it later. Close the window to run the programmatic part.");
    show();

    // The view of the last frame stays available after the window closes.
    let view_path = std::env::temp_dir().join("polyscope_view_state_demo.json");
    save_view_to_file(&view_path)?;
    let saved = current_view_state()?;
    println!(
        "Saved the final view to {} (camera position {:?})",
        view_path.display(),
        saved.camera.position
    );

    // Change the background, then load the saved view. The loaded view
    // restores the camera and the render look, including the background.
    with_context_mut(|ctx| ctx.options.background_color = Vec3::new(0.8, 0.2, 0.2));
    load_view_from_file(&view_path, ViewTransition::Instant)?;

    let image_path = std::env::temp_dir().join("polyscope_view_state_demo.png");
    render_to_file(&image_path.to_string_lossy(), 800, 600)?;
    let restored = current_view_state()?;
    println!(
        "Rendered the restored view to {} (camera position {:?}, background {:?})",
        image_path.display(),
        restored.camera.position,
        restored.render.background_color
    );
    Ok(())
}
