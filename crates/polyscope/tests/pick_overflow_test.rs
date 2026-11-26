//! GPU test for pick indices past the 24 bits of the pick buffer.
//!
//! An index past the limit must read back as the background, also when the
//! first index plus the element index overflows `u32`. Without these limits in
//! the pick shaders, such an index reads back as a small index, which can
//! belong to another structure. The test also creates the pick pipelines that
//! the engine creates only when a scene needs them, so their shaders compile.
//! It needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{PointCloud, RenderEngine, Vec3};

/// Number of indices in the 24-bit pick buffer.
const PICK_INDEX_COUNT: u32 = 1 << 24;

/// Width and height of the image.
const SIZE: u16 = 64;

/// Three points in a row, so each point covers pixels of its own.
fn points() -> Vec<Vec3> {
    vec![
        Vec3::new(-0.5, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::new(0.5, 0.0, 0.0),
    ]
}

/// Returns the pixel of a world position.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inside the 64 x 64 image.
fn pixel_of(engine: &RenderEngine, p: Vec3) -> (u32, u32) {
    let clip = engine.camera.view_projection_matrix() * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let size = f32::from(SIZE);
    let x = f32::midpoint(ndc.x, 1.0) * size;
    let y = f32::midpoint(-ndc.y, 1.0) * size;
    (x as u32, y as u32)
}

/// Draws the points with this first pick index and reads the pixel of each point.
fn pick_points(engine: &RenderEngine, pc: &mut PointCloud, global_start: u32) -> Vec<Option<u32>> {
    pc.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        global_start,
    );
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
        pass.draw(0..6, 0..3);
    }
    engine.queue.submit(std::iter::once(encoder.finish()));
    points()
        .into_iter()
        .map(|p| {
            let (x, y) = pixel_of(engine, p);
            engine.pick_at(x, y)
        })
        .collect()
}

#[test]
fn pick_index_past_the_limit_reads_back_as_background() {
    let size = u32::from(SIZE);
    let Ok(mut engine) = RenderEngine::new_headless(size, size).block_on() else {
        eprintln!("Skipping pick overflow test: no GPU adapter available");
        return;
    };
    engine.init_pick_buffers(size, size);
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

    let mut pc = PointCloud::new("pick_overflow_points", points());
    pc.set_point_radius(0.15);
    pc.init_gpu_resources(
        &engine.device,
        engine.point_bind_group_layout(),
        engine.camera_buffer(),
    );

    // The last two indices of the pick buffer read back. The third point has
    // the first index past the limit.
    let last = PICK_INDEX_COUNT - 1;
    assert_eq!(
        pick_points(&engine, &mut pc, last - 1),
        [Some(last - 1), Some(last), Some(0)]
    );
    // Indices past the limit read back as the background, not as 4, 5 and 6.
    assert_eq!(
        pick_points(&engine, &mut pc, PICK_INDEX_COUNT + 4),
        [Some(0); 3]
    );
    // A sum that overflows `u32` reads back as the background, not as index 1.
    assert_eq!(pick_points(&engine, &mut pc, u32::MAX), [Some(0); 3]);
}
