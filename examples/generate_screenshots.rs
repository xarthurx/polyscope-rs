#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
//! Generate headless screenshots for the README gallery.
//!
//! Run with: cargo run --example generate_screenshots
//!
//! Outputs PNG files to docs/images/

use glam::{UVec3, Vec3};
use polyscope::Structure;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufRead, BufReader};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const OUT_DIR: &str = "docs/images";

// ─── Geometry helpers ───────────────────────────────────────────────────────

fn create_icosphere(subdivisions: u32) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let mut vertices: Vec<Vec3> = vec![
        Vec3::new(-1.0, t, 0.0),
        Vec3::new(1.0, t, 0.0),
        Vec3::new(-1.0, -t, 0.0),
        Vec3::new(1.0, -t, 0.0),
        Vec3::new(0.0, -1.0, t),
        Vec3::new(0.0, 1.0, t),
        Vec3::new(0.0, -1.0, -t),
        Vec3::new(0.0, 1.0, -t),
        Vec3::new(t, 0.0, -1.0),
        Vec3::new(t, 0.0, 1.0),
        Vec3::new(-t, 0.0, -1.0),
        Vec3::new(-t, 0.0, 1.0),
    ];
    for v in &mut vertices {
        *v = v.normalize();
    }

    let mut faces: Vec<glam::UVec3> = vec![
        glam::UVec3::new(0, 11, 5),
        glam::UVec3::new(0, 5, 1),
        glam::UVec3::new(0, 1, 7),
        glam::UVec3::new(0, 7, 10),
        glam::UVec3::new(0, 10, 11),
        glam::UVec3::new(1, 5, 9),
        glam::UVec3::new(5, 11, 4),
        glam::UVec3::new(11, 10, 2),
        glam::UVec3::new(10, 7, 6),
        glam::UVec3::new(7, 1, 8),
        glam::UVec3::new(3, 9, 4),
        glam::UVec3::new(3, 4, 2),
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 8),
        glam::UVec3::new(3, 8, 9),
        glam::UVec3::new(4, 9, 5),
        glam::UVec3::new(2, 4, 11),
        glam::UVec3::new(6, 2, 10),
        glam::UVec3::new(8, 6, 7),
        glam::UVec3::new(9, 8, 1),
    ];

    let mut midpoint_cache: HashMap<(u32, u32), u32> = HashMap::new();
    for _ in 0..subdivisions {
        let mut new_faces = Vec::new();
        midpoint_cache.clear();
        for face in &faces {
            let a = get_midpoint(face.x, face.y, &mut vertices, &mut midpoint_cache);
            let b = get_midpoint(face.y, face.z, &mut vertices, &mut midpoint_cache);
            let c = get_midpoint(face.z, face.x, &mut vertices, &mut midpoint_cache);
            new_faces.push(glam::UVec3::new(face.x, a, c));
            new_faces.push(glam::UVec3::new(face.y, b, a));
            new_faces.push(glam::UVec3::new(face.z, c, b));
            new_faces.push(glam::UVec3::new(a, b, c));
        }
        faces = new_faces;
    }
    (vertices, faces)
}

fn get_midpoint(
    i0: u32,
    i1: u32,
    vertices: &mut Vec<Vec3>,
    cache: &mut HashMap<(u32, u32), u32>,
) -> u32 {
    let key = if i0 < i1 { (i0, i1) } else { (i1, i0) };
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }
    let mid = (vertices[i0 as usize] + vertices[i1 as usize]).normalize();
    let idx = vertices.len() as u32;
    vertices.push(mid);
    cache.insert(key, idx);
    idx
}

fn load_obj(path: &str) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let (models, _) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for model in models {
        let mesh = model.mesh;
        let offset = vertices.len() as u32;
        for i in (0..mesh.positions.len()).step_by(3) {
            vertices.push(Vec3::new(
                mesh.positions[i],
                mesh.positions[i + 1],
                mesh.positions[i + 2],
            ));
        }
        for i in (0..mesh.indices.len()).step_by(3) {
            faces.push(glam::UVec3::new(
                mesh.indices[i] + offset,
                mesh.indices[i + 1] + offset,
                mesh.indices[i + 2] + offset,
            ));
        }
    }
    (vertices, faces)
}

