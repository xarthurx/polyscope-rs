//! GPU test: a volume mesh gets new pick resources when a slice plane changes
//! its render data.
//!
//! The pick resources refer to the vertex buffer and the cell order of the
//! render data. A slice plane culls cells and replaces the render data. Old
//! pick resources then draw old geometry, so a cell that the plane shows again
//! cannot be picked. The test needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{RenderEngine, Vec3, VolumeMesh};

/// Width and height of the image.
const SIZE: u16 = 64;

/// First pick index of the volume mesh.
const START: u32 = 1;

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

/// Draws the volume mesh into the pick buffer and reads the pick index at
/// each position.
fn pick(engine: &RenderEngine, vm: &VolumeMesh, positions: &[Vec3]) -> Vec<Option<u32>> {
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("volume mesh pick culling test encoder"),
        });
    {
        let mut pass = engine
            .begin_pick_pass(&mut encoder)
            .expect("pick buffers exist");
        pass.set_pipeline(engine.mesh_pick_pipeline());
        pass.set_bind_group(0, vm.pick_bind_group().expect("pick bind group"), &[]);
        pass.draw(0..vm.num_render_vertices(), 0..1);
    }
    engine.queue.submit(std::iter::once(encoder.finish()));
    positions
        .iter()
        .map(|&p| {
            let (x, y) = pixel_of(engine, p);
            engine.pick_at(x, y)
        })
        .collect()
}

#[test]
fn volume_mesh_pick_follows_slice_plane_culling() {
    let size = u32::from(SIZE);
    let Ok(mut engine) = RenderEngine::new_headless(size, size).block_on() else {
        eprintln!("Skipping volume mesh pick culling test: no GPU adapter available");
        return;
    };
    engine.init_pick_buffers(size, size);
    // The camera looks along -Z at the origin.
    engine.camera.set_aspect_ratio(1.0);
    engine
        .camera
        .look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
    engine.update_camera_uniforms();

    // Two separate tetrahedra: cell 0 at x = -0.5 and cell 1 at x = 0.5. The
    // apex of each one points to the camera.
    let tet = |x: f32| {
        [
            Vec3::new(x - 0.25, -0.25, 0.0),
            Vec3::new(x + 0.25, -0.25, 0.0),
            Vec3::new(x, 0.25, 0.0),
            Vec3::new(x, 0.0, 0.3),
        ]
    };
    let vertices: Vec<Vec3> = tet(-0.5).into_iter().chain(tet(0.5)).collect();
    let mut vm = VolumeMesh::new_tet_mesh(
        "pick_culling_tets",
        vertices,
        vec![[0, 1, 2, 3], [4, 5, 6, 7]],
    );
    let cells = [Vec3::new(-0.5, -0.05, 0.0), Vec3::new(0.5, -0.05, 0.0)];
    let init_pick = |vm: &mut VolumeMesh, engine: &RenderEngine| {
        vm.init_pick_resources(
            &engine.device,
            engine.mesh_pick_bind_group_layout(),
            engine.camera_buffer(),
            START,
        );
    };

    // A slice plane through the origin culls cell 0, at negative x.
    vm.update_render_data_with_culling(
        &engine.device,
        engine.mesh_bind_group_layout(),
        engine.camera_buffer(),
        &[(Vec3::ZERO, Vec3::X)],
    );
    init_pick(&mut vm, &engine);
    assert_eq!(pick(&engine, &vm, &cells), [Some(0), Some(START + 1)]);

    // Without the plane, the render data shows both cells again. The pick
    // resources of the culled render data must go.
    vm.reset_render_data(
        &engine.device,
        engine.mesh_bind_group_layout(),
        engine.camera_buffer(),
    );
    assert!(
        vm.pick_bind_group().is_none(),
        "new render data needs new pick resources"
    );
    init_pick(&mut vm, &engine);
    assert_eq!(pick(&engine, &vm, &cells), [Some(START), Some(START + 1)]);

    // A new plane culls again, so the pick resources go again.
    vm.update_render_data_with_culling(
        &engine.device,
        engine.mesh_bind_group_layout(),
        engine.camera_buffer(),
        &[(Vec3::ZERO, -Vec3::X)],
    );
    assert!(vm.pick_bind_group().is_none());
    init_pick(&mut vm, &engine);
    assert_eq!(pick(&engine, &vm, &cells), [Some(START), Some(0)]);
}
