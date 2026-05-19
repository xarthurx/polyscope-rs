//! Demonstrates all four volume-mesh cell types side by side: tet, hex,
//! prism (wedge), and pyramid. Each cell is rendered as its own mesh so
//! the structure list shows them separately.
//!
//! Run with: cargo run --example `volume_mesh_mixed_cells_demo`

use glam::Vec3;
use polyscope_rs::{
    Result, init, register_hex_mesh, register_prism_mesh, register_pyramid_mesh,
    register_tet_mesh, show,
};

fn main() -> Result<()> {
    env_logger::init();
    init()?;

    // ----- Tet -----
    let tet_verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tet = register_tet_mesh("tet", tet_verts, vec![[0, 1, 2, 3]]);
    tet.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 1.0]);

    // ----- Hex (translated +3 in x) -----
    let dx = Vec3::new(3.0, 0.0, 0.0);
    let hex_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 1.0, 1.0) + dx,
        Vec3::new(0.0, 1.0, 1.0) + dx,
    ];
    let hex = register_hex_mesh("hex", hex_verts, vec![[0, 1, 2, 3, 4, 5, 6, 7]]);
    hex.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);

    // ----- Prism (translated +6 in x) -----
    let dx = Vec3::new(6.0, 0.0, 0.0);
    let prism_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(0.5, 1.0, 0.0) + dx,
        Vec3::new(0.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 0.0, 1.0) + dx,
        Vec3::new(0.5, 1.0, 1.0) + dx,
    ];
    let prism = register_prism_mesh("prism", prism_verts, vec![[0, 1, 2, 3, 4, 5]]);
    prism.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

    // ----- Pyramid (translated +9 in x) -----
    let dx = Vec3::new(9.0, 0.0, 0.0);
    let pyr_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 1.0, 0.0) + dx,
        Vec3::new(0.5, 0.5, 1.0) + dx,
    ];
    let pyr = register_pyramid_mesh("pyramid", pyr_verts, vec![[0, 1, 2, 3, 4]]);
    pyr.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 0.0, 1.0]);

    show();
    Ok(())
}
