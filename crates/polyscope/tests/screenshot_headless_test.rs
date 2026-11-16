//! Headless test: a transparent-background screenshot has alpha 0 where only
//! the background shows and keeps the alpha of the scene elsewhere.
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

#[test]
fn transparent_background_screenshot() {
    let _ = init();
    remove_all_structures();

    // A quad that faces the auto-fitted camera and covers the image center
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces: Vec<[u32; 3]> = vec![[0, 1, 2], [0, 2, 3]];
    let quad = register_surface_mesh("quad", vertices, faces);

    let Ok(opaque) = render_to_image(W, H) else {
        eprintln!("Skipping headless screenshot test: no GPU adapter available");
        return;
    };
    let transparent_options = ScreenshotOptions {
        transparent_background: true,
    };
    let transparent =
        render_to_image_with_options(W, H, &transparent_options).expect("transparent render");

    // Background: opaque by default, alpha 0 with the option
    assert_eq!(pixel(&opaque, 2, 2)[3], 255);
    assert_eq!(pixel(&transparent, 2, 2)[3], 0);

    // Opaque scene pixels keep alpha 1 and their color
    let center = (W / 2, H / 2);
    assert_eq!(pixel(&transparent, center.0, center.1)[3], 255);
    assert_eq!(
        pixel(&opaque, center.0, center.1),
        pixel(&transparent, center.0, center.1),
        "the option must not change the scene color"
    );

    // The ground plane is part of the scene and stays opaque near the scene,
    // as in C++ Polyscope. Without it, the bottom row is background.
    let bottom = (W / 2, H - 2);
    assert_eq!(pixel(&transparent, bottom.0, bottom.1)[3], 255);
    with_context_mut(|ctx| ctx.options.ground_plane.mode = GroundPlaneMode::None);
    let no_ground =
        render_to_image_with_options(W, H, &transparent_options).expect("render without ground");
    assert_eq!(pixel(&no_ground, bottom.0, bottom.1)[3], 0);
    with_context_mut(|ctx| ctx.options.ground_plane.mode = GroundPlaneMode::default());

    // A half-transparent mesh keeps its alpha over the transparent background
    quad.set_transparency(0.5);
    let half = render_to_image_with_options(W, H, &transparent_options).expect("half render");
    let alpha = pixel(&half, center.0, center.1)[3];
    assert!(
        (110..=145).contains(&alpha),
        "mesh with transparency 0.5 must give alpha near 128, got {alpha}"
    );

    remove_all_structures();
}
