//! Slice Plane Demo
//!
//! Demonstrates slice plane functionality with different structure types:
//! - Surface mesh (Stanford Bunny)
//! - Point cloud (random points)
//! - Volume mesh (tetrahedral cube)
//!
//! Run with: cargo run --example slice_plane_demo

use glam::Vec3;

/// Load an OBJ file and return vertices and triangle faces.
fn load_obj(path: &str) -> (Vec<Vec3>, Vec<glam::UVec3>) {
    let (models, _materials) = tobj::load_obj(
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
        let vertex_offset = vertices.len() as u32;

        for i in (0..mesh.positions.len()).step_by(3) {
            vertices.push(Vec3::new(
                mesh.positions[i],
                mesh.positions[i + 1],
                mesh.positions[i + 2],
            ));
        }

        for i in (0..mesh.indices.len()).step_by(3) {
            faces.push(glam::UVec3::new(
                mesh.indices[i] + vertex_offset,
                mesh.indices[i + 1] + vertex_offset,
                mesh.indices[i + 2] + vertex_offset,
            ));
        }
    }

    (vertices, faces)
}

/// Generate a simple tetrahedral cube mesh.
///
/// Creates a cube from (0,0,0) to (size,size,size) subdivided into tetrahedra.
fn generate_tet_cube(size: f32, subdivisions: usize) -> (Vec<Vec3>, Vec<[u32; 4]>) {
    let n = subdivisions + 1;
    let step = size / subdivisions as f32;

    // Generate grid of vertices
    let mut vertices = Vec::new();
    for iz in 0..n {
        for iy in 0..n {
            for ix in 0..n {
                vertices.push(Vec3::new(
                    ix as f32 * step,
                    iy as f32 * step,
                    iz as f32 * step,
                ));
            }
        }
    }

    // Helper to get vertex index
    let idx = |ix: usize, iy: usize, iz: usize| -> u32 {
        (iz * n * n + iy * n + ix) as u32
    };

    // Generate tetrahedra (5 tets per cube)
    let mut tets = Vec::new();
    for iz in 0..subdivisions {
        for iy in 0..subdivisions {
            for ix in 0..subdivisions {
                // 8 vertices of this cube cell
                let v000 = idx(ix, iy, iz);
                let v100 = idx(ix + 1, iy, iz);
                let v010 = idx(ix, iy + 1, iz);
                let v110 = idx(ix + 1, iy + 1, iz);
                let v001 = idx(ix, iy, iz + 1);
                let v101 = idx(ix + 1, iy, iz + 1);
                let v011 = idx(ix, iy + 1, iz + 1);
                let v111 = idx(ix + 1, iy + 1, iz + 1);

                // Decompose cube into 5 tetrahedra
                tets.push([v000, v100, v010, v001]);
                tets.push([v100, v110, v010, v111]);
                tets.push([v100, v101, v001, v111]);
                tets.push([v010, v011, v001, v111]);
                tets.push([v100, v010, v001, v111]);
            }
        }
    }

    (vertices, tets)
}

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    println!("Slice Plane Demo");
    println!("================");
    println!("This demo shows slice plane functionality with multiple structure types.");
    println!();

    // 1. Load and register the Stanford Bunny (surface mesh)
    let (bunny_verts, bunny_faces) = load_obj("assets/bunny.obj");
    println!("Loaded bunny: {} vertices, {} faces", bunny_verts.len(), bunny_faces.len());

    let _bunny = polyscope::register_surface_mesh("bunny", bunny_verts.clone(), bunny_faces);

    polyscope::with_surface_mesh("bunny", |mesh| {
        mesh.set_surface_color(Vec3::new(0.8, 0.6, 0.4));
    });

    // 2. Create a point cloud (random points in a sphere)
    let num_points = 500;
    let point_cloud_offset = Vec3::new(0.12, 0.08, 0.0); // Offset to the right of bunny
    let points: Vec<Vec3> = (0..num_points)
        .map(|i| {
            // Use golden ratio spiral for even distribution on sphere
            let phi = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt()) * i as f32;
            let y = 1.0 - (i as f32 / (num_points - 1) as f32) * 2.0;
            let radius_at_y = (1.0 - y * y).sqrt();
            let x = phi.cos() * radius_at_y;
            let z = phi.sin() * radius_at_y;
            Vec3::new(x, y, z) * 0.03 + point_cloud_offset
        })
        .collect();

    println!("Created point cloud: {} points", points.len());

    let pc_handle = polyscope::register_point_cloud("point_sphere", points.clone());

    // Add scalar quantity based on height
    let heights: Vec<f32> = points.iter().map(|p| p.y).collect();
    pc_handle.add_scalar_quantity("height", heights);

    // 3. Create a tetrahedral volume mesh (offset to the left)
    let tet_offset = Vec3::new(-0.1, 0.02, 0.0);
    let (tet_verts, tets) = generate_tet_cube(0.06, 3);
    let tet_verts: Vec<Vec3> = tet_verts.iter().map(|v| *v + tet_offset).collect();

    println!("Created tet mesh: {} vertices, {} tets", tet_verts.len(), tets.len());

    let tet_handle = polyscope::register_tet_mesh("tet_cube", tet_verts.clone(), tets);

    // Add vertex color quantity for visualization
    let colors: Vec<Vec3> = tet_verts
        .iter()
        .map(|v| {
            // Color based on position
            let t = (v.y - tet_offset.y) / 0.06;
            Vec3::new(1.0 - t, t, 0.5)
        })
        .collect();
    tet_handle
        .add_vertex_color_quantity("position_color", colors)
        .set_color(Vec3::new(0.4, 0.5, 0.7))
        .set_interior_color(Vec3::new(0.6, 0.3, 0.3));

    // 4. Add a slice plane
    println!();
    println!("Adding slice plane...");

    let slice_handle = polyscope::add_slice_plane("main_slicer");

    // Position the plane to cut through all structures
    slice_handle
        .set_origin(Vec3::new(0.0, 0.06, 0.0))
        .set_normal(Vec3::new(0.0, 1.0, 0.2).normalize())
        .set_color(Vec3::new(0.9, 0.8, 0.3))
        .set_transparency(0.4)
        .set_draw_plane(true);

    println!();
    println!("Controls:");
    println!("- Use the UI panel on the left to adjust slice plane parameters");
    println!("- Toggle 'draw_plane' to show/hide the slice plane visualization");
    println!("- Adjust origin and normal to move/rotate the slice");
    println!("- The tet_cube shows cross-section capping with interpolated colors");
    println!();

    polyscope::show();
}
