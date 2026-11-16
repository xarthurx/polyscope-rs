//! Headless test: `render_to_file` writes the colour channels in RGBA order.
//!
//! The test renders a red quad, decodes the PNG file and checks that red
//! dominates the quad pixels. One test function, because the global context
//! can only be initialized once per process.

use polyscope_rs::*;

const W: u32 = 160;
const H: u32 = 120;

#[test]
fn render_to_file_keeps_red_red() {
    let _ = init();
    remove_all_structures();

    // A red quad that faces the auto-fitted camera and covers the image center
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces: Vec<[u32; 3]> = vec![[0, 1, 2], [0, 2, 3]];
    register_surface_mesh("red_quad", vertices, faces).set_surface_color(Vec3::new(1.0, 0.0, 0.0));

    let Ok(pixels) = render_to_image(W, H) else {
        eprintln!("Skipping render_to_file channel test: no GPU adapter available");
        return;
    };

    let path = std::env::temp_dir().join(format!(
        "polyscope_render_file_channels_{}.png",
        std::process::id()
    ));
    render_to_file(path.to_str().expect("UTF-8 temp path"), W, H).expect("render_to_file");
    let decoded = image::open(&path).expect("decode PNG").to_rgba8();
    let _ = std::fs::remove_file(&path);
    assert_eq!(decoded.dimensions(), (W, H));

    let center = decoded.get_pixel(W / 2, H / 2).0;
    let [r, g, b, a] = center;
    assert!(
        u16::from(r) > u16::from(g) + 60 && u16::from(r) > u16::from(b) + 60,
        "the red quad must be red in the file, got {center:?}"
    );
    assert_eq!(a, 255, "an opaque quad must be opaque in the file");

    // The file holds the same pixels as the RGBA buffer.
    let i = ((H / 2 * W + W / 2) * 4) as usize;
    assert_eq!(
        center,
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]],
        "render_to_file and render_to_image must agree"
    );
    assert_eq!(
        decoded.into_raw(),
        pixels,
        "render_to_file must write the render_to_image buffer"
    );

    remove_all_structures();
}
