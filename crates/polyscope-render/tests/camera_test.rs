//! Unit tests of the camera: navigation, projection, home view, flights and
//! view-state conversion. They use the public API of `polyscope_render::camera`.

// The library allows this lint at crate level; the tests use the same casts.
#![allow(clippy::cast_precision_loss)]

use glam::{Mat4, Vec3};
use polyscope_core::view_state::ViewTransition;
use polyscope_render::camera::*;
use std::time::{Duration, Instant};

fn make_camera_state_input() -> Camera {
    let mut c = Camera::new(1.5);
    c.position = Vec3::new(2.0, 3.0, 4.0);
    c.target = Vec3::new(0.5, 0.0, 0.0);
    c.up = Vec3::new(0.0, 1.0, 0.0);
    c.fov = 0.8;
    c.near = 0.1;
    c.far = 500.0;
    c.projection_mode = ProjectionMode::Orthographic;
    c.ortho_scale = 2.5;
    c.navigation_style = NavigationStyle::Arcball;
    c.up_direction = AxisDirection::NegY;
    c.front_direction = AxisDirection::PosZ;
    c
}

#[test]
fn test_camera_state_serializes_snake_case_enums() {
    let s = CameraState::from_camera(&make_camera_state_input());
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["projection_mode"], "orthographic");
    assert_eq!(v["navigation_style"], "arcball");
    assert_eq!(v["up_direction"], "neg_y");
    assert_eq!(v["front_direction"], "pos_z");
}

#[test]
fn test_camera_state_roundtrip() {
    let cam = make_camera_state_input();
    let state = CameraState::from_camera(&cam);
    let json = serde_json::to_string(&state).unwrap();
    let parsed: CameraState = serde_json::from_str(&json).unwrap();

    let mut fresh = Camera::new(1.5);
    parsed.apply(&mut fresh, ViewTransition::Instant);

    let eps = 1e-5;
    assert!((fresh.position - cam.position).length() < eps);
    assert!((fresh.target - cam.target).length() < eps);
    assert!((fresh.up - cam.up).length() < eps);
    assert!((fresh.fov - cam.fov).abs() < eps);
    assert!((fresh.near - cam.near).abs() < eps);
    assert!((fresh.far - cam.far).abs() < eps);
    assert_eq!(fresh.projection_mode, cam.projection_mode);
    assert!((fresh.ortho_scale - cam.ortho_scale).abs() < eps);
    assert_eq!(fresh.navigation_style, cam.navigation_style);
    assert_eq!(fresh.up_direction, cam.up_direction);
    assert_eq!(fresh.front_direction, cam.front_direction);
}

#[test]
fn test_camera_state_instant_does_not_start_flight() {
    let cam = make_camera_state_input();
    let state = CameraState::from_camera(&cam);
    let mut fresh = Camera::new(1.5);
    state.apply(&mut fresh, ViewTransition::Instant);
    assert!(fresh.flight.is_none());
}

#[test]
fn test_camera_state_flyto_starts_flight() {
    let cam = make_camera_state_input();
    let state = CameraState::from_camera(&cam);
    let mut fresh = Camera::new(1.5);
    fresh.position = Vec3::new(-1.0, -1.0, -1.0);
    fresh.target = Vec3::ZERO;
    fresh.up = Vec3::Y;

    state.apply(&mut fresh, ViewTransition::FlyTo);
    assert!(fresh.flight.is_some(), "FlyTo should start a camera flight");
    // Non-pose fields apply immediately
    assert_eq!(fresh.projection_mode, cam.projection_mode);
    assert!((fresh.ortho_scale - cam.ortho_scale).abs() < 1e-5);
}

/// Steps an active flight to `secs` seconds after its start.
fn step_flight(camera: &mut Camera, secs: f32) {
    assert!(camera.is_in_flight(), "flight must be active");
    camera.update_flight_at(Instant::now() + Duration::from_secs_f32(secs));
}