fn normalize_mesh(vertices: &mut [Vec3], target_size: f32) {
    if vertices.is_empty() {
        return;
    }
    let min = vertices.iter().copied().reduce(Vec3::min).unwrap();
    let max = vertices.iter().copied().reduce(Vec3::max).unwrap();
    let center = (min + max) * 0.5;
    let extent = max - min;
    let max_extent = extent.x.max(extent.y).max(extent.z);
    let scale = target_size / max_extent;
    for v in vertices.iter_mut() {
        *v = (*v - center) * scale;
    }
}

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
                                tets.push([parts[0] - 1, parts[1] - 1, parts[2] - 1, parts[3] - 1]);
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

// ─── Screenshot scenes ──────────────────────────────────────────────────────

/// Screenshot 1: Materials - 4 icospheres with different materials
fn scene_materials() {
    polyscope::remove_all_structures();

    let (sphere_verts, sphere_faces) = create_icosphere(3);
    let materials = ["clay", "wax", "candy", "ceramic"];
    let colors = [
        Vec3::new(0.55, 0.55, 0.55),
        Vec3::new(0.85, 0.75, 0.55),
        Vec3::new(0.90, 0.40, 0.50),
        Vec3::new(0.80, 0.85, 0.90),
    ];

    for (i, &mat) in materials.iter().enumerate() {
        let x = (i as f32 - 1.5) * 2.5;
        let verts: Vec<Vec3> = sphere_verts.iter().map(|v| *v + Vec3::new(x, 0.0, 0.0)).collect();
        let name = format!("sphere_{mat}");
        polyscope::register_surface_mesh(&name, verts, sphere_faces.clone());
        polyscope::with_surface_mesh(&name, |mesh| {
            mesh.set_surface_color(colors[i]);
            mesh.set_material(mat);
        });
    }
}

/// Screenshot 2: Point cloud with color quantity
fn scene_point_cloud() {
    polyscope::remove_all_structures();

    // Fibonacci sphere + spiral
    let n = 2000;
    let mut points = Vec::with_capacity(n);
    let mut colors = Vec::with_capacity(n);
    let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;

    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let theta = 2.0 * PI * i as f32 / golden_ratio;
        let phi = (1.0 - 2.0 * t).acos();
        let r = 1.0 + 0.3 * (8.0 * PI * t).sin();
        points.push(Vec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            r * phi.cos(),
        ));
        // Color by position
        colors.push(Vec3::new(
            0.5 + 0.5 * phi.sin() * theta.cos(),
            0.5 + 0.5 * phi.cos(),
            0.5 + 0.5 * phi.sin() * theta.sin(),
        ));
    }

    let pc = polyscope::register_point_cloud("fibonacci_sphere", points);
    pc.add_color_quantity("position_color", colors);
    polyscope::with_point_cloud("fibonacci_sphere", |pc| {
        pc.set_point_radius(0.008);
    });
}

/// Screenshot 3: Surface mesh (Spot cow) with vertex color
fn scene_surface_mesh() {
    polyscope::remove_all_structures();

    let (mut verts, faces) = load_obj("assets/spot.obj");
    normalize_mesh(&mut verts, 2.0);

    // Height-based vertex colors
    let min_y = verts.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let max_y = verts.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    let colors: Vec<Vec3> = verts
        .iter()
        .map(|v| {
            let t = (v.y - min_y) / (max_y - min_y);
            // Blue to yellow gradient
            Vec3::new(t, 0.4 + 0.4 * t, 1.0 - 0.8 * t)
        })
        .collect();

    let handle = polyscope::register_surface_mesh("spot", verts, faces);
    handle.add_vertex_color_quantity("height_color", colors);
}

/// Screenshot 4: Curve network
fn scene_curve_network() {
    polyscope::remove_all_structures();

    // Trefoil knot
    let n = 300;
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let t = 2.0 * PI * i as f32 / n as f32;
        let x = (2.0 + (3.0 * t).cos()) * t.cos();
        let y = (2.0 + (3.0 * t).cos()) * t.sin();
        let z = (3.0 * t).sin();
        nodes.push(Vec3::new(x, y, z));
    }
    polyscope::register_curve_network_line("trefoil_knot", nodes.clone());
    polyscope::with_curve_network("trefoil_knot", |cn| {
        cn.set_color(Vec3::new(0.9, 0.4, 0.1));
        cn.set_radius(0.08, true);
        cn.set_material("candy");
    });

    // A second helix curve offset to the side
    let m = 200;
    let mut helix = Vec::with_capacity(m);
    for i in 0..m {
        let t = i as f32 / m as f32;
        let angle = 6.0 * PI * t;
        helix.push(Vec3::new(
            5.0 + 0.8 * angle.cos(),
            (t - 0.5) * 6.0,
            0.8 * angle.sin(),
        ));
    }
    polyscope::register_curve_network_line("helix", helix);
    polyscope::with_curve_network("helix", |cn| {
        cn.set_color(Vec3::new(0.2, 0.6, 0.9));
        cn.set_radius(0.06, true);
        cn.set_material("ceramic");
    });
}

