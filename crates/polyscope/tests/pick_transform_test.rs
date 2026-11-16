//! GPU test: the pick pass applies the model transform of each structure, as
//! its render shader does. A translated structure is hit at its transformed
//! position and not at its untransformed position.
//!
//! The test needs a GPU adapter (real or software fallback).

use pollster::FutureExt;
use polyscope_rs::{CurveNetwork, Mat4, PointCloud, RenderEngine, Structure, SurfaceMesh, Vec3};

const SIZE: u16 = 96;

/// Returns the pixel that shows the world point `p`.
fn pixel_of(engine: &RenderEngine, p: Vec3) -> (u32, u32) {
    let clip = engine.camera.view_projection_matrix() * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let size = f32::from(SIZE);
    let x = f32::midpoint(ndc.x, 1.0) * size;
    let y = f32::midpoint(-ndc.y, 1.0) * size;
    let index = |v: f32| {
        (0..SIZE)
            .find(|&i| f32::from(i) <= v && v < f32::from(i) + 1.0)
            .map(u32::from)
            .expect("point on screen")
    };
    (index(x), index(y))
}

/// Returns whether a pixel in the 3x3 block around `pixel` holds an index in `range`.
fn hit_near(engine: &RenderEngine, pixel: (u32, u32), range: &std::ops::Range<u32>) -> bool {
    (0..3).any(|i| {
        (0..3).any(|j| {
            let x = (pixel.0 + i).saturating_sub(1);
            let y = (pixel.1 + j).saturating_sub(1);
            engine
                .pick_at(x, y)
                .is_some_and(|index| range.contains(&index))
        })
    })
}

/// Clears the pick buffers and draws one structure with `pipeline`.
fn draw_pick(
    engine: &RenderEngine,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    vertices: std::ops::Range<u32>,
) {
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick transform test encoder"),
        });
    {
        let mut pass = engine
            .begin_pick_pass(&mut encoder)
            .expect("pick buffers exist");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(vertices, 0..1);
    }
    engine.queue.submit(std::iter::once(encoder.finish()));
}

/// Asserts a hit at `offset` and no hit at the world origin.
fn assert_hit_only_at(engine: &RenderEngine, offset: Vec3, start: u32, label: &str) {
    let range = start..start + 1;
    assert!(
        hit_near(engine, pixel_of(engine, offset), &range),
        "{label}: no hit at the transformed position"
    );
    assert!(
        !hit_near(engine, pixel_of(engine, Vec3::ZERO), &range),
        "{label}: hit at the untransformed position"
    );
}

/// One point at the local origin, translated by `offset`.
fn check_point_cloud(engine: &mut RenderEngine, offset: Vec3) {
    let mut pc = PointCloud::new("pick_transform_points", vec![Vec3::ZERO]);
    pc.set_point_radius(0.15);
    pc.init_gpu_resources(
        &engine.device,
        engine.point_bind_group_layout(),
        engine.camera_buffer(),
    );
    let start = engine.assign_pick_range("PointCloud", "pick_transform_points", 1);
    pc.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        start,
    );
    // The transform changes after the pick resources exist, as in a session.
    pc.set_transform(Mat4::from_translation(offset));
    pc.update_pick_uniforms(&engine.queue);
    let bind_group = pc.pick_bind_group().expect("point pick bind group");
    draw_pick(engine, engine.point_pick_pipeline(), bind_group, 0..6);
    assert_hit_only_at(engine, offset, start, "point cloud");
}

/// A triangle around the local origin, translated by `offset`.
fn check_surface_mesh(engine: &mut RenderEngine, offset: Vec3) {
    let mut mesh = SurfaceMesh::new(
        "pick_transform_mesh",
        vec![
            Vec3::new(-0.15, -0.15, 0.0),
            Vec3::new(0.15, -0.15, 0.0),
            Vec3::new(0.0, 0.2, 0.0),
        ],
        vec![vec![0, 1, 2]],
    );
    mesh.init_gpu_resources(
        &engine.device,
        engine.mesh_bind_group_layout(),
        engine.camera_buffer(),
    );
    let start = engine.assign_pick_range("SurfaceMesh", "pick_transform_mesh", 1);
    mesh.init_pick_resources(
        &engine.device,
        engine.mesh_pick_bind_group_layout(),
        engine.camera_buffer(),
        start,
    );
    mesh.set_transform(Mat4::from_translation(offset));
    mesh.update_pick_uniforms(&engine.queue);
    let bind_group = mesh.pick_bind_group().expect("mesh pick bind group");
    let vertices = 0..mesh.num_triangulation_vertices();
    draw_pick(engine, engine.mesh_pick_pipeline(), bind_group, vertices);
    assert_hit_only_at(engine, offset, start, "surface mesh");
}

/// One edge through the local origin, translated by `offset`.
fn check_curve_network(engine: &mut RenderEngine, offset: Vec3) {
    let mut cn = CurveNetwork::new(
        "pick_transform_curve",
        vec![Vec3::new(-0.2, 0.0, 0.0), Vec3::new(0.2, 0.0, 0.0)],
        vec![[0, 1]],
    );
    cn.init_gpu_resources(
        &engine.device,
        engine.curve_network_edge_bind_group_layout(),
        engine.camera_buffer(),
    );
    let start = engine.assign_pick_range("CurveNetwork", "pick_transform_curve", 1);
    cn.init_pick_resources(
        &engine.device,
        engine.pick_bind_group_layout(),
        engine.camera_buffer(),
        start,
    );
    cn.set_transform(Mat4::from_translation(offset));
    cn.update_gpu_buffers(&engine.queue, &engine.color_maps);
    cn.update_pick_uniforms(&engine.queue);
    let bind_group = cn.pick_bind_group().expect("curve pick bind group");
    draw_pick(
        engine,
        engine.curve_network_pick_pipeline(),
        bind_group,
        0..2,
    );
    assert_hit_only_at(engine, offset, start, "curve network");
}

#[test]
fn pick_pass_applies_the_model_transform() {
    let mut engine = RenderEngine::new_headless(u32::from(SIZE), u32::from(SIZE))
        .block_on()
        .expect("headless engine");
    engine.init_pick_buffers(u32::from(SIZE), u32::from(SIZE));
    engine.init_curve_network_pick_pipeline();
    engine.camera.set_aspect_ratio(1.0);
    engine
        .camera
        .look_at_box(Vec3::splat(-1.0), Vec3::splat(1.0));
    engine.update_camera_uniforms();

    check_point_cloud(&mut engine, Vec3::new(0.6, 0.3, 0.0));
    check_surface_mesh(&mut engine, Vec3::new(-0.6, -0.3, 0.0));
    check_curve_network(&mut engine, Vec3::new(0.0, -0.6, 0.0));
}