/// After a flight ends, every camera field must equal the requested state,
/// also when the camera-to-target distance changes.
#[test]
fn test_camera_state_flyto_ends_at_requested_state() {
    let cam = make_camera_state_input();
    let state = CameraState::from_camera(&cam);
    let mut fresh = Camera::new(1.5);
    fresh.position = Vec3::new(-1.0, -1.0, -1.0);
    fresh.target = Vec3::ZERO;
    fresh.up = Vec3::Y;
    let start_dist = fresh.position.distance(fresh.target);
    let end_dist = cam.position.distance(cam.target);
    assert!((start_dist - end_dist).abs() > 1.0, "distances must differ");

    state.apply(&mut fresh, ViewTransition::FlyTo);

    // Mid-flight: the view is valid and the distance lies between both ends.
    step_flight(&mut fresh, 0.2);
    assert!(fresh.is_in_flight());
    assert!(fresh.is_view_valid());
    let mid_dist = fresh.position.distance(fresh.target);
    assert!(mid_dist > start_dist.min(end_dist) && mid_dist < start_dist.max(end_dist));

    // Step past the end of the flight.
    step_flight(&mut fresh, 1.0);
    assert!(!fresh.is_in_flight());
    assert_eq!(CameraState::from_camera(&fresh), state);
    assert_eq!(fresh.position, cam.position);
    assert_eq!(fresh.target, cam.target);
    assert_eq!(fresh.up, cam.up);
}

#[test]
fn test_camera_state_unknown_enum_falls_back_in_apply() {
    let mut bad = CameraState::from_camera(&make_camera_state_input());
    bad.projection_mode = "definitely_not_a_mode".to_string();
    bad.navigation_style = "also_not_real".to_string();

    let mut target = Camera::new(1.5);
    let original_proj = target.projection_mode;
    let original_nav = target.navigation_style;
    bad.apply(&mut target, ViewTransition::Instant);

    assert_eq!(target.projection_mode, original_proj);
    assert_eq!(target.navigation_style, original_nav);
}

#[test]
fn test_projection_mode_perspective() {
    let camera = Camera::new(1.0);
    let proj = camera.projection_matrix();
    // Perspective matrix has non-zero w division
    assert!(proj.w_axis.z != 0.0);
}

#[test]
fn test_projection_mode_orthographic() {
    let mut camera = Camera::new(1.0);
    camera.projection_mode = ProjectionMode::Orthographic;
    camera.ortho_scale = 5.0;
    let proj = camera.projection_matrix();
    // Orthographic matrix has w_axis.w = 1.0, w_axis.z = 0.0
    assert!((proj.w_axis.w - 1.0).abs() < 0.001);
}

#[test]
fn test_set_fov_clamping() {
    let mut camera = Camera::new(1.0);
    camera.set_fov(0.0); // Too small
    assert!(camera.fov >= 0.1);

    camera.set_fov(std::f32::consts::PI); // Too large
    assert!(camera.fov < std::f32::consts::PI);
}

#[test]
fn test_zoom_perspective() {
    let mut camera = Camera::new(1.0);
    camera.projection_mode = ProjectionMode::Perspective;
    camera.position = Vec3::new(0.0, 0.0, 5.0);
    camera.target = Vec3::ZERO;

    let initial_distance = camera.position.distance(camera.target);
    camera.zoom(1.0); // Zoom in
    let new_distance = camera.position.distance(camera.target);

    assert!(
        new_distance < initial_distance,
        "Perspective zoom in should decrease distance"
    );
}

/// A scene of size 0.01 gives a fitted camera about 0.013 from the target.
/// One scroll step in must move the camera closer, not out to an absolute floor.
#[test]
fn test_zoom_in_small_scene_moves_closer() {
    let mut camera = Camera::new(1.0);
    camera.look_at_box(Vec3::splat(-0.005), Vec3::splat(0.005));
    let fitted = camera.position.distance(camera.target);
    assert!(fitted < 0.1, "fitted distance {fitted} must be small");

    let mut direct = camera.clone();
    direct.zoom(0.1 * fitted);
    let after_direct = direct.position.distance(direct.target);
    assert!(
        (after_direct - 0.9 * fitted).abs() < fitted * 1e-4,
        "zoom must not apply an absolute floor: {fitted} -> {after_direct}"
    );

    camera.zoom_scroll(1.0);
    let after = camera.position.distance(camera.target);
    assert!(
        after < fitted,
        "scroll-in must decrease the distance: {fitted} -> {after}"
    );
    // The step is relative to the distance (C++ CenterRelative: 0.075 per step).
    assert!((after - fitted * 0.925).abs() < fitted * 1e-4);

    // Many steps in never reach or cross the target, also far from the origin.
    for offset in [Vec3::ZERO, Vec3::new(1000.0, -50.0, 3.0)] {
        camera.look_at_box(offset - 0.005, offset + 0.005);
        for _ in 0..1000 {
            camera.zoom_scroll(5.0);
        }
        let close = camera.position.distance(camera.target);
        assert!(close > 0.0 && close < fitted, "offset {offset}: {close}");
        assert!(camera.view_matrix().is_finite());
        assert!(camera.forward().dot(Vec3::NEG_Z) > 0.999);
    }
}

