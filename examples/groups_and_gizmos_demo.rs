#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
//! Groups and Gizmos demonstration.
//!
//! This demo shows:
//! - Creating groups to organize structures
//! - Hierarchical groups (parent/child)
//! - Gizmo modes (translate, rotate, scale)
//! - Gizmo coordinate spaces (world vs local)
//! - Selecting structures for manipulation
//!
//! Run with: cargo run --example `groups_and_gizmos_demo`

use glam::Vec3;
use std::f32::consts::PI;

/// Generate a simple cube mesh.
fn create_cube(center: Vec3, size: f32) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let half = size / 2.0;
    let vertices = vec![
        center + Vec3::new(-half, -half, -half),
        center + Vec3::new(half, -half, -half),
        center + Vec3::new(half, half, -half),
        center + Vec3::new(-half, half, -half),
        center + Vec3::new(-half, -half, half),
        center + Vec3::new(half, -half, half),
        center + Vec3::new(half, half, half),
        center + Vec3::new(-half, half, half),
    ];

    let faces = vec![
        glam::UVec3::new(0, 1, 2),
        glam::UVec3::new(0, 2, 3), // front
        glam::UVec3::new(5, 4, 7),
        glam::UVec3::new(5, 7, 6), // back
        glam::UVec3::new(3, 2, 6),
        glam::UVec3::new(3, 6, 7), // top
        glam::UVec3::new(4, 5, 1),
        glam::UVec3::new(4, 1, 0), // bottom
        glam::UVec3::new(1, 5, 6),
        glam::UVec3::new(1, 6, 2), // right
        glam::UVec3::new(4, 0, 3),
        glam::UVec3::new(4, 3, 7), // left
    ];

    (vertices, faces)
}

/// Generate a sphere point cloud.
fn create_sphere_points(center: Vec3, radius: f32, count: usize) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(count);
    let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;

    for i in 0..count {
        let theta = 2.0 * PI * i as f32 / golden_ratio;
        let phi = (1.0 - 2.0 * (i as f32 + 0.5) / count as f32).acos();
        points.push(
            center
                + Vec3::new(
                    radius * phi.sin() * theta.cos(),
                    radius * phi.sin() * theta.sin(),
                    radius * phi.cos(),
                ),
        );
    }
    points
}

