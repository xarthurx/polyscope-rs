//! GPU and CPU tests of picking: the pick ranges of the window pick pass,
//! the picks of meshes and grids, and the selection of a pick hit.
//!
//! The tests share the context lock and the renderer helpers of the parent
//! `tests` module.

use super::*;

#[test]
fn test_curve_pick_radius_follows_the_radius() {
    let _guard = setup();
    register_curve_network(
        "pick_radius_cn",
        vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
        vec![[0, 1]],
    );
    with_curve_network("pick_radius_cn", |cn| {
        cn.set_render_mode(1);
        cn.set_radius(0.01, false);
    });
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping curve pick radius test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);

    // The first frames create the pick resources with the small radius.
    app.render_pick_frame();
    app.render_pick_frame();
    // A point 0.15 away from the tube axis, in the image plane.
    let engine = app.engine.as_ref().expect("engine");
    let up = engine.camera.up_direction.to_vec3();
    let (x, y) = pixel_of(engine, up * 0.15);
    let hit = |app: &app::App| {
        let engine = app.engine.as_ref().expect("engine");
        engine.pick_at(x, y).is_some_and(|index| index > 0)
    };
    assert!(!hit(app), "the thin tube must not cover the test pixel");

    // The pick pass runs after the scene passes, which make the tube geometry
    // of the frame. So the next frame picks with the new radius.
    with_curve_network("pick_radius_cn", |cn| {
        cn.set_radius(0.3, false);
    });
    app.render_pick_frame();
    assert!(hit(app), "the pick pass must use the new tube radius");
}

/// Returns the pixel of a world position in the image of the engine.
fn pixel_of(engine: &RenderEngine, p: Vec3) -> (u32, u32) {
    let clip = engine.camera.view_projection_matrix() * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let x = f32::midpoint(ndc.x, 1.0) * engine.width as f32;
    let y = f32::midpoint(-ndc.y, 1.0) * engine.height as f32;
    (x as u32, y as u32)
}

/// Reads the pick buffer at the pixel of a world position. Returns the
/// structure type, the structure name and the element index.
fn pick_at_position(app: &app::App, p: Vec3) -> Option<(String, String, u32)> {
    let engine = app.engine.as_ref().expect("engine");
    let (x, y) = pixel_of(engine, p);
    let index = engine.pick_at(x, y)?;
    let (type_name, name, element) = engine.lookup_global_index(index)?;
    Some((type_name.to_owned(), name.to_owned(), element))
}

/// A structure gets new pick indices when its element count changes, and
/// a removed or empty structure owns no pick indices.
#[test]
fn test_pick_ranges_follow_element_count_changes() {
    let _guard = setup();
    // The corners of a cube. The camera fits the cube, so it shows all test points.
    let corners = vec![Vec3::splat(-1.0), Vec3::splat(1.0)];
    register_point_cloud("pick_grow", corners.clone());
    with_point_cloud("pick_grow", |pc| pc.set_point_radius(0.1));
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping pick range test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();

    // A row of four points in the image plane, at a height above the center.
    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up) = (view.row(0).truncate(), view.row(1).truncate());
    let row = |height: f32| -> Vec<Vec3> {
        [-0.6, -0.2, 0.2, 0.6]
            .iter()
            .map(|&x| right * x + up * height)
            .collect()
    };
    let element = |name: &str, index: u32| Some(("PointCloud".to_owned(), name.to_owned(), index));

    // The second point cloud gets the pick indices after the first one.
    register_point_cloud("pick_other", row(0.5));
    with_point_cloud("pick_other", |pc| pc.set_point_radius(0.1));
    app.render_pick_frame();

    // The first point cloud gets two more points.
    let mut grown = corners;
    grown.extend_from_slice(&row(-0.5)[..2]);
    with_point_cloud("pick_grow", |pc| pc.update_points(grown.clone()));
    app.render_pick_frame();
    assert_eq!(
        pick_at_position(app, grown[2]),
        element("pick_grow", 2),
        "a new point must not show as an element of another structure"
    );
    assert_eq!(pick_at_position(app, row(0.5)[0]), element("pick_other", 0));

    // A removed structure frees its pick indices.
    remove_structure("pick_other");
    app.render_pick_frame();
    let engine = app.engine.as_ref().expect("engine");
    assert_eq!(
        engine.get_pick_range_start("PointCloud", "pick_other"),
        None
    );
    assert_eq!(pick_at_position(app, grown[3]), element("pick_grow", 3));

    // An empty structure owns no pick indices.
    with_point_cloud("pick_grow", |pc| pc.update_points(Vec::new()));
    app.render_pick_frame();
    let engine = app.engine.as_ref().expect("engine");
    assert_eq!(engine.get_pick_range_start("PointCloud", "pick_grow"), None);
}