/// The clip-plane and ortho-scale setters keep the fitted values of a small
/// or large scene. They only keep near > 0, far > near and scale > 0.
#[test]
fn test_clip_setters_keep_values_at_any_scale() {
    for half in [0.005_f32, 5.0e4] {
        let mut camera = Camera::new(1.0);
        camera.look_at_box(Vec3::splat(-half), Vec3::splat(half));
        let (near, far, ortho_scale) = (camera.near, camera.far, camera.ortho_scale);
        // The fitted ortho scale follows the box (0.6 of its height).
        assert!((ortho_scale - 1.2 * half).abs() < 1e-4 * half);
        camera.set_near(near);
        camera.set_far(far);
        camera.set_ortho_scale(ortho_scale);
        assert_eq!(camera.near.to_bits(), near.to_bits());
        assert_eq!(camera.far.to_bits(), far.to_bits());
        assert_eq!(camera.ortho_scale.to_bits(), ortho_scale.to_bits());
    }

    let mut camera = Camera::new(1.0);
    camera.set_near(0.0);
    assert!(camera.near > 0.0);
    camera.set_near(1e-6);
    camera.set_far(1e-7);
    assert!(camera.far > camera.near);
    camera.set_ortho_scale(-1.0);
    assert!(camera.ortho_scale > 0.0);
}

const ALL_AXES: [AxisDirection; 6] = [
    AxisDirection::PosX,
    AxisDirection::NegX,
    AxisDirection::PosY,
    AxisDirection::NegY,
    AxisDirection::PosZ,
    AxisDirection::NegZ,
];

/// Asserts a finite view that looks at `target` with `up` on top.
fn assert_looks_at(camera: &Camera, target: Vec3, up: Vec3, label: &str) {
    assert!(camera.is_view_valid(), "{label}: view not finite");
    let to_target = (target - camera.position).normalize();
    assert!(
        camera.forward().dot(to_target) > 0.9999,
        "{label}: not at target"
    );
    assert!(camera.camera_up().dot(up) > 0.0, "{label}: up not on top");
}

/// Changing the up direction from any view must keep a valid view of the
/// target (C++ commit 1e5ff25). Free and None navigation keep the view.
#[test]
fn test_set_up_direction_keeps_view_valid() {
    let styles = [
        NavigationStyle::Turntable,
        NavigationStyle::Planar,
        NavigationStyle::Arcball,
        NavigationStyle::FirstPerson,
        NavigationStyle::Free,
        NavigationStyle::None,
    ];
    let center = Vec3::new(1.0, 2.0, 3.0);
    for style in styles {
        for up in ALL_AXES {
            let mut camera = Camera::new(1.5);
            camera.look_at_box(center - 0.5, center + 0.5);
            camera.navigation_style = style;
            let view_before = camera.view_matrix();
            camera.set_up_direction(up);
            let label = format!("{style:?} up {}", up.name());
            assert!(camera.is_view_valid(), "{label}: view not finite");
            let up_vec = camera.up_direction.to_vec3();
            assert!(up_vec.dot(camera.front_direction.to_vec3()).abs() < 1e-6);
            if matches!(style, NavigationStyle::Free | NavigationStyle::None) {
                assert_eq!(camera.view_matrix(), view_before, "{label}");
            } else {
                assert_looks_at(&camera, center, up.to_vec3(), &label);
            }
        }
    }
}

/// The home view must honour every pair of up and front directions.
#[test]
fn test_home_view_honours_up_and_front() {
    let center = Vec3::new(-2.0, 0.5, 4.0);
    for up in ALL_AXES {
        for front in ALL_AXES {
            let label = format!("up {} front {}", up.name(), front.name());
            let orthogonal = up.to_vec3().dot(front.to_vec3()) == 0.0;

            // Fields set directly, as `CameraState::apply` does.
            let mut camera = Camera::new(1.5);
            camera.up_direction = up;
            camera.front_direction = front;
            camera.look_at_box(center - 1.0, center + 1.0);
            assert_looks_at(&camera, center, up.to_vec3(), &label);
            assert!(camera.forward().dot(up.to_vec3()).abs() < 1e-6, "{label}");
            if orthogonal {
                assert!(camera.forward().dot(front.to_vec3()) > 0.9999, "{label}");
            }

            // Setters: the front direction wins over a parallel up direction.
            let mut camera = Camera::new(1.5);
            camera.set_up_direction(up);
            camera.set_front_direction(front);
            assert_eq!(camera.front_direction, front, "{label}");
            if orthogonal {
                assert_eq!(camera.up_direction, up, "{label}");
            }
            camera.look_at_box(center - 1.0, center + 1.0);
            let up_vec = camera.up_direction.to_vec3();
            assert_looks_at(&camera, center, up_vec, &label);
            assert!(camera.forward().dot(front.to_vec3()) > 0.9999, "{label}");
        }
    }
}

