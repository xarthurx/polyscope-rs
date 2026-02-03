# Getting Started with polyscope-rs

A comprehensive guide to using polyscope-rs for 3D visualization.

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Structures](#structures)
  - [Point Clouds](#point-clouds)
  - [Surface Meshes](#surface-meshes)
  - [Curve Networks](#curve-networks)
  - [Volume Meshes](#volume-meshes)
  - [Volume Grids](#volume-grids)
  - [Camera Views](#camera-views)
- [Quantities](#quantities)
  - [Scalar Quantities](#scalar-quantities)
  - [Vector Quantities](#vector-quantities)
  - [Color Quantities](#color-quantities)
  - [Parameterization](#parameterization)
- [Appearance](#appearance)
  - [Materials](#materials)
  - [Colormaps](#colormaps)
  - [Transparency](#transparency)
- [Scene Features](#scene-features)
  - [Ground Plane](#ground-plane)
  - [Slice Planes](#slice-planes)
  - [Groups](#groups)
  - [Gizmos](#gizmos)
- [Screenshots and Headless Rendering](#screenshots-and-headless-rendering)
- [UI Controls](#ui-controls)
- [API Patterns](#api-patterns)

---

## Installation

Add polyscope-rs to your `Cargo.toml`:

```toml
[dependencies]
polyscope-rs = "0.5"
```

## Basic Usage

Every polyscope-rs program follows this pattern:

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    // 1. Initialize polyscope
    init()?;

    // 2. Register structures and add quantities
    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    register_point_cloud("my points", points);

    // 3. Show the viewer (blocks until window is closed)
    show();

    Ok(())
}
```

**Key functions:**
- `init()` - Must be called first, initializes the global state
- `register_*()` - Registers a structure (point cloud, mesh, etc.)
- `show()` - Opens the viewer window (blocking)

---

## Structures

Structures are geometric objects in the scene. Each structure type has a dedicated registration function.

### Point Clouds

Point clouds are sets of points in 3D space, rendered as spheres.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Create some points
    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
    ];

    // Register the point cloud
    let pc = register_point_cloud("my points", points);

    // Add quantities (see Quantities section)
    pc.add_scalar_quantity("height", vec![0.0, 0.0, 1.0]);

    show();
    Ok(())
}
```

### Surface Meshes

Surface meshes are triangular or polygonal meshes. polyscope-rs supports arbitrary n-gon faces.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Vertices
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];

    // Triangle faces (using UVec3)
    let triangles = vec![
        glam::UVec3::new(0, 1, 2),
        glam::UVec3::new(0, 2, 3),
    ];
    let mesh = register_surface_mesh("triangles", vertices.clone(), triangles);

    // Or use arrays: Vec<[u32; 3]>
    let triangles_arr = vec![[0, 1, 2], [0, 2, 3]];
    register_surface_mesh("triangles_arr", vertices.clone(), triangles_arr);

    // Or polygon faces: Vec<Vec<u32>> for quads, pentagons, etc.
    let quad = vec![vec![0, 1, 2, 3]];
    register_surface_mesh("quad", vertices.clone(), quad);

    show();
    Ok(())
}
```

**Appearance options:**

```rust
let mesh = register_surface_mesh("mesh", vertices, faces);

mesh.set_surface_color(Vec3::new(0.2, 0.5, 0.8))  // Blue surface
    .set_show_edges(true)                          // Show wireframe
    .set_edge_color(Vec3::new(0.0, 0.0, 0.0))     // Black edges
    .set_edge_width(1.5)                           // Edge thickness
    .set_transparency(0.3)                         // 30% transparent
    .set_material("clay");                         // Matcap material
```

### Curve Networks

Curve networks are collections of nodes connected by edges.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];

    // Explicit edges
    let edges = vec![[0, 1], [1, 2], [2, 3]];
    register_curve_network("explicit", nodes.clone(), edges);

    // Connected line: 0-1-2-3
    register_curve_network_line("line", nodes.clone());

    // Closed loop: 0-1-2-3-0
    register_curve_network_loop("loop", nodes.clone());

    // Separate segments: 0-1, 2-3
    register_curve_network_segments("segments", nodes);

    show();
    Ok(())
}
```

### Volume Meshes

Volume meshes contain tetrahedral or hexahedral cells.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Single tetrahedron
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tets = vec![[0u32, 1, 2, 3]];
    register_volume_mesh("tet", vertices, tets, VolumeCellType::Tet);

    // Hexahedral cells use 8 vertices per cell
    // register_volume_mesh("hex", hex_verts, hexes, VolumeCellType::Hex);

    show();
    Ok(())
}
```

### Volume Grids

Volume grids are regular 3D grids with scalar data, useful for implicit surfaces.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Create a 10x10x10 grid
    let dims = (10, 10, 10);
    let bound_min = Vec3::new(-1.0, -1.0, -1.0);
    let bound_max = Vec3::new(1.0, 1.0, 1.0);

    let grid = register_volume_grid("grid", dims, bound_min, bound_max);

    // Add scalar data (node-based, size = 10*10*10 = 1000)
    let mut values = Vec::new();
    for k in 0..10 {
        for j in 0..10 {
            for i in 0..10 {
                // Sphere SDF
                let x = -1.0 + 2.0 * i as f32 / 9.0;
                let y = -1.0 + 2.0 * j as f32 / 9.0;
                let z = -1.0 + 2.0 * k as f32 / 9.0;
                values.push((x*x + y*y + z*z).sqrt() - 0.5);
            }
        }
    }
    grid.add_node_scalar_quantity("sdf", values);

    show();
    Ok(())
}
```

**Visualization modes:**
- **Gridcube** - Shows grid cells colored by scalar value
- **Isosurface** - Extracts and renders the zero-level isosurface (marching cubes)

### Camera Views

Camera views visualize camera frustums.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    let intrinsics = CameraIntrinsics::from_fov_vertical(
        std::f32::consts::FRAC_PI_4,  // 45 degree vertical FOV
        1.0,                           // aspect ratio
    );

    let extrinsics = CameraExtrinsics::from_look_at(
        Vec3::new(2.0, 2.0, 2.0),  // camera position
        Vec3::ZERO,                 // look at origin
        Vec3::Y,                    // up vector
    );

    let params = CameraParameters::new(intrinsics, extrinsics);
    register_camera_view("camera", params);

    show();
    Ok(())
}
```

---

## Quantities

Quantities are data associated with structures. They control how structures are colored and can visualize scalar fields, vector fields, and more.

### Scalar Quantities

Scalar quantities map a single value to each element, visualized using colormaps.

```rust
// Point cloud: one value per point
let pc = register_point_cloud("points", points);
pc.add_scalar_quantity("temperature", temperatures);

// Surface mesh: per-vertex or per-face
let mesh = register_surface_mesh("mesh", vertices, faces);
mesh.add_vertex_scalar_quantity("curvature", vertex_values);
mesh.add_face_scalar_quantity("area", face_values);

// Curve network: per-node or per-edge
let cn = register_curve_network("curves", nodes, edges);
cn.add_node_scalar_quantity("pressure", node_values);
cn.add_edge_scalar_quantity("flow", edge_values);
```

### Vector Quantities

Vector quantities display arrows at each element.

```rust
// Point cloud
pc.add_vector_quantity("velocity", velocities);

// Surface mesh: per-vertex or per-face
mesh.add_vertex_vector_quantity("normals", vertex_normals);
mesh.add_face_vector_quantity("face_normals", face_normals);

// Curve network
cn.add_node_vector_quantity("tangent", tangents);
```

Vectors are automatically scaled based on scene size. The UI allows adjusting vector length and radius.

### Color Quantities

Color quantities assign RGB or RGBA colors directly.

```rust
// RGB colors (Vec3, alpha defaults to 1.0)
mesh.add_vertex_color_quantity("vertex_colors", rgb_colors);
mesh.add_face_color_quantity("face_colors", face_rgb);

// RGBA colors with per-element transparency (Vec4)
mesh.add_vertex_color_quantity_with_alpha("rgba_colors", rgba_colors);
mesh.add_face_color_quantity_with_alpha("rgba_faces", face_rgba);
```

### Parameterization

Parameterization quantities visualize UV coordinates on surface meshes.

```rust
// Per-vertex UVs
mesh.add_vertex_parameterization_quantity("uv", vertex_uvs);

// Per-corner UVs (for seams/discontinuities)
mesh.add_corner_parameterization_quantity("corner_uv", corner_uvs);
```

Visualization styles (configurable in UI):
- **Checker** - Checkerboard pattern
- **Grid** - Grid lines
- **Local Check** - Per-face checkerboard
- **Local Rad** - Radial pattern

### Advanced Surface Mesh Quantities

```rust
// Intrinsic vectors (tangent space)
mesh.add_vertex_intrinsic_vector_quantity("flow", vectors_2d, basis_x, basis_y);
mesh.add_vertex_intrinsic_vector_quantity_auto("auto_flow", vectors_2d);  // auto-compute basis

// One-forms (edge-based differential forms)
mesh.add_one_form_quantity("one_form", edge_values, edge_orientations);
```

---

## Appearance

### Materials

polyscope-rs includes 8 built-in matcap materials:

- `clay` (default)
- `wax`
- `candy`
- `flat`
- `mud`
- `ceramic`
- `jade`
- `normal`

```rust
mesh.set_material("ceramic");
```

**Custom materials** can be loaded from image files:

```rust
// Blendable material (4 channels: R, G, B, K)
load_blendable_material("metal", [
    "assets/metal_r.hdr",
    "assets/metal_g.hdr",
    "assets/metal_b.hdr",
    "assets/metal_k.hdr",
]);

// Or with base path + extension
load_blendable_material_ext("metal", "assets/metal", ".hdr");

// Static material (single texture, not tintable)
load_static_material("stone", "assets/stone.jpg");

// Use the material
mesh.set_material("metal");
```

### Colormaps

Available colormaps for scalar quantities:
- `viridis` (default)
- `blues`, `reds`, `coolwarm`
- `piyg`, `pink`, `spectral`
- `rainbow`, `jet`, `turbo`

Colormaps are configured in the quantity UI panel.

### Transparency

Two transparency modes:

1. **Simple** - Basic alpha blending (fast but may have ordering artifacts)
2. **Pretty** - Depth peeling (correct transparency, more expensive)

```rust
mesh.set_transparency(0.5);  // 50% transparent
```

The transparency mode is configured globally in the Appearance panel.

---

## Scene Features

### Ground Plane

The ground plane provides visual grounding for the scene.

**Modes:**
- **None** - No ground plane
- **Tile** - Infinite tiled ground
- **Shadow Only** - Shows shadows on an invisible plane
- **Shadow + Reflection** - Shadows plus mirror reflections

Configure via the Appearance panel in the UI.

### Slice Planes

Slice planes cut through geometry to reveal interiors. Up to 4 slice planes are supported.

```rust
use polyscope_rs::*;

// Add a slice plane
let plane_id = add_slice_plane()?;

// Configure the plane
apply_slice_plane_settings(SlicePlaneSettings {
    id: plane_id,
    enabled: true,
    draw_plane: true,
    draw_widget: true,
    // Plane pose (position and orientation)
    ..Default::default()
});
```

Slice planes can be positioned interactively using gizmos in the UI.

### Groups

Groups organize structures hierarchically and control visibility.

```rust
// Create a group
create_group("geometry")?;

// Add structures to the group
add_to_group("geometry", "mesh1")?;
add_to_group("geometry", "mesh2")?;

// Create nested groups
create_group("geometry/subgroup")?;

// Control visibility
set_group_enabled("geometry", false);  // Hides all structures in group
```

### Gizmos

Gizmos allow interactive manipulation of structure transforms.

Enable gizmos for a structure:
1. Select the structure in the UI
2. Enable the gizmo in the Gizmo panel
3. Choose mode: Translate, Rotate, or Scale

The transform is applied to the structure's model matrix.

---

## Screenshots and Headless Rendering

### Screenshots

Take screenshots while the viewer is running:

```rust
// Auto-generated filename (screenshot_0001.png, etc.)
screenshot();

// Specific filename
screenshot_to_file("my_scene.png");

// With options (e.g., transparent background)
screenshot_with_options(ScreenshotOptions {
    transparent_background: true,
    ..Default::default()
});
```

### Headless Rendering

Render without opening a window (useful for batch processing, testing, CI):

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Register structures
    register_point_cloud("pts", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

    // Render to file (auto-positions camera)
    render_to_file("output.png", 800, 600)?;

    // Or get raw pixel data
    let pixels = render_to_image(800, 600)?;
    assert_eq!(pixels.len(), 800 * 600 * 4);  // RGBA, 4 bytes per pixel

    Ok(())
}
```

---

## UI Controls

### Mouse Controls

| Action | Input |
|--------|-------|
| Rotate/orbit | Left drag |
| Pan | Shift + Left drag, or Right drag |
| Zoom | Scroll wheel |
| Select | Left click (no drag) |
| Set view center | Double-click |
| Deselect | Right click (no drag) |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| ESC | Exit |
| R | Reset camera view |

### UI Panels

- **Structures** - List of registered structures with visibility toggles
- **Appearance** - Global rendering settings (ground plane, transparency, tone mapping)
- **View** - Camera settings (navigation mode, projection, FOV)
- **Quantities** - Per-quantity settings (colormap, range, visibility)

---

## API Patterns

### Handle Pattern

Registration functions return handles for chained configuration:

```rust
let mesh = register_surface_mesh("mesh", vertices, faces);

mesh.set_surface_color(Vec3::new(1.0, 0.5, 0.0))
    .set_show_edges(true)
    .add_vertex_scalar_quantity("height", heights)
    .add_vertex_vector_quantity("normals", normals);
```

### Closure Access Pattern

For advanced access, use the `with_*` functions:

```rust
// Mutable access
with_surface_mesh("mesh", |mesh| {
    mesh.set_surface_color(Vec3::new(1.0, 0.0, 0.0));
    // Access internal state, call multiple methods, etc.
});

// Immutable access
let vertex_count = with_surface_mesh_ref("mesh", |mesh| {
    mesh.n_vertices()
});
```

### Error Handling

polyscope-rs uses `Result<T, PolyscopeError>` for fallible operations:

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;  // Returns Result

    // Registration functions panic on duplicate names
    // Use get_* to check if a structure exists
    if get_surface_mesh("mesh").is_none() {
        register_surface_mesh("mesh", vertices, faces);
    }

    Ok(())
}
```

---

## Running the Examples

The repository includes many example programs:

```bash
cargo run --example point_cloud_demo
cargo run --example surface_mesh_demo
cargo run --example curve_network_demo
cargo run --example volume_mesh_demo
cargo run --example volume_grid_demo
cargo run --example camera_view_demo
cargo run --example slice_plane_demo
cargo run --example groups_and_gizmos_demo
cargo run --example ground_plane_demo
cargo run --example polygon_mesh_demo
cargo run --example materials_demo
cargo run --example transparency_demo
```

---

## Next Steps

- Explore the [API documentation](https://docs.rs/polyscope) (rustdoc)
- Check the [Feature Status](feature-status.md) for supported features
- Read the [Development Guide](development-guide.md) for contributing
- See [Architecture Differences](architecture-differences.md) for C++ vs Rust details