/// A structure that returns to the registry with its pick resources gets
/// a range of the frame. It cannot draw the indices of another structure.
#[test]
fn test_returning_structure_gets_a_pick_range() {
    let _guard = setup();
    // The corners of a cube. The camera fits the cube, so it shows all test points.
    register_point_cloud("pick_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping returning structure pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up) = (view.row(0).truncate(), view.row(1).truncate());
    let (left_point, right_point) = (up * 0.5 - right * 0.5, up * 0.5 + right * 0.5);
    register_point_cloud("pick_returning", vec![left_point]);
    with_point_cloud("pick_returning", |pc| pc.set_point_radius(0.1));
    app.render_pick_frame();

    // The structure leaves the registry with its pick resources. A new
    // structure gets the pick indices of the next frame.
    let returning = with_context_mut(|ctx| ctx.registry.remove("PointCloud", "pick_returning"))
        .expect("registered structure");
    register_point_cloud("pick_new", vec![right_point]);
    with_point_cloud("pick_new", |pc| pc.set_point_radius(0.1));
    app.render_pick_frame();

    with_context_mut(|ctx| ctx.registry.register(returning)).expect("register again");
    app.render_pick_frame();
    let element = |name: &str| Some(("PointCloud".to_owned(), name.to_owned(), 0));
    assert_eq!(pick_at_position(app, left_point), element("pick_returning"));
    assert_eq!(pick_at_position(app, right_point), element("pick_new"));
}

/// A structure that returns to the registry after the frame preparation,
/// as from another thread, gets its pick range in the pick pass of that
/// frame. It cannot draw the pick indices of another structure.
#[test]
fn test_structure_returning_before_the_pick_pass_gets_a_pick_range() {
    let _guard = setup();
    // The corners of a cube. The camera fits the cube, so it shows all test points.
    register_point_cloud("race_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping mid-frame pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up) = (view.row(0).truncate(), view.row(1).truncate());
    let (left_point, right_point) = (up * 0.5 - right * 0.5, up * 0.5 + right * 0.5);
    register_point_cloud("race_returning", vec![left_point]);
    with_point_cloud("race_returning", |pc| pc.set_point_radius(0.1));
    app.render_pick_frame();
    let returning = with_context_mut(|ctx| ctx.registry.remove("PointCloud", "race_returning"))
        .expect("registered structure");
    register_point_cloud("race_new", vec![right_point]);
    with_point_cloud("race_new", |pc| pc.set_point_radius(0.1));
    app.render_pick_frame();

    // The structure returns between the frame preparation and the pick pass.
    app.render_pick_frame_with(|| {
        with_context_mut(|ctx| ctx.registry.register(returning)).expect("register again");
    });
    let element = |name: &str| Some(("PointCloud".to_owned(), name.to_owned(), 0));
    assert_eq!(pick_at_position(app, left_point), element("race_returning"));
    assert_eq!(pick_at_position(app, right_point), element("race_new"));
}

/// A hidden structure gets no pick range, so it cannot use up the pick
/// indices of visible structures. It gets a range when it is visible.
#[test]
fn test_hidden_structures_get_no_pick_range() {
    let _guard = setup();
    register_point_cloud("pick_shown", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    register_point_cloud("pick_hidden", vec![Vec3::ZERO; 3]);
    with_point_cloud("pick_hidden", |pc| pc.set_enabled(false));
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping hidden structure pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();
    let start = |app: &app::App, name: &str| {
        let engine = app.engine.as_ref().expect("engine");
        engine.get_pick_range_start("PointCloud", name)
    };
    assert_eq!(start(app, "pick_hidden"), None);
    assert_eq!(start(app, "pick_shown"), Some(1));

    with_point_cloud("pick_hidden", |pc| pc.set_enabled(true));
    app.render_pick_frame();
    assert!(start(app, "pick_hidden").is_some());
}

/// The pick pass gives ranges to surface meshes and volume meshes, and a
/// pick reports the face or the cell.
#[test]
fn test_pick_pass_reports_mesh_faces_and_cells() {
    let _guard = setup();
    register_point_cloud("mesh_pick_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping mesh pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();

    // A square of two triangles at the left and a tetrahedron at the
    // right, in the image plane through the origin.
    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let at = |x: f32, y: f32| right * x + up * y;
    let square = vec![at(-0.8, 0.3), at(-0.3, 0.3), at(-0.3, 0.8), at(-0.8, 0.8)];
    register_surface_mesh(
        "mesh_pick_square",
        square,
        vec![vec![0, 1, 2], vec![0, 2, 3]],
    );
    let tet = vec![
        at(0.3, 0.3),
        at(0.8, 0.3),
        at(0.55, 0.8),
        at(0.55, 0.5) + back * 0.2,
    ];
    register_tet_mesh("mesh_pick_tet", tet, vec![[0, 1, 2, 3]]);
    app.render_pick_frame();

    let hit = |type_name: &str, name: &str, index: u32| {
        Some((type_name.to_owned(), name.to_owned(), index))
    };
    let square_hit = |index| hit("SurfaceMesh", "mesh_pick_square", index);
    assert_eq!(pick_at_position(app, at(-0.4, 0.4)), square_hit(0));
    assert_eq!(pick_at_position(app, at(-0.7, 0.7)), square_hit(1));
    assert_eq!(
        pick_at_position(app, at(0.55, 0.5)),
        hit("VolumeMesh", "mesh_pick_tet", 0)
    );
}

/// A pick on a gridcube reports the volume grid, also when its name
/// contains `/`. The quantity owns the pick range.
#[test]
fn test_gridcube_pick_reports_the_volume_grid() {
    let _guard = setup();
    let name = unique_name("pick/grid");
    // Three cells on each axis, so the grid center is inside a cell.
    let grid = register_volume_grid(
        &name,
        glam::UVec3::splat(4),
        Vec3::splat(-0.5),
        Vec3::splat(0.5),
    );
    grid.add_cell_scalar_quantity("density", vec![1.0; 27]);
    with_volume_grid(&name, |vg| enable_quantity(vg, "density"));
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping gridcube pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    // One frame creates the pick resources and gives them a range.
    app.render_pick_frame();
    let (type_name, structure, cell) =
        pick_at_position(app, Vec3::ZERO).expect("a gridcube at the grid center");
    assert_eq!(
        (type_name.as_str(), structure.as_str()),
        ("VolumeGrid", name.as_str())
    );
    assert!(cell < 27, "cell {cell}");
}

/// A node quantity and a cell quantity with the same name get separate
/// pick ranges, so the gridcubes of both report the grid.
#[test]
fn test_grid_quantities_with_the_same_name_get_separate_pick_ranges() {
    let _guard = setup();
    let name = unique_name("pick_same_names");
    // Three nodes on each axis. At the grid center pixel, the node cube at
    // the center of the front face is in front of all cell cubes.
    let grid = register_volume_grid(
        &name,
        glam::UVec3::splat(3),
        Vec3::splat(-0.5),
        Vec3::splat(0.5),
    );
    grid.add_node_scalar_quantity("density", vec![1.0; 27]);
    grid.add_cell_scalar_quantity("density", vec![1.0; 8]);
    with_volume_grid(&name, |vg| {
        for quantity in vg.quantities_mut() {
            quantity.set_enabled(true);
        }
    });
    let Some(mut renderer) = LiveRenderer::new() else {
        eprintln!("Skipping same-name grid pick test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    let engine = app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    app.render_pick_frame();
    app.render_pick_frame();
    let engine = app.engine.as_ref().expect("engine");
    let node_start = engine.get_quantity_pick_range_start("VolumeGrid", &name, 0);
    let cell_start = engine.get_quantity_pick_range_start("VolumeGrid", &name, 1);
    assert!(node_start.is_some() && cell_start.is_some());
    assert_ne!(node_start, cell_start);
    let (type_name, structure, _) =
        pick_at_position(app, Vec3::ZERO).expect("a gridcube at the grid center");
    assert_eq!(
        (type_name.as_str(), structure.as_str()),
        ("VolumeGrid", name.as_str())
    );
}

/// A volume grid hit keeps the full grid name when the selection applies
/// it, also when the name contains `/`.
#[test]
fn test_volume_grid_hit_keeps_a_name_with_slash() {
    use crate::app::{App, ClickHit};
    use std::time::{Duration, Instant};

    let _guard = setup();
    let name = unique_name("grid/with_slash");
    register_volume_grid(&name, glam::UVec3::splat(2), Vec3::ZERO, Vec3::ONE);
    let mut app = App::new();
    let hit = ClickHit::Structure {
        type_name: "VolumeGrid".to_string(),
        name: name.clone(),
        element_index: 0,
        element_type: polyscope_render::PickElementType::Cell,
    };
    app.queue_click_pick(Some(hit), glam::Vec2::ZERO);
    app.update_pending_pick(Instant::now() + Duration::from_secs(1));
    let selection = app
        .selection
        .as_ref()
        .expect("the click must select the grid");
    assert_eq!(selection.structure_name, name);
}

/// Creates a live renderer with pick buffers, or returns None without a GPU
/// adapter.
fn pick_renderer() -> Option<LiveRenderer> {
    let mut renderer = LiveRenderer::new()?;
    let engine = renderer.app.engine.as_mut().expect("engine");
    let (width, height) = (engine.width, engine.height);
    engine.init_pick_buffers(width, height);
    Some(renderer)
}

/// Returns the click hit at the pixel of a world position.
fn click_at_position(app: &app::App, p: Vec3) -> Option<crate::app::ClickHit> {
    let engine = app.engine.as_ref().expect("engine");
    let (x, y) = pixel_of(engine, p);
    app.click_hit_at(glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5))
}

/// Returns a structure hit of a click.
fn structure_hit(
    type_name: &str,
    name: &str,
    element_index: u32,
    element_type: polyscope_render::PickElementType,
) -> crate::app::ClickHit {
    crate::app::ClickHit::Structure {
        type_name: type_name.to_owned(),
        name: name.to_owned(),
        element_index,
        element_type,
    }
}

/// Tone maps the scene of the last frame and reads its RGBA pixels.
fn capture_scene(app: &mut app::App) -> Vec<u8> {
    let engine = app.engine.as_mut().expect("engine");
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test capture encoder"),
        });
    engine.tone_map_to_screenshot_target(&mut encoder);
    engine.queue.submit(std::iter::once(encoder.finish()));
    app.capture_to_buffer().expect("capture")
}

/// A click reports the face of a surface mesh and the cell of a volume mesh
/// that the pick buffer shows. Before, a click on a mesh reported element 0.
#[test]
fn test_click_reports_mesh_faces_and_cells() {
    use polyscope_render::PickElementType;

    let _guard = setup();
    register_point_cloud("click_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping mesh click test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    // In the image plane through the origin: a square of two triangles at the
    // left, and two separate tetrahedra at the right.
    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let at = |x: f32, y: f32| right * x + up * y;
    let square = vec![at(-0.8, 0.3), at(-0.3, 0.3), at(-0.3, 0.8), at(-0.8, 0.8)];
    register_surface_mesh("click_square", square, vec![vec![0, 1, 2], vec![0, 2, 3]]);
    let tet = |x: f32| {
        [
            at(x - 0.15, 0.3),
            at(x + 0.15, 0.3),
            at(x, 0.7),
            at(x, 0.45) + back * 0.1,
        ]
    };
    let vertices: Vec<Vec3> = tet(0.35).into_iter().chain(tet(0.75)).collect();
    register_tet_mesh("click_tets", vertices, vec![[0, 1, 2, 3], [4, 5, 6, 7]]);
    app.render_pick_frame();

    let face = |index| {
        Some(structure_hit(
            "SurfaceMesh",
            "click_square",
            index,
            PickElementType::Face,
        ))
    };
    assert_eq!(click_at_position(app, at(-0.7, 0.7)), face(1));
    assert_eq!(click_at_position(app, at(-0.4, 0.4)), face(0));
    let cell = |index| {
        Some(structure_hit(
            "VolumeMesh",
            "click_tets",
            index,
            PickElementType::Cell,
        ))
    };
    assert_eq!(click_at_position(app, at(0.75, 0.45)), cell(1));
    assert_eq!(click_at_position(app, at(0.35, 0.45)), cell(0));
}

/// A click on a gridcube selects the volume grid with the cell of a cell
/// quantity or the node of a node quantity. Before, a click ignored gridcubes.
#[test]
fn test_click_reports_volume_grid_cells_and_nodes() {
    use crate::app::ClickHit;
    use polyscope_render::PickElementType;

    let _guard = setup();
    // Cell gridcubes at the left, node gridcubes at the right. Each grid has
    // three cubes on each axis.
    let cells = unique_name("click/cells");
    let cell_grid = register_volume_grid(
        &cells,
        glam::UVec3::splat(4),
        Vec3::new(-1.0, -0.4, -0.4),
        Vec3::new(-0.2, 0.4, 0.4),
    );
    cell_grid.add_cell_scalar_quantity("density", vec![1.0; 27]);
    with_volume_grid(&cells, |vg| enable_quantity(vg, "density"));
    let nodes = unique_name("click_nodes");
    let node_grid = register_volume_grid(
        &nodes,
        glam::UVec3::splat(3),
        Vec3::new(0.2, -0.4, -0.4),
        Vec3::new(1.0, 0.4, 0.4),
    );
    node_grid.add_node_scalar_quantity("height", vec![1.0; 27]);
    with_volume_grid(&nodes, |vg| enable_quantity(vg, "height"));
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping volume grid click test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    // At the center of a grid, the click ray passes through the cubes with
    // i = 1 and j = 1. Their flat index is 4 + 9 k.
    let center_hit = |p: Vec3| match click_at_position(app, p) {
        Some(ClickHit::Structure {
            type_name,
            name,
            element_index,
            element_type,
        }) if type_name == "VolumeGrid" && element_index % 9 == 4 => Some((name, element_type)),
        other => panic!("no gridcube at the grid center: {other:?}"),
    };
    assert_eq!(
        center_hit(Vec3::new(-0.6, 0.0, 0.0)),
        Some((cells, PickElementType::Cell))
    );
    assert_eq!(
        center_hit(Vec3::new(0.6, 0.0, 0.0)),
        Some((nodes, PickElementType::Vertex))
    );
}

/// A click on the wireframe of a camera view selects the camera view.
/// Before, a click ignored camera views.
#[test]
fn test_click_selects_a_camera_view() {
    use crate::app::ClickHit;
    use polyscope_render::PickElementType;

    let _guard = setup();
    register_point_cloud("click_cv_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let name = unique_name("click_camera");
    // A camera view whose wireframe starts at `root` and points along -X.
    let root = Vec3::new(0.3, 0.2, 0.0);
    register_camera_view_look_at(&name, root, Vec3::ZERO, Vec3::Y, 60.0, 1.5);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping camera view click test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    // The wireframe edges meet at the root: a pixel next to it shows the view.
    let (rx, ry) = pixel_of(app.engine.as_ref().expect("engine"), root);
    let hit = (0..3)
        .flat_map(|i| (0..3).map(move |j| ((rx + i).saturating_sub(1), (ry + j).saturating_sub(1))))
        .find_map(|(x, y)| {
            match app.click_hit_at(glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5)) {
                Some(ClickHit::Structure {
                    type_name,
                    name,
                    element_type,
                    ..
                }) if type_name == "CameraView" => Some((name, element_type)),
                _ => None,
            }
        });
    assert_eq!(hit, Some((name, PickElementType::Edge)));
}

/// The pick pass draws a structure with the transform of the scene passes of
/// its frame. If another thread moves the structure after the frame
/// preparation, the scene and the pick buffer of that frame both show the old
/// position, and the next frame shows the new position in both.
#[test]
fn test_pick_pass_uses_the_transform_of_the_scene() {
    let _guard = setup();
    register_point_cloud("move_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping pick transform test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();
    let reference = capture_scene(app);

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let old = up * 0.5 - right * 0.5;
    let new = up * 0.5 + right * 0.5;
    register_point_cloud("move_point", vec![old]);
    with_point_cloud("move_point", |pc| pc.set_point_radius(0.1));
    // A tetrahedron below the point. Its center is inside its outline.
    let old_tet = -right * 0.5;
    let tet = vec![
        old_tet - right * 0.12 - up * 0.1,
        old_tet + right * 0.12 - up * 0.1,
        old_tet + up * 0.15,
        old_tet + back * 0.1,
    ];
    register_tet_mesh("move_tet", tet, vec![[0, 1, 2, 3]]);
    app.render_pick_frame();

    app.render_pick_frame_with(|| {
        set_point_cloud_transform("move_point", Mat4::from_translation(new - old));
        set_volume_mesh_transform("move_tet", Mat4::from_translation(new - old));
    });
    let image = capture_scene(app);
    let engine = app.engine.as_ref().expect("engine");
    let color = |image: &[u8], p: Vec3| -> [u8; 4] {
        let (x, y) = pixel_of(engine, p);
        let i = 4 * (y * engine.width + x) as usize;
        image[i..i + 4].try_into().expect("an RGBA pixel")
    };
    // The scene of this frame shows the point at the old position.
    assert_ne!(color(&image, old), color(&reference, old));
    assert_eq!(color(&image, new), color(&reference, new));
    // The pick buffer of this frame shows it there too, and the tetrahedron.
    let point = Some(("PointCloud".to_owned(), "move_point".to_owned(), 0));
    assert_eq!(pick_at_position(app, old), point);
    assert_ne!(pick_at_position(app, new), point);
    let cell = Some(("VolumeMesh".to_owned(), "move_tet".to_owned(), 0));
    let new_tet = old_tet + (new - old);
    assert_eq!(pick_at_position(app, old_tet), cell);
    assert_ne!(pick_at_position(app, new_tet), cell);

    // The next frame shows the new position in both.
    app.render_pick_frame();
    assert_eq!(pick_at_position(app, new), point);
    assert_ne!(pick_at_position(app, old), point);
    assert_eq!(pick_at_position(app, new_tet), cell);
}

/// The pick uniforms of a gridcube quantity follow its render uniforms in
/// every mode. Gridcubes that the UI shows again after an isosurface are
/// picked where the scene draws them.
#[test]
fn test_gridcubes_after_an_isosurface_are_picked_in_place() {
    use crate::app::ClickHit;
    use polyscope_structures::volume_grid::VolumeGridNodeScalarQuantity;

    let _guard = setup();
    register_point_cloud("mode_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let name = unique_name("mode_grid");
    let grid = register_volume_grid(
        &name,
        glam::UVec3::splat(3),
        Vec3::new(-0.9, -0.3, -0.3),
        Vec3::new(-0.3, 0.3, 0.3),
    );
    grid.add_node_scalar_quantity("height", vec![1.0; 27]);
    with_volume_grid(&name, |vg| enable_quantity(vg, "height"));
    let set_mode = |mode| {
        with_volume_grid(&name, |vg| {
            for quantity in vg.quantities_mut() {
                if let Some(nsq) = quantity
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    nsq.set_viz_mode(mode);
                }
            }
        });
    };
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping gridcube mode test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    // The grid moves to the right while it shows an isosurface.
    set_mode(VolumeGridVizMode::Isosurface);
    let shift = Vec3::new(1.2, 0.0, 0.0);
    with_volume_grid(&name, |vg| vg.set_transform(Mat4::from_translation(shift)));
    app.render_pick_frame();

    // The UI shows the gridcubes again after the frame preparation.
    app.render_pick_frame_with(|| set_mode(VolumeGridVizMode::Gridcube));
    match click_at_position(app, Vec3::new(-0.6, 0.0, 0.0) + shift) {
        Some(ClickHit::Structure {
            type_name,
            name: hit_name,
            ..
        }) if type_name == "VolumeGrid" => assert_eq!(hit_name, name),
        other => panic!("no gridcube at the moved grid center: {other:?}"),
    }
}

/// A gridcube click reports the element type of the frame that the window
/// shows, also if the quantities of the grid change before the click.
#[test]
fn test_gridcube_click_keeps_the_element_type_of_the_frame() {
    use crate::app::ClickHit;
    use polyscope_render::PickElementType;
    use polyscope_structures::volume_grid::VolumeGridNodeScalarQuantity;

    let _guard = setup();
    let name = unique_name("removal_grid");
    let grid = register_volume_grid(
        &name,
        glam::UVec3::splat(4),
        Vec3::splat(-0.4),
        Vec3::splat(0.4),
    );
    // Quantity 0 shows an isosurface, so the pick buffer shows the cell
    // gridcubes of quantity 1.
    grid.add_node_scalar_quantity("height", vec![1.0; 64]);
    grid.add_cell_scalar_quantity("density", vec![1.0; 27]);
    with_volume_grid(&name, |vg| {
        for quantity in vg.quantities_mut() {
            quantity.set_enabled(true);
            if let Some(nsq) = quantity
                .as_any_mut()
                .downcast_mut::<VolumeGridNodeScalarQuantity>()
            {
                nsq.set_viz_mode(VolumeGridVizMode::Isosurface);
            }
        }
    });
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping gridcube element type test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    // After the frame, the node quantity goes, and the cell quantity becomes quantity 0.
    with_volume_grid(&name, |vg| {
        vg.remove_quantity("height");
    });
    match click_at_position(app, Vec3::ZERO) {
        Some(ClickHit::Structure {
            type_name,
            element_index,
            element_type,
            ..
        }) if type_name == "VolumeGrid" => {
            assert_eq!(element_type, PickElementType::Cell);
            assert_eq!(element_index % 9, 4, "cell {element_index}");
        }
        other => panic!("no gridcube at the grid center: {other:?}"),
    }
}

/// A slice plane under the cursor wins if it is nearer than the element in
/// the pick buffer, and loses if it is behind the element. The clicks are at
/// the corner of a pixel, so they test that the plane test and the pick depth
/// use the same pixel center.
#[test]
fn test_click_compares_slice_planes_with_the_pick_depth() {
    use crate::app::ClickHit;
    use polyscope_render::PickElementType;

    let _guard = setup();
    register_point_cloud("plane_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping slice plane click test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let at = |x: f32, y: f32| right * x + up * y;
    let square = vec![at(-0.8, 0.3), at(-0.3, 0.3), at(-0.3, 0.8), at(-0.8, 0.8)];
    register_surface_mesh("plane_square", square, vec![vec![0, 1, 2, 3]]);
    let target = at(-0.55, 0.55);
    let click = |app: &app::App| {
        let (x, y) = pixel_of(app.engine.as_ref().expect("engine"), target);
        app.click_hit_at(glam::Vec2::new(x as f32, y as f32))
    };

    // A slice plane behind the square. The square stays on its visible side.
    add_slice_plane_with_pose("plane_behind", target - back * 0.3, back).set_plane_size(1.0);
    app.render_pick_frame();
    assert_eq!(
        click(app),
        Some(structure_hit(
            "SurfaceMesh",
            "plane_square",
            0,
            PickElementType::Face
        ))
    );
    remove_slice_plane("plane_behind");

    // A slice plane in front of the square.
    add_slice_plane_with_pose("plane_front", target + back * 0.3, -back).set_plane_size(1.0);
    app.render_pick_frame();
    assert_eq!(click(app), Some(ClickHit::Plane("plane_front".to_owned())));
}

/// A curve network in line mode draws a line, so its pick depth is the depth
/// of the line and not the front of its wider click area. A line just behind
/// a surface mesh does not cover the mesh in the pick buffer.
#[test]
fn test_line_behind_a_mesh_does_not_cover_it() {
    use polyscope_render::PickElementType;

    let _guard = setup();
    register_point_cloud("line_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping line pick depth test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let at = |x: f32, y: f32| right * x + up * y;
    let square = vec![at(-0.8, 0.3), at(-0.3, 0.3), at(-0.3, 0.8), at(-0.8, 0.8)];
    register_surface_mesh("line_square", square, vec![vec![0, 1, 2, 3]]);
    // A line 0.02 behind the square. Its click area has a radius of 0.05.
    let behind = -back * 0.02;
    register_curve_network(
        "line_behind",
        vec![at(-1.0, 0.55) + behind, at(-0.1, 0.55) + behind],
        vec![[0, 1]],
    );
    with_curve_network("line_behind", |cn| {
        cn.set_radius(0.05, false);
    });
    app.render_pick_frame();

    // The square hides the line, so a click there selects the square.
    assert_eq!(
        click_at_position(app, at(-0.55, 0.55)),
        Some(structure_hit(
            "SurfaceMesh",
            "line_square",
            0,
            PickElementType::Face
        ))
    );
    // Beside the square, a click selects the line.
    assert_eq!(
        click_at_position(app, at(-0.95, 0.55) + behind),
        Some(structure_hit(
            "CurveNetwork",
            "line_behind",
            0,
            PickElementType::Edge
        ))
    );
}

/// A click uses the camera of the frame in the pick buffer. A scroll moves
/// the camera before the next frame, but a click until then still compares
/// the pick depth with a slice plane as the window shows them.
#[test]
fn test_click_uses_the_camera_of_the_pick_buffer() {
    use polyscope_render::PickElementType;

    let _guard = setup();
    register_point_cloud("zoom_cube", vec![Vec3::splat(-1.0), Vec3::splat(1.0)]);
    let Some(mut renderer) = pick_renderer() else {
        eprintln!("Skipping click camera test: no GPU adapter available");
        return;
    };
    let app = &mut renderer.app;
    app.render_pick_frame();

    let view = app.engine.as_ref().expect("engine").camera.view_matrix();
    let (right, up, back) = (
        view.row(0).truncate(),
        view.row(1).truncate(),
        view.row(2).truncate(),
    );
    let at = |x: f32, y: f32| right * x + up * y;
    let square = vec![at(-0.8, 0.3), at(-0.3, 0.3), at(-0.3, 0.8), at(-0.8, 0.8)];
    register_surface_mesh("zoom_square", square, vec![vec![0, 1, 2, 3]]);
    // A slice plane 0.1 behind the square. The square stays on its visible side.
    let target = at(-0.55, 0.55);
    add_slice_plane_with_pose("zoom_plane", target - back * 0.1, back).set_plane_size(1.0);
    app.render_pick_frame();

    let (x, y) = pixel_of(app.engine.as_ref().expect("engine"), target);
    let screen = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
    let square_hit = Some(structure_hit(
        "SurfaceMesh",
        "zoom_square",
        0,
        PickElementType::Face,
    ));
    assert_eq!(app.click_hit_at(screen), square_hit);

    // A scroll moves the camera toward the scene, by more than the distance
    // from the square to the plane. The window still shows the frame before.
    app.engine.as_mut().expect("engine").camera.zoom_scroll(2.0);
    assert_eq!(app.click_hit_at(screen), square_hit);
}

/// The selection checks the element of a click when it applies the click. A
/// point cloud that loses points during the double-click time is not selected
/// with a point that it no longer has.
#[test]
fn test_click_selection_checks_the_element() {
    use crate::app::{App, ClickHit};
    use polyscope_render::PickElementType;
    use std::time::{Duration, Instant};

    let _guard = setup();
    deselect_structure();
    let name = unique_name("shrink_pc");
    register_point_cloud(&name, vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let hit = |element_index| {
        Some(ClickHit::Structure {
            type_name: "PointCloud".to_owned(),
            name: name.clone(),
            element_index,
            element_type: PickElementType::Point,
        })
    };
    let screen = glam::Vec2::new(10.0, 20.0);
    let later = || Instant::now() + Duration::from_secs(1);
    let mut app = App::new();

    // The point cloud loses two points before the click applies.
    app.queue_click_pick(hit(2), screen);
    with_point_cloud(&name, |pc| pc.update_points(vec![Vec3::ZERO]));
    app.update_pending_pick(later());
    assert!(app.selection.is_none());
    assert!(with_context(|ctx| ctx.selected_structure().is_none()));

    // A point that it still has is selected.
    app.queue_click_pick(hit(0), screen);
    app.update_pending_pick(later());
    let selection = app.selection.as_ref().expect("selection of point 0");
    assert_eq!(selection.element_index, 0);
}