/// Screenshot 5: Volume mesh (bunny)
fn scene_volume_mesh() {
    polyscope::remove_all_structures();

    if let Some((mut verts, tets)) = load_mesh_file("assets/bunny.mesh") {
        // Normalize to a good size
        normalize_mesh_f32(&mut verts, 2.0);

        let min_y = verts.iter().map(|v| v.y).fold(f32::MAX, f32::min);
        let max_y = verts.iter().map(|v| v.y).fold(f32::MIN, f32::max);
        let scalars: Vec<f32> = verts
            .iter()
            .map(|v| (v.y - min_y) / (max_y - min_y))
            .collect();

        let mesh = polyscope::register_tet_mesh("bunny", verts, tets);
        mesh.set_color(Vec3::new(0.2, 0.5, 0.8));
        mesh.set_edge_width(0.5);
        polyscope::with_volume_mesh("bunny", |m| {
            m.add_vertex_scalar_quantity("height", scalars);
        });
    }
}

/// Screenshot 6: Volume grid with scalar field
fn scene_volume_grid() {
    polyscope::remove_all_structures();

    let dim = UVec3::new(20, 20, 20);
    let bound_min = Vec3::new(-2.0, -2.0, -2.0);
    let bound_max = Vec3::new(2.0, 2.0, 2.0);

    let vg = polyscope::register_volume_grid("scalar_field", dim, bound_min, bound_max);

    // Scalar field: distance-based function with oscillation
    let nx = dim.x as usize;
    let ny = dim.y as usize;
    let nz = dim.z as usize;
    let mut values = Vec::with_capacity((nx + 1) * (ny + 1) * (nz + 1));
    for k in 0..=nz {
        for j in 0..=ny {
            for i in 0..=nx {
                let x = bound_min.x + (i as f32 / nx as f32) * (bound_max.x - bound_min.x);
                let y = bound_min.y + (j as f32 / ny as f32) * (bound_max.y - bound_min.y);
                let z = bound_min.z + (k as f32 / nz as f32) * (bound_max.z - bound_min.z);
                let r = (x * x + y * y + z * z).sqrt();
                let val = (3.0 * r).sin() / (r + 0.1);
                values.push(val);
            }
        }
    }

    vg.add_node_scalar_quantity("sinc_field", values);
}

fn normalize_mesh_f32(vertices: &mut [Vec3], target_size: f32) {
    if vertices.is_empty() {
        return;
    }
    let min = vertices.iter().copied().reduce(Vec3::min).unwrap();
    let max = vertices.iter().copied().reduce(Vec3::max).unwrap();
    let center = (min + max) * 0.5;
    let extent = max - min;
    let max_extent = extent.x.max(extent.y).max(extent.z);
    let scale = target_size / max_extent;
    for v in vertices.iter_mut() {
        *v = (*v - center) * scale;
    }
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    std::fs::create_dir_all(OUT_DIR).expect("Failed to create output directory");

    let scenes: Vec<(&str, fn())> = vec![
        ("screenshot_materials", scene_materials),
        ("screenshot_point_cloud", scene_point_cloud),
        ("screenshot_surface_mesh", scene_surface_mesh),
        ("screenshot_curve_network", scene_curve_network),
        ("screenshot_volume_mesh", scene_volume_mesh),
        ("screenshot_volume_grid", scene_volume_grid),
    ];

    for (name, setup) in &scenes {
        print!("Rendering {name}... ");
        setup();
        let path = format!("{OUT_DIR}/{name}.png");
        match polyscope::render_to_file(&path, WIDTH, HEIGHT) {
            Ok(()) => println!("OK -> {path}"),
            Err(e) => println!("FAILED: {e}"),
        }
    }

    println!("\nDone! Screenshots saved to {OUT_DIR}/");
}
