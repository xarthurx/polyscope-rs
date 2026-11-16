//! Camera motions without absolute floors: the first-person step and the
//! flight distance scale with the scene (scene sizes 0.01 and 1000).

use glam::Vec3;
use polyscope_render::Camera;

const SCALES: [f32; 2] = [0.01, 1000.0];

/// Returns a camera fitted to a cube of edge `scale` around `center`.
fn fitted_camera(scale: f32, center: Vec3) -> Camera {
    let mut camera = Camera::new(1.5);
    camera.look_at_box(center - 0.5 * scale, center + 0.5 * scale);
    camera
}

/// The first-person step follows the camera-to-target distance (C++
/// `CenterRelative`), so one second moves the same share of the scene at
/// every scale. An absolute floor of 1.0 moved 100 scene sizes at scale 0.01.
#[test]
fn first_person_step_scales_with_the_scene() {
    let ratios: Vec<f32> = SCALES
        .iter()
        .map(|&scale| {
            let camera = fitted_camera(scale, Vec3::ZERO);
            let step = camera.first_person_step(1.0);
            let distance = camera.position.distance(camera.target);
            assert!(
                (step - distance * camera.move_speed).abs() <= 1e-5 * distance,
                "scale {scale}: step {step}, distance {distance}"
            );
            step / scale
        })
        .collect();
    assert!(
        ratios[0] < 5.0,
        "step must stay near the scene size: {ratios:?}"
    );
    assert!(
        (ratios[0] - ratios[1]).abs() < 1e-3 * ratios[1],
        "{ratios:?}"
    );
}

/// A flight to a view matrix keeps the current camera-to-target distance,
/// also below the former absolute floor of 0.01.
#[test]
fn flight_keeps_the_view_distance_at_every_scale() {
    for scale in SCALES {
        let center = Vec3::new(0.2, -0.1, 0.3) * scale;
        let mut camera = fitted_camera(scale, center);
        // Zoom in to a distance of 0.3 scene sizes (0.003 at scale 0.01).
        let distance = 0.3 * scale;
        let fitted = camera.position.distance(camera.target);
        camera.zoom(fitted - distance);
        assert!((camera.position.distance(camera.target) - distance).abs() < 1e-4 * distance);

        let new_position = center + Vec3::new(1.0, 0.5, 1.0) * scale;
        let view = glam::camera::rh::view::look_at_mat4(new_position, center, Vec3::Y);
        camera.start_flight_to(view, camera.fov, 0.0);
        camera.update_flight();
        assert!(!camera.is_in_flight());

        let after = camera.position.distance(camera.target);
        assert!(
            (after - distance).abs() < 1e-3 * distance,
            "scale {scale}: distance {after}, expected {distance}"
        );
        assert!(camera.position.distance(new_position) < 1e-3 * scale);
        assert!(camera.is_view_valid());
    }
}

/// A degenerate view (camera at the target) flies with the near-plane
/// distance, which scales with the scene, and ends with a valid view.
#[test]
fn flight_from_a_degenerate_view_scales_with_the_scene() {
    for scale in SCALES {
        let mut camera = fitted_camera(scale, Vec3::ZERO);
        camera.position = camera.target;
        let new_position = Vec3::new(0.0, 0.0, 2.0 * scale);
        let view = glam::camera::rh::view::look_at_mat4(new_position, Vec3::ZERO, Vec3::Y);
        camera.start_flight_to(view, camera.fov, 0.0);
        camera.update_flight();
        let after = camera.position.distance(camera.target);
        assert!(
            (after - camera.near).abs() <= 1e-3 * camera.near,
            "scale {scale}: distance {after}, near {}",
            camera.near
        );
        assert!(after < 0.01 * scale);
        assert!(camera.is_view_valid());
    }
}
