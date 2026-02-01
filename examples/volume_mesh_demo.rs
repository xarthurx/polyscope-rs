#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Volume mesh demonstration showcasing tet and hex mesh visualization.
//!
//! This demo showcases:
//! - Loading real tetrahedral meshes from MEDIT .mesh format files
//! - Tetrahedral and hexahedral mesh rendering
//! - Interior face detection (only exterior faces rendered)
//! - Vertex and cell scalar quantities
//! - Vertex and cell color quantities
//!
//! Mesh assets:
//! - `assets/p01.mesh`: Rectangular channel (584 vertices, 2568 tetrahedra)
//! - `assets/cyl248.mesh`: Cylinder (92 vertices, 248 tetrahedra)
//!
//! Run with: cargo run --example `volume_mesh_demo`

use glam::Vec3;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Load a MEDIT .mesh file and return vertices and tetrahedra.
///
/// MEDIT format:
/// ```text
/// MeshVersionFormatted 1
/// Dimension 3
/// Vertices
/// <num_vertices>
/// x y z ref
/// ...
/// Tetrahedra
/// <num_tets>
/// v0 v1 v2 v3 ref
/// ...
/// End
/// ```
fn load_mesh_file(path: &str) -> Option<(Vec<Vec3>, Vec<[u32; 4]>)> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines().map_while(Result::ok).peekable();

    let mut vertices = Vec::new();
    let mut tets = Vec::new();

    while let Some(line) = lines.next() {
        let line = line.trim();

        if line.eq_ignore_ascii_case("Vertices") {
            if let Some(count_line) = lines.next() {
                if let Ok(count) = count_line.trim().parse::<usize>() {
                    for _ in 0..count {
                        if let Some(v_line) = lines.next() {
                            let parts: Vec<f32> = v_line
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            if parts.len() >= 3 {
                                vertices.push(Vec3::new(parts[0], parts[1], parts[2]));
                            }
                        }
                    }
                }
            }
        } else if line.eq_ignore_ascii_case("Tetrahedra") {
            if let Some(count_line) = lines.next() {
                if let Ok(count) = count_line.trim().parse::<usize>() {
                    for _ in 0..count {
                        if let Some(t_line) = lines.next() {
                            let parts: Vec<u32> = t_line
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            if parts.len() >= 4 {
                                // MEDIT uses 1-based indexing
                                tets.push([
                                    parts[0] - 1,
                                    parts[1] - 1,
                                    parts[2] - 1,
                                    parts[3] - 1,
                                ]);
                            }
                        }
                    }
                }
            }
        }
    }

    if vertices.is_empty() || tets.is_empty() {
        None
    } else {
        Some((vertices, tets))
    }
}

/// Generate a subdivided octahedron tet mesh (fallback if no file available).
fn generate_subdivided_octahedron(
    center: Vec3,
    radius: f32,
    subdivisions: u32,
) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let mut vertices = Vec::new();
    let mut tets = Vec::new();

    let oct_verts = [
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];

    vertices.push(center);
    for v in oct_verts {
        vertices.push(center + v * radius);
    }

    let oct_faces = [
        [1, 3, 5],
        [1, 5, 4],
        [1, 4, 6],
        [1, 6, 3],
        [2, 5, 3],
        [2, 4, 5],
        [2, 6, 4],
        [2, 3, 6],
    ];

    for face in oct_faces {
        tets.push([0, face[0], face[1], face[2]]);
    }

    for _ in 0..subdivisions {
        let mut new_tets = Vec::new();
        let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

        for tet in &tets {
            let edges = [
                (tet[0], tet[1]),
                (tet[0], tet[2]),
                (tet[0], tet[3]),
                (tet[1], tet[2]),
                (tet[1], tet[3]),
                (tet[2], tet[3]),
            ];

            let mut mids = [0u32; 6];
            for (i, &(a, b)) in edges.iter().enumerate() {
                let key = if a < b { (a, b) } else { (b, a) };
                mids[i] = *edge_midpoints.entry(key).or_insert_with(|| {
                    let mid = (vertices[a as usize] + vertices[b as usize]) * 0.5;
                    vertices.push(mid);
                    (vertices.len() - 1) as u32
                });
            }

            new_tets.push([tet[0], mids[0], mids[1], mids[2]]);
            new_tets.push([tet[1], mids[0], mids[3], mids[4]]);
            new_tets.push([tet[2], mids[1], mids[3], mids[5]]);
            new_tets.push([tet[3], mids[2], mids[4], mids[5]]);
            new_tets.push([mids[0], mids[1], mids[3], mids[4]]);
            new_tets.push([mids[1], mids[2], mids[4], mids[5]]);
            new_tets.push([mids[1], mids[3], mids[4], mids[5]]);
            new_tets.push([mids[0], mids[1], mids[2], mids[4]]);
        }

        tets = new_tets;
    }

    (vertices, tets)
}

