//! GPU test for the pick depth buffer that a double-click uses to set the
//! view center (C++ `pickAtScreenCoords`).
//!
//! The test draws one point into the pick buffers of a headless engine. It
//! needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{PointCloud, RenderEngine, Vec3};

#[test]
fn pick_depth_gives_world_position_of_point_cloud_hit() {
    const SIZE: u32 = 64;
    let Ok(mut engine) = RenderEngine::new_headless(SIZE, SIZE).block_on() else {
        eprintln!("Skipping pick depth test: no GPU adapter available");
        return;
    };
    engine.init_pick_buffers(SIZE, SIZE);

    // The camera looks along -Z at the origin.
    engine.camera.set_aspect_ratio(1.0);
    engine
        .camera
        .look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
    engine.update_camera_uniforms();

    let point = Vec3::new(0.0, 0.0, 0.5);
    let mut pc = PointCloud::new("pick_depth_point", vec![point]);
    pc.set_point_radius(0.2);
    pc.init_gpu_resources(
        &engine.device,
        engine.point_bind_group_layout(),
        engine.camera_buffer(),
    );
    let global_start = engine.assign_pick_range("PointCloud", "pick_depth_point", 1);
    pc.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        global_start,
    );

    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick depth test encoder"),
        });
    {
        let mut pass = engine
            .begin_pick_pass(&mut encoder)
            .expect("pick buffers exist");
        pass.set_pipeline(engine.point_pick_pipeline());
        pass.set_bind_group(0, pc.pick_bind_group().expect("pick bind group"), &[]);
        pass.draw(0..6, 0..1);
    }
    engine.queue.submit(std::iter::once(encoder.finish()));

    // The pixels at the center show the point (even and odd columns).
    for (px, py) in [(32_u16, 32_u16), (33, 31)] {
        let (x, y) = (u32::from(px), u32::from(py));
        assert_eq!(engine.pick_at(x, y), Some(global_start));
        let depth = engine.pick_depth_at(x, y).expect("depth readback");
        assert!(depth > 0.0 && depth < 1.0, "depth {depth}");
        let screen = glam::Vec2::new(f32::from(px) + 0.5, f32::from(py) + 0.5);
        let hit = engine
            .camera
            .screen_depth_to_world(screen, depth, SIZE, SIZE)
            .expect("hit position");
        assert!(
            hit.distance(point) < 0.1,
            "pixel ({x}, {y}): hit {hit} must lie at the point {point}"
        );
    }

    // A corner pixel shows the background: no hit and no position.
    assert_eq!(engine.pick_at(0, 0), Some(0));
    let background = engine.pick_depth_at(0, 0).expect("depth readback");
    assert!(
        engine
            .camera
            .screen_depth_to_world(glam::Vec2::splat(0.5), background, SIZE, SIZE)
            .is_none()
    );
}
