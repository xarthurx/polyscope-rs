//! Headless tests: the render-look options drive the frame, and view states
//! round-trip through the options.
//!
//! One test function, because the global context can only be initialized
//! once per process.

use polyscope_rs::*;

const W: u32 = 160;
const H: u32 = 120;

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn set_options(f: impl FnOnce(&mut Options)) {
    with_context_mut(|ctx| f(&mut ctx.options));
}

/// Two parallel quads in one mesh, the near quad first in the index buffer.
fn register_two_layer_mesh() {
    let vertices = vec![
        // near quad (z = 0.5)
        Vec3::new(0.0, 0.0, 0.5),
        Vec3::new(1.0, 0.0, 0.5),
        Vec3::new(1.0, 1.0, 0.5),
        Vec3::new(0.0, 1.0, 0.5),
        // far quad (z = 0.0), shifted so that the quads overlap partly
        Vec3::new(0.3, 0.3, 0.0),
        Vec3::new(1.3, 0.3, 0.0),
        Vec3::new(1.3, 1.3, 0.0),
        Vec3::new(0.3, 1.3, 0.0),
    ];
    let faces: Vec<[u32; 3]> = vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]];
    register_surface_mesh("layers", vertices, faces).set_transparency(0.5);
}

#[test]
fn options_drive_headless_frame() {
    let _ = init();
    remove_all_structures();

    // --- Background color ---
    let Ok(default_bg) = render_to_image(W, H) else {
        eprintln!("Skipping headless options test: no GPU adapter available");
        return;
    };
    set_options(|o| o.background_color = Vec3::new(1.0, 0.0, 0.0));
    let red_bg = render_to_image(W, H).expect("render with red background");
    let (before, after) = (pixel(&default_bg, 5, 5), pixel(&red_bg, 5, 5));
    assert_ne!(
        before, after,
        "background color option must change the background"
    );
    assert!(
        after[0] > 200 && after[1] < 30 && after[2] < 30,
        "background must be red (RGBA), got {after:?}"
    );
    set_options(|o| o.background_color = Options::default().background_color);

    // --- SSAA factor ---
    register_two_layer_mesh();
    let ssaa_1 = render_to_image(W, H).expect("render with SSAA 1");
    set_options(|o| o.ssaa_factor = 2);
    let ssaa_2 = render_to_image(W, H).expect("render with SSAA 2");
    assert_eq!(
        ssaa_2.len(),
        (W * H * 4) as usize,
        "SSAA keeps the output size"
    );
    assert_ne!(ssaa_1, ssaa_2, "SSAA factor 2 must change the edge pixels");
    set_options(|o| o.ssaa_factor = 1);
    let ssaa_1_again = render_to_image(W, H).expect("render with SSAA 1 again");
    assert_eq!(
        ssaa_1, ssaa_1_again,
        "SSAA factor 1 must restore the output"
    );

    // --- Transparency mode ---
    // Depth peeling separates two layers only if their depth values differ
    // enough. Put the camera close to the quads, with a large near plane.
    let mut close_view = current_view_state().expect("snapshot after render");
    close_view.camera.position = [0.65, 0.65, 2.5];
    close_view.camera.target = [0.65, 0.65, 0.0];
    close_view.camera.near = 0.5;
    close_view.camera.far = 100.0;
    let render_with_mode = |mode: TransparencyMode| {
        set_options(|o| o.transparency_mode = mode);
        // The view state moves the camera and keeps the render look of the options.
        let mut view = close_view.clone();
        view.render = with_context(|ctx| RenderState::from_options(&ctx.options));
        apply_view_state(&view, ViewTransition::Instant).expect("queue view state");
        render_to_image(W, H).expect("render with transparency mode")
    };
    let simple = render_with_mode(TransparencyMode::Simple);
    let pretty = render_with_mode(TransparencyMode::Pretty);
    // The image center shows both quads. Simple blending with depth writes
    // hides the far quad (drawn second); depth peeling shows it.
    assert_ne!(
        pixel(&simple, W / 2, H / 2),
        pixel(&pretty, W / 2, H / 2),
        "depth peeling must show the far layer behind the near layer"
    );

    // --- View-state round trip ---
    let custom = Options {
        background_color: Vec3::new(0.2, 0.4, 0.6),
        ground_plane: GroundPlaneConfig {
            mode: GroundPlaneMode::ShadowOnly,
            height: -0.5,
            height_is_relative: false,
            shadow_darkness: 0.6,
            reflection_intensity: 0.3,
        },
        transparency_mode: TransparencyMode::Pretty,
        transparency_render_passes: 4,
        ssaa_factor: 2,
        ssao: SsaoConfig {
            enabled: true,
            ..SsaoConfig::default()
        },
        ..Options::default()
    };
    let custom_render = RenderState::from_options(&custom);
    set_options(|o| custom_render.apply_to_options(o));
    let _ = render_to_image(W, H).expect("render with custom options");
    let saved = current_view_state().expect("snapshot after render");
    assert_eq!(
        saved.render, custom_render,
        "snapshot must report the options"
    );

    let mut other = saved.clone();
    other.render.background_color = [0.9, 0.8, 0.7];
    other.render.ground_plane.mode = GroundPlaneMode::None;
    other.render.transparency.mode = TransparencyMode::Simple;
    other.render.transparency.render_passes = 8;
    other.render.ssaa_factor = 1;
    other.render.ssao.enabled = false;
    apply_view_state(&other, ViewTransition::Instant).expect("queue view state");
    let applied = render_to_image(W, H).expect("render with applied view state");
    let options_render = with_context(|ctx| RenderState::from_options(&ctx.options));
    assert_eq!(
        options_render, other.render,
        "options must hold the applied state"
    );
    assert_eq!(
        current_view_state().expect("snapshot").render,
        other.render,
        "snapshot must report the applied state"
    );
    let bg = pixel(&applied, 5, 5);
    assert!(
        bg[0] > bg[1] && bg[1] > bg[2],
        "applied background (0.9, 0.8, 0.7) must reach the frame, got {bg:?}"
    );

    remove_all_structures();
    set_options(|o| *o = Options::default());
}