/// Generate a grid of hexahedra.
fn generate_hex_grid(
    origin: Vec3,
    size: Vec3,
    divisions: (u32, u32, u32),
) -> (Vec<Vec3>, Vec<[u32; 8]>) {
    let mut vertices = Vec::new();
    let mut hexes = Vec::new();

    let (nx, ny, nz) = divisions;
    let dx = size.x / nx as f32;
    let dy = size.y / ny as f32;
    let dz = size.z / nz as f32;

    for k in 0..=nz {
        for j in 0..=ny {
            for i in 0..=nx {
                vertices.push(origin + Vec3::new(i as f32 * dx, j as f32 * dy, k as f32 * dz));
            }
        }
    }

    let stride_x = 1u32;
    let stride_y = nx + 1;
    let stride_z = (nx + 1) * (ny + 1);

    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let v0 = k * stride_z + j * stride_y + i * stride_x;
                let v1 = v0 + stride_x;
                let v2 = v0 + stride_y + stride_x;
                let v3 = v0 + stride_y;
                let v4 = v0 + stride_z;
                let v5 = v4 + stride_x;
                let v6 = v4 + stride_y + stride_x;
                let v7 = v4 + stride_y;
                hexes.push([v0, v1, v2, v3, v4, v5, v6, v7]);
            }
        }
    }

    (vertices, hexes)
}

/// Add scalar and color quantities to a tet mesh.
fn add_tet_quantities(name: &str, vertices: &[Vec3], tets: &[[u32; 4]]) {
    let (min_bound, max_bound) = {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in vertices {
            min = min.min(*v);
            max = max.max(*v);
        }
        (min, max)
    };
    let extent = max_bound - min_bound;
    let center = (min_bound + max_bound) * 0.5;

    let vertex_heights: Vec<f32> = vertices
        .iter()
        .map(|v| (v.y - min_bound.y) / extent.y)
        .collect();
    polyscope::with_volume_mesh(name, |mesh| {
        mesh.add_vertex_scalar_quantity("height", vertex_heights);
    });

    let vertex_distances: Vec<f32> = vertices
        .iter()
        .map(|v| (*v - center).length() / extent.length())
        .collect();
    polyscope::with_volume_mesh(name, |mesh| {
        mesh.add_vertex_scalar_quantity("distance_from_center", vertex_distances);
    });

    let vertex_colors: Vec<Vec3> = vertices
        .iter()
        .map(|v| {
            Vec3::new(
                (v.x - min_bound.x) / extent.x,
                (v.y - min_bound.y) / extent.y,
                (v.z - min_bound.z) / extent.z,
            )
        })
        .collect();
    polyscope::with_volume_mesh(name, |mesh| {
        mesh.add_vertex_color_quantity("position_color", vertex_colors);
    });

    let cell_volumes: Vec<f32> = tets
        .iter()
        .map(|tet| {
            let v0 = vertices[tet[0] as usize];
            let v1 = vertices[tet[1] as usize];
            let v2 = vertices[tet[2] as usize];
            let v3 = vertices[tet[3] as usize];
            let a = v1 - v0;
            let b = v2 - v0;
            let c = v3 - v0;
            (a.dot(b.cross(c))).abs() / 6.0
        })
        .collect();
    let max_vol = cell_volumes.iter().copied().fold(0.0f32, f32::max);
    let cell_volumes_normalized: Vec<f32> = if max_vol > 0.0 {
        cell_volumes.iter().map(|v| v / max_vol).collect()
    } else {
        cell_volumes
    };
    polyscope::with_volume_mesh(name, |mesh| {
        mesh.add_cell_scalar_quantity("cell_volume", cell_volumes_normalized);
    });

    let cell_colors: Vec<Vec3> = tets
        .iter()
        .map(|tet| {
            let centroid = (vertices[tet[0] as usize]
                + vertices[tet[1] as usize]
                + vertices[tet[2] as usize]
                + vertices[tet[3] as usize])
                / 4.0;
            let t = (centroid.y - min_bound.y) / extent.y;
            Vec3::new(t, 0.2, 1.0 - t)
        })
        .collect();
    polyscope::with_volume_mesh(name, |mesh| {
        mesh.add_cell_color_quantity("centroid_height_color", cell_colors);
    });
}

