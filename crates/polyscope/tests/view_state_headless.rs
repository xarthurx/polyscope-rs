//! Integration test for view-state reproducibility through headless rendering.

use polyscope_core::view_state::{ViewTransition, apply_view_state, current_view_state};
use polyscope_rs::{Vec3, init, register_point_cloud, render_to_image};

#[test]
fn test_load_view_then_headless_uses_loaded_view() {
    // Initialize polyscope (ignore AlreadyInitialized when run alongside other tests)
    let _ = init();

    // Register a tiny scene
    register_point_cloud("view_state_test_pts", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

    // Render one preview frame so a snapshot exists
    let _ = render_to_image(64, 64).expect("first render");
    let baseline = current_view_state().expect("baseline");

    // Construct a view state with a deliberately-non-auto-fit camera
    let mut customised = baseline.clone();
    customised.camera.position = [10.0, 0.0, 0.0];
    customised.camera.target = [0.0, 0.0, 0.0];
    customised.camera.up = [0.0, 1.0, 0.0];

    // Queue the state, then render. Headless must apply it and skip auto-fit.
    apply_view_state(&customised, ViewTransition::Instant).expect("queue");
    let _ = render_to_image(64, 64).expect("second render");

    // After the second render, the published snapshot should reflect the
    // queued camera (not auto-fit's value).
    let after = current_view_state().expect("after");
    assert!(
        (after.camera.position[0] - 10.0).abs() < 1e-4,
        "expected position[0] ≈ 10.0 (from loaded view), got {}",
        after.camera.position[0]
    );
}
