//! Volume mesh demonstration showcasing tet and hex mesh visualization.
//!
//! This demo showcases:
//! - Tetrahedral and hexahedral mesh rendering
//! - Interior face detection (only exterior faces rendered)
//! - Vertex and cell scalar quantities
//! - Vertex and cell color quantities
//! - Loading tet meshes from MEDIT .mesh format
//!
//! Run with: cargo run --example volume_mesh_demo
//!
//! To use with custom models (e.g., Armadillo):
//! 1. Download a .mesh file (MEDIT format) and place in assets/
//! 2. Modify the `mesh_path` variable below
//!
//! The demo includes procedurally generated meshes by default.

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
    let mut lines = reader.lines().filter_map(|l| l.ok()).peekable();

    let mut vertices = Vec::new();
    let mut tets = Vec::new();

    while let Some(line) = lines.next() {
        let line = line.trim();

        if line.eq_ignore_ascii_case("Vertices") {
            // Read vertex count
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
            // Read tet count
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

/// Generate a subdivided octahedron tet mesh.
/// This creates a nice spherical-ish shape with clear interior structure.
fn generate_subdivided_octahedron(center: Vec3, radius: f32, subdivisions: u32) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let mut vertices = Vec::new();
    let mut tets = Vec::new();

    // Start with octahedron vertices (6 vertices)
    let oct_verts = [
        Vec3::new(0.0, 1.0, 0.0),   // top
        Vec3::new(0.0, -1.0, 0.0),  // bottom
        Vec3::new(1.0, 0.0, 0.0),   // +x
        Vec3::new(-1.0, 0.0, 0.0),  // -x
        Vec3::new(0.0, 0.0, 1.0),   // +z
        Vec3::new(0.0, 0.0, -1.0),  // -z
    ];

    // Add center
    vertices.push(center);

    // Add octahedron vertices
    for v in oct_verts {
        vertices.push(center + v * radius);
    }

    // 8 faces of octahedron, each creates a tet with center
    let oct_faces = [
        [1, 3, 5], [1, 5, 4], [1, 4, 6], [1, 6, 3],  // top 4
        [2, 5, 3], [2, 4, 5], [2, 6, 4], [2, 3, 6],  // bottom 4
    ];

    // Create base tets from center to each face
    for face in oct_faces {
        tets.push([0, face[0], face[1], face[2]]);
    }

    // Subdivide each tet if requested
    for _ in 0..subdivisions {
        let mut new_tets = Vec::new();
        let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

        for tet in &tets {
            // Get midpoint of each edge (6 edges per tet)
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

            // Split into 8 smaller tets
            // Corner tets
            new_tets.push([tet[0], mids[0], mids[1], mids[2]]);
            new_tets.push([tet[1], mids[0], mids[3], mids[4]]);
            new_tets.push([tet[2], mids[1], mids[3], mids[5]]);
            new_tets.push([tet[3], mids[2], mids[4], mids[5]]);

            // Inner octahedron split into 4 tets
            new_tets.push([mids[0], mids[1], mids[3], mids[4]]);
            new_tets.push([mids[1], mids[2], mids[4], mids[5]]);
            new_tets.push([mids[1], mids[3], mids[4], mids[5]]);
            new_tets.push([mids[0], mids[1], mids[2], mids[4]]);
        }

        tets = new_tets;
    }

    (vertices, tets)
}

/// Generate a tet mesh bunny-like shape (ellipsoid with ears).
fn generate_bunny_tets(center: Vec3, scale: f32) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let mut vertices = Vec::new();
    let mut tets = Vec::new();

    // Body: subdivided octahedron scaled as ellipsoid
    let (body_verts, body_tets) = generate_subdivided_octahedron(center, scale, 2);

    // Scale to make ellipsoid (wider than tall)
    for v in body_verts {
        let local = v - center;
        let scaled = Vec3::new(local.x * 0.8, local.y * 1.0, local.z * 0.7);
        vertices.push(center + scaled);
    }

    for tet in body_tets {
        tets.push(tet);
    }

    // Add head (smaller sphere offset forward and up)
    let head_center = center + Vec3::new(0.0, scale * 0.6, scale * 0.5);
    let head_scale = scale * 0.5;
    let vertex_offset = vertices.len() as u32;

    let (head_verts, head_tets) = generate_subdivided_octahedron(head_center, head_scale, 1);

    for v in head_verts {
        vertices.push(v);
    }

    for tet in head_tets {
        tets.push([
            tet[0] + vertex_offset,
            tet[1] + vertex_offset,
            tet[2] + vertex_offset,
            tet[3] + vertex_offset,
        ]);
    }

    // Add ears (elongated tets)
    let ear_base_y = head_center.y + head_scale * 0.3;
    let ear_tip_y = head_center.y + head_scale * 1.5;

    for ear_x in [-0.3f32, 0.3f32] {
        let base = vertices.len() as u32;
        let ear_center_x = head_center.x + ear_x * scale;

        // Ear base vertices (triangle)
        vertices.push(Vec3::new(ear_center_x - 0.1 * scale, ear_base_y, head_center.z));
        vertices.push(Vec3::new(ear_center_x + 0.1 * scale, ear_base_y, head_center.z));
        vertices.push(Vec3::new(ear_center_x, ear_base_y, head_center.z - 0.1 * scale));
        // Ear tip
        vertices.push(Vec3::new(ear_center_x, ear_tip_y, head_center.z));

        tets.push([base, base + 1, base + 2, base + 3]);
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

    // Generate vertices
    for k in 0..=nz {
        for j in 0..=ny {
            for i in 0..=nx {
                vertices.push(origin + Vec3::new(i as f32 * dx, j as f32 * dy, k as f32 * dz));
            }
        }
    }

    // Generate hexes
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

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Try to load a custom mesh, or use procedural generation
    let mesh_path = "assets/armadillo.mesh";
    let (tet_vertices, tets, mesh_name) = if Path::new(mesh_path).exists() {
        match load_mesh_file(mesh_path) {
            Some((v, t)) => {
                println!("Loaded mesh: {} vertices, {} tets", v.len(), t.len());
                (v, t, "custom_mesh")
            }
            None => {
                println!("Failed to parse {}, using procedural mesh", mesh_path);
                let (v, t) = generate_bunny_tets(Vec3::ZERO, 1.0);
                (v, t, "tet_bunny")
            }
        }
    } else {
        println!("No custom mesh found. Generating procedural tet meshes...");
        let (v, t) = generate_bunny_tets(Vec3::ZERO, 1.0);
        (v, t, "tet_bunny")
    };

    // Register the tet mesh
    let tet_mesh = polyscope::register_tet_mesh(mesh_name, tet_vertices.clone(), tets.clone());
    tet_mesh.set_color(Vec3::new(0.2, 0.5, 0.8));
    tet_mesh.set_edge_width(0.5);

    // Compute bounding box for normalization
    let (min_bound, max_bound) = {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in &tet_vertices {
            min = min.min(*v);
            max = max.max(*v);
        }
        (min, max)
    };
    let extent = max_bound - min_bound;
    let center = (min_bound + max_bound) * 0.5;

    // Add vertex scalar quantity: height (Y coordinate normalized)
    let vertex_heights: Vec<f32> = tet_vertices
        .iter()
        .map(|v| (v.y - min_bound.y) / extent.y)
        .collect();

    polyscope::with_volume_mesh(mesh_name, |mesh| {
        mesh.add_vertex_scalar_quantity("height", vertex_heights.clone());
    });

    // Add vertex scalar quantity: distance from center
    let vertex_distances: Vec<f32> = tet_vertices
        .iter()
        .map(|v| (*v - center).length() / extent.length())
        .collect();

    polyscope::with_volume_mesh(mesh_name, |mesh| {
        mesh.add_vertex_scalar_quantity("distance_from_center", vertex_distances);
    });

    // Add vertex color quantity: RGB based on position
    let vertex_colors: Vec<Vec3> = tet_vertices
        .iter()
        .map(|v| {
            Vec3::new(
                (v.x - min_bound.x) / extent.x,
                (v.y - min_bound.y) / extent.y,
                (v.z - min_bound.z) / extent.z,
            )
        })
        .collect();

    polyscope::with_volume_mesh(mesh_name, |mesh| {
        mesh.add_vertex_color_quantity("position_color", vertex_colors);
    });

    // Add cell scalar quantity: cell volume (approximate)
    let cell_volumes: Vec<f32> = tets
        .iter()
        .map(|tet| {
            let v0 = tet_vertices[tet[0] as usize];
            let v1 = tet_vertices[tet[1] as usize];
            let v2 = tet_vertices[tet[2] as usize];
            let v3 = tet_vertices[tet[3] as usize];
            // Tet volume = |det([v1-v0, v2-v0, v3-v0])| / 6
            let a = v1 - v0;
            let b = v2 - v0;
            let c = v3 - v0;
            (a.dot(b.cross(c))).abs() / 6.0
        })
        .collect();

    // Normalize cell volumes for visualization
    let max_vol = cell_volumes.iter().cloned().fold(0.0f32, f32::max);
    let cell_volumes_normalized: Vec<f32> = cell_volumes.iter().map(|v| v / max_vol).collect();

    polyscope::with_volume_mesh(mesh_name, |mesh| {
        mesh.add_cell_scalar_quantity("cell_volume", cell_volumes_normalized);
    });

    // Add cell color quantity: color by centroid height
    let cell_colors: Vec<Vec3> = tets
        .iter()
        .map(|tet| {
            let centroid = (tet_vertices[tet[0] as usize]
                + tet_vertices[tet[1] as usize]
                + tet_vertices[tet[2] as usize]
                + tet_vertices[tet[3] as usize])
                / 4.0;
            let t = (centroid.y - min_bound.y) / extent.y;
            // Blue to red gradient
            Vec3::new(t, 0.2, 1.0 - t)
        })
        .collect();

    polyscope::with_volume_mesh(mesh_name, |mesh| {
        mesh.add_cell_color_quantity("centroid_height_color", cell_colors);
    });

    // Also create a hex grid to demonstrate hex mesh support
    let (hex_vertices, hexes) =
        generate_hex_grid(Vec3::new(extent.x * 1.5, 0.0, 0.0), Vec3::splat(1.0), (3, 3, 3));

    let hex_mesh = polyscope::register_hex_mesh("hex_grid", hex_vertices.clone(), hexes.clone());
    hex_mesh.set_color(Vec3::new(0.8, 0.5, 0.2));
    hex_mesh.set_edge_width(1.0);

    // Add cell scalar to hex mesh
    let hex_cell_ids: Vec<f32> = (0..hexes.len()).map(|i| i as f32 / hexes.len() as f32).collect();

    polyscope::with_volume_mesh("hex_grid", |mesh| {
        mesh.add_cell_scalar_quantity("cell_id", hex_cell_ids);
    });

    println!();
    println!("Volume Mesh Demo");
    println!("================");
    println!();
    println!("Structures:");
    println!("  - {}: {} vertices, {} tets", mesh_name, tet_vertices.len(), tets.len());
    println!("  - hex_grid: {} vertices, {} hexes", hex_vertices.len(), hexes.len());
    println!();
    println!("Quantities on {}:", mesh_name);
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