#[allow(clippy::too_many_lines)]
fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Load primary tet mesh: p01.mesh (rectangular channel)
    let primary_path = "assets/p01.mesh";
    let (tet_vertices, tets, mesh_name) = if Path::new(primary_path).exists() {
        if let Some((v, t)) = load_mesh_file(primary_path) {
            println!(
                "Loaded {primary_path}: {} vertices, {} tets",
                v.len(),
                t.len()
            );
            (v, t, "channel")
        } else {
            println!("Failed to parse {primary_path}, using fallback");
            let (v, t) = generate_subdivided_octahedron(Vec3::ZERO, 1.0, 2);
            (v, t, "octahedron")
        }
    } else {
        println!("No {primary_path} found, using fallback");
        let (v, t) = generate_subdivided_octahedron(Vec3::ZERO, 1.0, 2);
        (v, t, "octahedron")
    };

    let tet_mesh = polyscope::register_tet_mesh(mesh_name, tet_vertices.clone(), tets.clone());
    tet_mesh.set_color(Vec3::new(0.2, 0.5, 0.8));
    tet_mesh.set_edge_width(0.5);
    add_tet_quantities(mesh_name, &tet_vertices, &tets);

    // Load secondary tet mesh: cyl248.mesh (cylinder)
    let secondary_path = "assets/cyl248.mesh";
    if Path::new(secondary_path).exists() {
        if let Some((cyl_verts, cyl_tets)) = load_mesh_file(secondary_path) {
            println!(
                "Loaded {secondary_path}: {} vertices, {} tets",
                cyl_verts.len(),
                cyl_tets.len()
            );

            let primary_max_x = tet_vertices
                .iter()
                .map(|v| v.x)
                .fold(f32::MIN, f32::max);
            let cyl_min_x = cyl_verts.iter().map(|v| v.x).fold(f32::MAX, f32::min);
            let offset_x = primary_max_x - cyl_min_x + 0.5;

            let cyl_verts_offset: Vec<Vec3> = cyl_verts
                .iter()
                .map(|v| *v + Vec3::new(offset_x, 0.0, 0.0))
                .collect();

            let cyl_mesh = polyscope::register_tet_mesh(
                "cylinder",
                cyl_verts_offset.clone(),
                cyl_tets.clone(),
            );
            cyl_mesh.set_color(Vec3::new(0.7, 0.4, 0.2));
            cyl_mesh.set_edge_width(0.5);
            add_tet_quantities("cylinder", &cyl_verts_offset, &cyl_tets);
        }
    }

    // Hex grid
    let hex_offset_x = tet_vertices
        .iter()
        .map(|v| v.x)
        .fold(f32::MIN, f32::max)
        + 3.0;
    let (hex_vertices, hexes) = generate_hex_grid(
        Vec3::new(hex_offset_x, 0.0, 0.0),
        Vec3::splat(1.0),
        (3, 3, 3),
    );

    let hex_mesh = polyscope::register_hex_mesh("hex_grid", hex_vertices.clone(), hexes.clone());
    hex_mesh.set_color(Vec3::new(0.8, 0.5, 0.2));
    hex_mesh.set_edge_width(1.0);

    let hex_cell_ids: Vec<f32> = (0..hexes.len())
        .map(|i| i as f32 / hexes.len() as f32)
        .collect();
    polyscope::with_volume_mesh("hex_grid", |mesh| {
        mesh.add_cell_scalar_quantity("cell_id", hex_cell_ids);
    });

    println!();
    println!("Volume Mesh Demo");
    println!("================");
    println!();
    println!("Structures:");
    println!(
        "  - {mesh_name}: {} vertices, {} tets (from {primary_path})",
        tet_vertices.len(),
        tets.len(),
    );
    println!(
        "  - hex_grid: {} vertices, {} hexes",
        hex_vertices.len(),
        hexes.len()
    );
    println!();
    println!("Quantities on tet meshes:");
    println!("  - height (vertex scalar): Y-coordinate normalized");
    println!("  - distance_from_center (vertex scalar): distance from centroid");
    println!("  - position_color (vertex color): RGB from XYZ position");
    println!("  - cell_volume (cell scalar): tetrahedral volume");
    println!("  - centroid_height_color (cell color): blue-red by height");
    println!();
    println!("Quantities on hex_grid:");
    println!("  - cell_id (cell scalar): cell index");
    println!();
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - Click structure in UI to see quantities");
    println!("  - ESC: Exit");

    polyscope::show();
}
