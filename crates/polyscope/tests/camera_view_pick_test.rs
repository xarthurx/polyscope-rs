//! GPU test: camera views are in the pick pass, so a double-click on one can
//! set the view center (C++ camera views are pickable).
//!
//! The test needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{CameraView, RenderEngine, Vec3};

const SIZE: u16 = 96;

/// Returns the pixel that shows the world point `p`.
fn pixel_of(engine: &RenderEngine, p: Vec3) -> (u32, u32) {
    let clip = engine.camera.view_projection_matrix() * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let size = f32::from(SIZE);
    let index = |v: f32| {
        (0..SIZE)
            .find(|&i| f32::from(i) <= v && v < f32::from(i) + 1.0)
            .map(u32::from)
            .expect("point on screen")
    };
    (
        index(f32::midpoint(ndc.x, 1.0) * size),
        index(f32::midpoint(-ndc.y, 1.0) * size),
    )
}

#[test]
fn camera_view_is_in_the_pick_pass() {
    let size = u32::from(SIZE);
    let Ok(mut engine) = RenderEngine::new_headless(size, size).block_on() else {
        eprintln!("Skipping camera view pick test: no GPU adapter available");
        return;
    };
    engine.init_pick_buffers(size, size);
    engine.init_curve_network_pick_pipeline();
    engine.camera.set_aspect_ratio(1.0);
    engine
        .camera
        .look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
    engine.update_camera_uniforms();

    // A camera view whose wireframe starts at `root` and points along -X.
    let root = Vec3::new(0.3, 0.2, 0.0);
    let mut cv = CameraView::from_look_at("pick_camera_view", root, Vec3::ZERO, Vec3::Y, 60.0, 1.5);
    cv.init_render_data(
        &engine.device,
        engine.curve_network_edge_bind_group_layout(),
        engine.camera_buffer(),
        &engine.queue,
        1.0,
    );
    let num_edges = cv.render_data().expect("render data").num_edges;
    let start = engine.assign_pick_range("CameraView", "pick_camera_view", num_edges);
    cv.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        start,
    );

    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("camera view pick test encoder"),
        });
    {
        let mut pass = engine
            .begin_pick_pass(&mut encoder)
            .expect("pick buffers exist");
        pass.set_pipeline(engine.curve_network_pick_pipeline());
        pass.set_bind_group(0, cv.pick_bind_group().expect("pick bind group"), &[]);
        pass.draw(0..num_edges * 2, 0..1);
    }
    engine.queue.submit(std::iter::once(encoder.finish()));

    // The wireframe edges meet at the root: a pixel next to it shows the view.
    let (rx, ry) = pixel_of(&engine, root);
    let hit = (0..3)
        .flat_map(|i| (0..3).map(move |j| ((rx + i).saturating_sub(1), (ry + j).saturating_sub(1))))
        .find(|&(x, y)| {
            engine
                .pick_at(x, y)
                .is_some_and(|index| (start..start + num_edges).contains(&index))
        })
        .expect("camera view hit next to its root");

    // The lookup names the camera view, and the depth gives a world position on it.
    let index = engine.pick_at(hit.0, hit.1).expect("pick index");
    let (type_name, name, _) = engine.lookup_global_index(index).expect("pick range");
    assert_eq!((type_name, name), ("CameraView", "pick_camera_view"));
    let depth = engine.pick_depth_at(hit.0, hit.1).expect("depth readback");
    let screen = glam::Vec2::new(
        f32::from(u16::try_from(hit.0).expect("pixel")) + 0.5,
        f32::from(u16::try_from(hit.1).expect("pixel")) + 0.5,
    );
    let position = engine
        .camera
        .screen_depth_to_world(screen, depth, size, size)
        .expect("world position");
    assert!(
        position.distance(root) < 0.1,
        "position {position} must lie near the camera view root {root}"
    );
}