/// The default home view must not change: the camera sits on +Z.
#[test]
fn test_default_home_view_is_unchanged() {
    let mut camera = Camera::new(1.5);
    camera.look_at_box(Vec3::new(-1.0, 0.0, 2.0), Vec3::new(3.0, 1.0, 4.0));
    assert_eq!(camera.position.x, 1.0);
    assert_eq!(camera.position.y, 0.5);
    assert!(camera.position.z > 3.0);
    assert_eq!(camera.up, Vec3::Y);
}

/// A view that is not valid falls back to the home view (C++ `ensureViewValid`).
#[test]
fn test_invalid_view_falls_back_to_home_view() {
    let mut camera = Camera::new(1.5);
    camera.look_at_box(Vec3::ZERO, Vec3::ONE);
    assert!(!camera.ensure_view_valid(Vec3::ZERO, Vec3::ONE));

    camera.up = camera.forward();
    assert!(!camera.is_view_valid());
    assert!(camera.ensure_view_valid(Vec3::ZERO, Vec3::ONE));
    assert_looks_at(&camera, Vec3::splat(0.5), Vec3::Y, "after fallback");
}

/// A new view center changes the view as C++ `setViewCenterAndLookAt` does
/// for each navigation style (upstream commit 1b41445).
#[test]
fn test_set_view_center_per_navigation_style() {
    let styles = [
        NavigationStyle::Turntable,
        NavigationStyle::Free,
        NavigationStyle::Planar,
        NavigationStyle::Arcball,
        NavigationStyle::FirstPerson,
        NavigationStyle::None,
    ];
    let new_center = Vec3::new(0.4, -0.3, 0.2);
    for style in styles {
        let mut camera = Camera::new(1.5);
        camera.look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
        camera.navigation_style = style;
        let before = camera.clone();

        camera.set_view_center(new_center);
        if camera.is_in_flight() {
            step_flight(&mut camera, 1.0);
        }
        assert!(camera.is_view_valid(), "{style:?}");

        match style {
            NavigationStyle::None => {
                assert_eq!(camera.position, before.position);
                assert_eq!(camera.target, before.target);
            }
            NavigationStyle::Planar => {
                // The view translates in its plane and does not rotate.
                assert_eq!(camera.target, new_center);
                assert!(
                    camera.forward().dot(before.forward()) > 0.99999,
                    "{style:?}"
                );
                let shift = camera.position - before.position;
                assert!(shift.dot(before.forward()).abs() < 1e-5, "{style:?}");
                assert!(camera.camera_up().dot(before.camera_up()) > 0.99999);
            }
            _ => {
                // The camera stays in place and turns to the new center.
                assert_eq!(camera.position, before.position, "{style:?}");
                assert_eq!(camera.target, new_center, "{style:?}");
                assert_eq!(camera.up, Vec3::Y, "{style:?}");
            }
        }
    }
}

/// A screen position and its depth-buffer value give back the world point.
#[test]
fn test_screen_depth_to_world_round_trip() {
    let (width, height) = (800_u32, 600_u32);
    for mode in [ProjectionMode::Perspective, ProjectionMode::Orthographic] {
        let mut camera = Camera::new(width as f32 / height as f32);
        camera.projection_mode = mode;
        camera.look_at_box(Vec3::new(-1.0, 0.0, 2.0), Vec3::new(3.0, 1.0, 4.0));
        let point = Vec3::new(0.3, 0.8, 3.1);
        let clip = camera.view_projection_matrix() * point.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        let screen = glam::Vec2::new(
            f32::midpoint(ndc.x, 1.0) * width as f32,
            f32::midpoint(-ndc.y, 1.0) * height as f32,
        );
        let back = camera
            .screen_depth_to_world(screen, ndc.z, width, height)
            .expect("finite position");
        // The f32 depth of a perspective projection limits the precision.
        assert!(back.distance(point) < 1e-2, "{mode:?}: {back} vs {point}");
        assert!(
            camera
                .screen_depth_to_world(screen, 1.0, width, height)
                .is_none()
        );
    }
}

