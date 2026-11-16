//! Headless tests for scene content that the window draws: curve network
//! tubes and node spheres, volume grid gridcubes and the ground plane.
//!
//! Headless rendering uses the scene passes of the window, so these images
//! show what the window shows. One test function, because the global context
//! can only be initialized once per process.

use polyscope_rs::*;

const W: u32 = 160;
const H: u32 = 120;

fn render() -> Vec<u8> {
    render_to_image(W, H).expect("headless render")
}

fn set_ground_plane(mode: GroundPlaneMode) {
    with_context_mut(|ctx| ctx.options.ground_plane.mode = mode);
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Returns the largest channel difference of two pixels.
fn pixel_diff(a: [u8; 4], b: [u8; 4]) -> u8 {
    a.iter()
        .zip(&b)
        .map(|(p, q)| p.abs_diff(*q))
        .max()
        .unwrap_or(0)
}

/// Returns the number of pixels that differ from `background` by more than 8 levels.
fn count_non_background(pixels: &[u8], background: [u8; 4]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| pixel_diff(**p, background) > 8)
        .count()
}

/// Returns the number of pixels that differ by more than `threshold` levels.
fn count_changed(a: &[u8], b: &[u8], threshold: u8) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(p, q)| pixel_diff(**p, **q) > threshold)
        .count()
}

/// Returns the number of strongly colored pixels (channel spread above 40 levels).
fn count_colored(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| {
            let rgb = &p[..3];
            rgb.iter().max().unwrap() - rgb.iter().min().unwrap() > 40
        })
        .count()
}

fn square() -> Vec<Vec3> {
    vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]
}

/// The background color of the image, read from an empty scene.
fn background_pixel() -> [u8; 4] {
    remove_all_structures();
    pixel(&render(), W / 2, H / 2)
}

/// A curve network in tube mode with a relative radius covers far more
/// pixels than the same network in line mode (one-pixel lines).
fn check_tubes_cover_more_than_lines(background: [u8; 4]) {
    remove_all_structures();
    register_curve_network_loop("loop", square());
    with_curve_network("loop", |cn| {
        cn.set_radius(0.03, true);
    });
    let lines = count_non_background(&render(), background);
    with_curve_network("loop", |cn| {
        cn.set_render_mode(1);
    });
    let tubes = count_non_background(&render(), background);
    assert!(lines > 0, "line mode must draw the curve network");
    assert!(
        tubes > 3 * lines,
        "tubes must cover far more pixels than lines: {tubes} vs {lines}"
    );
}

/// An enabled node scalar quantity on a volume grid draws colored gridcubes.
fn check_volume_grid_gridcubes() {
    remove_all_structures();
    let grid = register_volume_grid("grid", glam::UVec3::splat(4), Vec3::splat(-1.0), Vec3::ONE);
    grid.add_node_scalar_quantity("s", (0..64u8).map(|i| f32::from(i) / 63.0).collect());
    with_volume_grid("grid", |vg| {
        vg.get_quantity_mut("s")
            .expect("quantity")
            .set_enabled(false);
    });
    let disabled = render();
    with_volume_grid("grid", |vg| {
        vg.get_quantity_mut("s")
            .expect("quantity")
            .set_enabled(true);
    });
    let enabled = render();
    assert_eq!(
        count_colored(&disabled),
        0,
        "a grid without an enabled quantity has no colored pixels"
    );
    assert!(
        count_colored(&enabled) > 50,
        "the enabled node scalar quantity must draw colored gridcubes, got {} pixels",
        count_colored(&enabled)
    );
}

/// The ground plane shows with `GroundPlaneMode::Tile` and not with `None`.
fn check_ground_plane(background: [u8; 4]) {
    remove_all_structures();
    register_surface_mesh(
        "tetra",
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ],
        vec![[0u32, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]],
    );
    // The bottom rows of the image show the ground below the mesh.
    let bottom_rows = |pixels: &[u8]| {
        (H - 8..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel_diff(pixel(pixels, x, y), background) > 8)
            .count()
    };
    set_ground_plane(GroundPlaneMode::Tile);
    let tile = bottom_rows(&render());
    set_ground_plane(GroundPlaneMode::None);
    let none = bottom_rows(&render());
    let total = (8 * W) as usize;
    assert!(
        tile > total * 9 / 10,
        "the tile ground plane must cover the bottom rows, got {tile} of {total}"
    );
    assert_eq!(
        none, 0,
        "without a ground plane the bottom rows show the background"
    );
}

/// A transformed curve network in tube mode draws its node spheres on its
/// tube ends: it equals an untransformed network with moved nodes.
///
/// Both renders use the same camera and no ground plane, so only the
/// curve network can differ.
fn check_transformed_node_spheres() {
    let offset = Vec3::new(0.6, 0.3, 0.0);
    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
    ];
    let edges = vec![[0, 1], [1, 2]];
    let setup = |cn: &mut CurveNetwork| {
        cn.set_render_mode(1);
        cn.set_radius(0.08, false);
    };

    remove_all_structures();
    let moved_nodes: Vec<Vec3> = nodes.iter().map(|p| *p + offset).collect();
    register_curve_network("curve", moved_nodes, edges.clone());
    with_curve_network("curve", setup);
    let reference = render();
    let view = current_view_state().expect("view state after a render");

    remove_all_structures();
    register_curve_network("curve", nodes, edges);
    with_curve_network("curve", |cn| {
        setup(cn);
        cn.set_transform(Mat4::from_translation(offset));
    });
    apply_view_state(&view, ViewTransition::Instant).expect("queue view state");
    let transformed = render();

    let changed = count_changed(&transformed, &reference, 16);
    assert!(
        changed < 20,
        "the node spheres of a transformed curve network must sit on its tube ends; \
         {changed} pixels differ from the untransformed reference"
    );
}

#[test]
fn headless_scene_content() {
    let _ = init();
    remove_all_structures();
    let Ok(_) = render_to_image(W, H) else {
        eprintln!("Skipping headless scene content test: no GPU adapter available");
        return;
    };

    // The default background is white, like the tiles of the ground plane.
    // A dark background makes the ground plane visible against it.
    with_context_mut(|ctx| ctx.options.background_color = Vec3::splat(0.1));
    check_ground_plane(background_pixel());
    with_context_mut(|ctx| ctx.options.background_color = Options::default().background_color);
    set_ground_plane(GroundPlaneMode::None);
    let background = background_pixel();
    check_tubes_cover_more_than_lines(background);
    check_volume_grid_gridcubes();
    check_transformed_node_spheres();
    set_ground_plane(GroundPlaneMode::default());

    remove_all_structures();
}