/// Generate a helix curve.
fn create_helix(center: Vec3, radius: f32, height: f32, turns: f32, points: usize) -> Vec<Vec3> {
    (0..points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            let angle = 2.0 * PI * turns * t;
            center
                + Vec3::new(
                    radius * angle.cos(),
                    height * (t - 0.5),
                    radius * angle.sin(),
                )
        })
        .collect()
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // === Create structures for Group A (geometric primitives) ===

    // Red cube
    let (cube_verts, cube_faces) = create_cube(Vec3::new(-1.5, 0.0, 0.0), 0.8);
    polyscope::register_surface_mesh("red_cube", cube_verts, cube_faces);
    polyscope::with_surface_mesh("red_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.9, 0.2, 0.2));
    });

    // Green cube
    let (cube_verts, cube_faces) = create_cube(Vec3::new(0.0, 0.0, 0.0), 0.8);
    polyscope::register_surface_mesh("green_cube", cube_verts, cube_faces);
    polyscope::with_surface_mesh("green_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.9, 0.2));
    });

    // Blue cube
    let (cube_verts, cube_faces) = create_cube(Vec3::new(1.5, 0.0, 0.0), 0.8);
    polyscope::register_surface_mesh("blue_cube", cube_verts, cube_faces);
    polyscope::with_surface_mesh("blue_cube", |mesh| {
        mesh.set_surface_color(Vec3::new(0.2, 0.2, 0.9));
    });

    // === Create structures for Group B (point clouds) ===

    // Sphere point cloud 1
    let sphere1_pts = create_sphere_points(Vec3::new(-1.5, 2.0, 0.0), 0.4, 200);
    let sphere1 = polyscope::register_point_cloud("sphere_1", sphere1_pts);
    sphere1.add_color_quantity(
        "color",
        (0..200).map(|_| Vec3::new(1.0, 0.6, 0.2)).collect(),
    );

    // Sphere point cloud 2
    let sphere2_pts = create_sphere_points(Vec3::new(0.0, 2.0, 0.0), 0.4, 200);
    let sphere2 = polyscope::register_point_cloud("sphere_2", sphere2_pts);
    sphere2.add_color_quantity(
        "color",
        (0..200).map(|_| Vec3::new(0.6, 0.2, 1.0)).collect(),
    );

    // Sphere point cloud 3
    let sphere3_pts = create_sphere_points(Vec3::new(1.5, 2.0, 0.0), 0.4, 200);
    let sphere3 = polyscope::register_point_cloud("sphere_3", sphere3_pts);
    sphere3.add_color_quantity(
        "color",
        (0..200).map(|_| Vec3::new(0.2, 1.0, 0.6)).collect(),
    );

    // === Create structures for Group C (curves) ===

    // Helix 1
    let helix1_pts = create_helix(Vec3::new(-1.5, -2.0, 0.0), 0.3, 1.0, 2.0, 50);
    polyscope::register_curve_network_line("helix_1", helix1_pts);
    polyscope::with_curve_network("helix_1", |cn| {
        cn.set_color(Vec3::new(1.0, 0.8, 0.0));
        cn.set_radius(0.02, true);
    });

    // Helix 2
    let helix2_pts = create_helix(Vec3::new(0.0, -2.0, 0.0), 0.3, 1.0, 3.0, 75);
    polyscope::register_curve_network_line("helix_2", helix2_pts);
    polyscope::with_curve_network("helix_2", |cn| {
        cn.set_color(Vec3::new(0.0, 0.8, 1.0));
        cn.set_radius(0.02, true);
    });

    // Helix 3
    let helix3_pts = create_helix(Vec3::new(1.5, -2.0, 0.0), 0.3, 1.0, 4.0, 100);
    polyscope::register_curve_network_line("helix_3", helix3_pts);
    polyscope::with_curve_network("helix_3", |cn| {
        cn.set_color(Vec3::new(1.0, 0.0, 0.8));
        cn.set_radius(0.02, true);
    });

    // === Create groups ===

    // Main parent group
    let all_objects = polyscope::create_group("All Objects");
    all_objects.set_show_child_details(true);

    // Sub-groups
    let cubes_group = polyscope::create_group("Cubes");
    cubes_group
        .add_surface_mesh("red_cube")
        .add_surface_mesh("green_cube")
        .add_surface_mesh("blue_cube");

    let spheres_group = polyscope::create_group("Spheres");
    spheres_group
        .add_point_cloud("sphere_1")
        .add_point_cloud("sphere_2")
        .add_point_cloud("sphere_3");

    let curves_group = polyscope::create_group("Curves");
    curves_group
        .add_curve_network("helix_1")
        .add_curve_network("helix_2")
        .add_curve_network("helix_3");

    // Build hierarchy
    all_objects
        .add_child_group("Cubes")
        .add_child_group("Spheres")
        .add_child_group("Curves");

    // === Select a structure and set up gizmo ===

    // Select the green cube for manipulation
    polyscope::select_structure("SurfaceMesh", "green_cube");
    polyscope::set_gizmo_visible(true);
    polyscope::set_gizmo_mode(polyscope::GizmoMode::Translate);
    polyscope::set_gizmo_space(polyscope::GizmoSpace::World);

    // Set up snap values
    polyscope::set_gizmo_snap_translate(0.1); // Snap to 0.1 units
    polyscope::set_gizmo_snap_rotate(15.0); // Snap to 15 degrees
    polyscope::set_gizmo_snap_scale(0.1); // Snap to 0.1 increments

    println!("Groups and Gizmos Demo");
    println!("======================");
    println!();
    println!("This demo shows how to organize structures into groups");
    println!("and manipulate them with gizmos.");
    println!();
    println!("Groups hierarchy:");
    println!("  All Objects");
    println!("    ├── Cubes (red, green, blue cubes)");
    println!("    ├── Spheres (3 point cloud spheres)");
    println!("    └── Curves (3 helix curves)");
    println!();
    println!("Gizmo Controls:");
    println!("  - The green cube is selected by default");
    println!("  - Click on a structure to select it");
    println!("  - Use the Gizmo panel in the UI to change modes:");
    println!("    * Translate (T): Move the structure");
    println!("    * Rotate (R): Rotate around center");
    println!("    * Scale (S): Scale uniformly or per-axis");
    println!("  - Toggle World/Local space in the UI");
    println!();
    println!("Group Controls:");
    println!("  - Toggle group visibility in the Groups panel");
    println!("  - Disabling a parent group hides all children");
    println!();
    println!("Camera Controls:");
    println!("  - Left drag: Orbit");
    println!("  - Right drag: Pan");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