/// The scroll-zoom step is relative, so it has the same effect at any scale.
#[test]
fn test_zoom_scroll_is_scale_invariant() {
    for scale in [1e-3_f32, 1.0, 1e3] {
        let mut camera = Camera::new(1.0);
        camera.look_at_box(Vec3::splat(-scale), Vec3::splat(scale));
        let before = camera.position.distance(camera.target);
        camera.zoom_scroll(-2.0);
        let ratio = camera.position.distance(camera.target) / before;
        assert!((ratio - 1.15).abs() < 1e-4, "scale {scale}: ratio {ratio}");

        camera.projection_mode = ProjectionMode::Orthographic;
        camera.ortho_scale = scale;
        camera.zoom_scroll(1.0);
        let ratio = camera.ortho_scale / scale;
        assert!(
            (ratio - 0.92).abs() < 1e-4,
            "scale {scale}: ortho ratio {ratio}"
        );
    }
}

#[test]
fn test_zoom_orthographic() {
    let mut camera = Camera::new(1.0);
    camera.projection_mode = ProjectionMode::Orthographic;
    camera.ortho_scale = 5.0;

    let initial_scale = camera.ortho_scale;
    camera.zoom(1.0); // Zoom in (positive delta)
    let new_scale = camera.ortho_scale;

    assert!(
        new_scale < initial_scale,
        "Orthographic zoom in should decrease scale"
    );
}

/// Mirrors the WGSL formula used by the curve tube ortho fix:
/// `forward = -vec3<f32>(view[0].z, view[1].z, view[2].z)`.
/// glam's `view.x_axis` is column 0, `view.x_axis.z` is `view[0].z` in WGSL.
fn shader_forward_from_view(view: Mat4) -> Vec3 {
    -Vec3::new(view.x_axis.z, view.y_axis.z, view.z_axis.z)
}

/// The shader's view-forward extraction must match `Camera::forward()` for
/// every camera pose, otherwise ortho ray casting in `curve_network_tube.wgsl`,
/// `reflected_curve_network_tube.wgsl`, and `pick_curve_tube.wgsl` will use
/// the wrong ray direction.
#[test]
fn shader_view_forward_matches_camera_forward() {
    let cases: &[(Vec3, Vec3, Vec3)] = &[
        // (eye, target, up)
        (Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y),
        (Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, Vec3::Y),
        (Vec3::new(0.0, 5.0, 0.0), Vec3::ZERO, Vec3::Z),
        (Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO, Vec3::Y),
        (
            Vec3::new(-2.0, 1.0, -3.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::Y,
        ),
        (
            Vec3::new(7.0, -2.0, 0.5),
            Vec3::new(-1.0, 0.0, 2.0),
            Vec3::Z,
        ),
    ];
    for &(eye, target, up) in cases {
        let view = glam::camera::rh::view::look_at_mat4(eye, target, up);
        let extracted = shader_forward_from_view(view);
        let expected = (target - eye).normalize();
        assert!(
            extracted.distance(expected) < 1e-5,
            "case eye={eye:?} target={target:?} up={up:?}: \
             extracted={extracted:?} expected={expected:?}"
        );
    }
}

/// `CameraUniforms` must be exactly 272 bytes: four mat4x4 (256) + vec3 +
/// f32 flag. If anyone changes the layout, every shader that aliases
/// `camera_pos` as `vec4<f32>` and reads `.w` as the ortho flag breaks
/// silently.
#[test]
fn camera_uniforms_layout_is_stable() {
    use polyscope_render::engine::CameraUniforms;
    assert_eq!(std::mem::size_of::<CameraUniforms>(), 272);
    assert_eq!(std::mem::align_of::<CameraUniforms>(), 4);
}

/// Setting `Camera::projection_mode` to `Orthographic` must propagate to
/// the GPU uniform's `is_orthographic` field as 1.0; perspective gives 0.0.
/// The three tube shaders branch on `camera.camera_pos.w > 0.5`, so the
/// exact float values matter.
///
/// Calls the same `CameraUniforms::from_camera` that `update_camera_uniforms`
/// uses on the render path, so this test catches drift if either side changes.
#[test]
fn ortho_flag_propagates_to_uniform() {
    use polyscope_render::engine::CameraUniforms;

    let mut camera = Camera::new(1.0);
    camera.projection_mode = ProjectionMode::Perspective;
    let u_persp = CameraUniforms::from_camera(&camera);
    assert!(
        u_persp.is_orthographic < 0.5,
        "perspective should produce flag below the 0.5 shader threshold, got {}",
        u_persp.is_orthographic
    );

    camera.projection_mode = ProjectionMode::Orthographic;
    let u_ortho = CameraUniforms::from_camera(&camera);
    assert!(
        u_ortho.is_orthographic > 0.5,
        "orthographic must clear the `> 0.5` shader threshold, got {}",
        u_ortho.is_orthographic
    );
}
