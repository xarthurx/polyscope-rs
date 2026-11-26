//! GPU test for pick indices past the 24 bits of the pick buffer.
//!
//! An index past the limit must read back as the background. Without the
//! limit in the pick shaders, the index `2^24 + 4` reads back as index 4,
//! which can belong to another structure. The test also creates the pick
//! pipelines that the engine creates only when a scene needs them, so their
//! shaders compile. It needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{PointCloud, RenderEngine, Vec3};

/// Number of indices in the 24-bit pick buffer.
const PICK_INDEX_COUNT: u32 = 1 << 24;

/// Draws the point cloud into the pick buffer and reads the center pixel.
fn pick_center(engine: &RenderEngine, pc: &PointCloud, size: u32) -> Option<u32> {
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick overflow test encoder"),
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
    engine.pick_at(size / 2, size / 2)
}

#[test]
fn pick_index_past_the_limit_reads_back_as_background() {
    const SIZE: u32 = 64;
    let Ok(mut engine) = RenderEngine::new_headless(SIZE, SIZE).block_on() else {
        eprintln!("Skipping pick overflow test: no GPU adapter available");
        return;
    };
    engine.init_pick_buffers(SIZE, SIZE);
    // These pipelines compile the other pick shaders with the same limit.
    engine.init_mesh_pick_pipeline();
    engine.init_curve_network_pick_pipeline();
    engine.init_curve_network_tube_pick_pipeline();
    engine.init_gridcube_pick_pipeline();

    // The camera looks along -Z at the origin.
    engine.camera.set_aspect_ratio(1.0);
    engine
        .camera
        .look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
    engine.update_camera_uniforms();

    let mut pc = PointCloud::new("pick_overflow_point", vec![Vec3::ZERO]);
    pc.set_point_radius(0.3);
    pc.init_gpu_resources(
        &engine.device,
        engine.point_bind_group_layout(),
        engine.camera_buffer(),
    );

    // The last index of the pick buffer reads back.
    let last = PICK_INDEX_COUNT - 1;
    pc.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        last,
    );
    assert_eq!(pick_center(&engine, &pc, SIZE), Some(last));

    // An index past the limit reads back as the background, not as index 4.
    pc.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        PICK_INDEX_COUNT + 4,
    );
    assert_eq!(pick_center(&engine, &pc, SIZE), Some(0));
}
