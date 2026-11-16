# Getting Started with polyscope-rs

A guide to 3D visualization with polyscope-rs 0.6.1.

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
  - [Enable a Quantity](#enable-a-quantity)
  - [Scalar Quantities](#scalar-quantities)
  - [Vector Quantities](#vector-quantities)
  - [Color Quantities](#color-quantities)
  - [Parameterization](#parameterization)
- [Appearance](#appearance)
  - [Materials](#materials)
  - [Colormaps](#colormaps)
  - [Transparency](#transparency)
  - [Render Look (Options)](#render-look-options)
- [Scene Features](#scene-features)
  - [Ground Plane](#ground-plane)
  - [Slice Planes](#slice-planes)
  - [Groups](#groups)
  - [Gizmos](#gizmos)
- [Save and Load a View](#save-and-load-a-view)
- [Screenshots and Headless Rendering](#screenshots-and-headless-rendering)
- [UI Controls](#ui-controls)
- [API Patterns](#api-patterns)

---

## Installation

Add polyscope-rs to your `Cargo.toml`:

```toml
[dependencies]
polyscope-rs = "0.6"
```

Requirements:

- Rust 1.96 or later.
- A graphics driver with Vulkan, Metal or DirectX 12. polyscope-rs does not support the OpenGL backend of wgpu.

If no supported backend is available, `render_to_image` returns an error that contains the message "no graphics adapter found (Vulkan, Metal or DirectX 12 is necessary)". `show()` stops with a panic.

`polyscope_rs` re-exports `Vec2`, `Vec3`, `Vec4` and `Mat4` from glam. Some examples below use other glam types, such as `glam::UVec3`. For these types, also add glam:

```toml
[dependencies]
polyscope-rs = "0.6"
glam = "0.34"
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

    // 3. Show the viewer (blocks until the window closes)
    show();

    Ok(())
}
```

**Key functions:**
- `init()` - Initializes the global state. Call it first, and only once.
- `register_*()` - Registers a structure (point cloud, mesh, etc.)
- `show()` - Opens the viewer window. It returns when the window closes.

The program can continue after `show()` returns. For example, it can save the last view or render images without a window.

---

## Structures

Structures are geometric objects in the scene. Each structure type has a registration function.

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

    // Point radius and color are on the structure itself
    with_point_cloud("my points", |pc| {
        pc.set_point_radius(0.02);
        pc.set_base_color(Vec3::new(0.9, 0.4, 0.1));
    });

    show();
    Ok(())
}
```

### Surface Meshes

Surface meshes are triangle or polygon meshes. polyscope-rs supports arbitrary n-gon faces.

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

    // Triangle faces (UVec3 needs the glam dependency)
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
    .set_transparency(0.3)                         // 0.0 = opaque, 1.0 = invisible
    .set_material("clay");                         // Matcap material
```

### Curve Networks

Curve networks are collections of nodes connected by edges. They render as lines (the default) or as tubes.

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
    let cn = register_curve_network_segments("segments", nodes);

    // Radius: the second argument selects a value relative to the scene size
    cn.set_radius(0.02, true).set_color(Vec3::new(0.2, 0.8, 0.4));

    // Render mode: 0 = lines, 1 = tubes
    with_curve_network("segments", |cn| {
        cn.set_render_mode(1);
    });

    show();
    Ok(())
}
```

The radius applies to the whole network. polyscope-rs has no per-node or per-edge radius.

### Volume Meshes

Volume meshes contain tetrahedra, hexahedra, prisms or pyramids. polyscope-rs shows only the exterior faces.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Single tetrahedron: 4 vertex indices per cell
    let tet_vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    register_tet_mesh("tet", tet_vertices, vec![[0, 1, 2, 3]]);

    // Single hexahedron: bottom face 0-3, top face 4-7 above it
    let hex_vertices = vec![
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(3.0, 1.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
        Vec3::new(2.0, 0.0, 1.0),
        Vec3::new(3.0, 0.0, 1.0),
        Vec3::new(3.0, 1.0, 1.0),
        Vec3::new(2.0, 1.0, 1.0),
    ];
    let hex = register_hex_mesh("hex", hex_vertices, vec![[0, 1, 2, 3, 4, 5, 6, 7]]);
    hex.add_cell_scalar_quantity("id", vec![1.0]);

    show();
    Ok(())
}
```

Other registration functions:

- `register_prism_mesh(name, vertices, Vec<[u32; 6]>)`: slots 0-2 are the bottom triangle, slots 3-5 the top triangle.
- `register_pyramid_mesh(name, vertices, Vec<[u32; 5]>)`: slots 0-3 are the base quad, slot 4 is the apex.
- `register_volume_mesh(name, vertices, Vec<[u32; 8]>)`: mixed cells. Fill the unused trailing slots with `u32::MAX`. A tet uses 4 slots, a pyramid 5, a prism 6 and a hex 8.

A slice plane removes each cell whose center is on the cut side. polyscope-rs draws no cut surface.

### Volume Grids

Volume grids are regular 3D grids with scalar data, for example for implicit surfaces.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Create a 10x10x10 grid of nodes
    let bound_min = Vec3::new(-1.0, -1.0, -1.0);
    let bound_max = Vec3::new(1.0, 1.0, 1.0);
    let grid = register_volume_grid_uniform("grid", 10, bound_min, bound_max);

    // Add scalar data (node-based, size = 10*10*10 = 1000, x varies fastest)
    let mut values = Vec::new();
    for k in 0..10 {
        for j in 0..10 {
            for i in 0..10 {
                // Sphere SDF
                let x = -1.0 + 2.0 * i as f32 / 9.0;
                let y = -1.0 + 2.0 * j as f32 / 9.0;
                let z = -1.0 + 2.0 * k as f32 / 9.0;
                values.push((x * x + y * y + z * z).sqrt() - 0.5);
            }
        }
    }
    grid.add_node_scalar_quantity("sdf", values);
    grid.set_quantity_enabled("sdf", true);

    // Show the zero level set instead of the gridcubes
    grid.set_node_scalar_viz_mode("sdf", VolumeGridVizMode::Isosurface)
        .set_isosurface_level("sdf", 0.0);

    show();
    Ok(())
}
```

For a grid with different node counts per axis, use `register_volume_grid(name, glam::UVec3::new(nx, ny, nz), bound_min, bound_max)`.

**Visualization modes** of a node scalar quantity:
- **Gridcube** - Shows grid cells colored by scalar value
- **Isosurface** - Extracts an isosurface with marching cubes. The level is adjustable; the default is the middle of the data range.

A node scalar quantity shows one mode at a time. Cell scalar quantities show gridcubes.

### Camera Views

Camera views visualize camera frustums.

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // Vertical field of view in degrees, and aspect ratio
    let intrinsics = CameraIntrinsics::new(45.0, 1.0);

    let extrinsics = CameraExtrinsics::look_at(
        Vec3::new(2.0, 2.0, 2.0), // camera position
        Vec3::ZERO,               // look at origin
        Vec3::Y,                  // up vector
    );

    let params = CameraParameters::new(intrinsics, extrinsics);
    register_camera_view("camera", params);

    // Or in one call: position, target, up, vertical FOV (degrees), aspect ratio
    register_camera_view_look_at("camera 2", Vec3::new(-2.0, 1.0, 2.0), Vec3::ZERO, Vec3::Y, 60.0, 1.5);

    show();
    Ok(())
}
```

The panel of a camera view has a "fly to" action. It moves the viewer camera to the camera view in 0.4 s.

---

## Quantities

Quantities are data associated with structures. They control how structures are colored and can visualize scalar fields, vector fields, and more.

The snippets in this section are fragments. They use structure handles and data vectors from the context, with the names in the snippet.

### Enable a Quantity

A new quantity is disabled. Enable it in the panel of its structure, or in code:

```rust
// Point clouds, surface meshes and curve networks
with_point_cloud("points", |pc| {
    if let Some(q) = pc.get_quantity_mut("temperature") {
        q.set_enabled(true);
    }
});

// Volume meshes and volume grids have a handle method
volume_mesh.set_quantity_enabled("pressure", true);
grid.set_quantity_enabled("sdf", true);
```

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

// Curve network: per-node or per-edge (on the structure, through with_curve_network)
register_curve_network("curves", nodes, edges);
with_curve_network("curves", |cn| {
    cn.add_node_scalar_quantity("pressure", node_values);
    cn.add_edge_scalar_quantity("flow", edge_values);
});

// Volume mesh: per-vertex or per-cell
volume_mesh.add_vertex_scalar_quantity("temperature", tet_vertex_values);
volume_mesh.add_cell_scalar_quantity("stress", cell_values);
```

The quantity panel sets the colormap and the range. Limits of scalar quantities in this release:

- No `DataType` (symmetric, magnitude, categorical), no isolines and no on-screen colorbar.
- The handles have no colormap or range setter, except `VolumeGridHandle::set_color_map`.
- polyscope-rs computes one color per element on the CPU. The GPU interpolates vertex colors across faces. C++ Polyscope samples the colormap per pixel.

### Vector Quantities

Vector quantities display arrows at each element.

```rust
// Point cloud
pc.add_vector_quantity("velocity", velocities);

// Surface mesh: per-vertex or per-face
mesh.add_vertex_vector_quantity("normals", vertex_normals);
mesh.add_face_vector_quantity("face_normals", face_normals);

// Curve network: per-node or per-edge
with_curve_network("curves", |cn| {
    cn.add_node_vector_quantity("tangent", tangents);
    cn.add_edge_vector_quantity("direction", edge_directions);
});

// Volume mesh: per-vertex or per-cell
volume_mesh.add_vertex_vector_quantity("displacement", tet_vertex_vectors);
volume_mesh.add_cell_vector_quantity("flux", cell_vectors);
```

Vectors are automatically scaled based on scene size. The UI allows adjusting vector length and radius.

### Color Quantities

Color quantities assign RGB or RGBA colors directly.

```rust
// RGB colors (Vec3, alpha defaults to 1.0)
mesh.add_vertex_color_quantity("vertex_colors", rgb_colors);
mesh.add_face_color_quantity("face_colors", face_rgb);

// RGBA colors with per-element transparency (Vec4, surface meshes only)
mesh.add_vertex_color_quantity_with_alpha("rgba_colors", rgba_colors);
mesh.add_face_color_quantity_with_alpha("rgba_faces", face_rgba);

// Other structures take RGB colors
pc.add_color_quantity("point_colors", point_rgb);
volume_mesh.add_cell_color_quantity("cell_colors", cell_rgb);
```

Per-element alpha needs the Pretty transparency mode (see [Transparency](#transparency)).

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
- **LocalCheck** - Checkerboard over a radial colormap
- **LocalRad** - Distance stripes over a radial colormap

### Advanced Surface Mesh Quantities

```rust
// Intrinsic vectors (tangent space)
mesh.add_vertex_intrinsic_vector_quantity("flow", vectors_2d, basis_x, basis_y);
mesh.add_vertex_intrinsic_vector_quantity_auto("auto_flow", auto_vectors_2d);  // auto-compute basis

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

The viewer loads the files on the next frame. Then the material is available in the material selectors.

### Colormaps

polyscope-rs has the 15 built-in colormaps of C++ Polyscope, with the same color tables:

- `viridis` (the default for all scalar quantities)
- `plasma`, `inferno`, `magma`, `gray`
- `coolwarm`, `blues`, `reds`, `pink-green`
- `phase`, `spectral`, `rainbow`, `jet`, `turbo`, `hsv`

The constant `BUILTIN_COLOR_MAP_NAMES` holds these names in the same order.

```rust
for name in BUILTIN_COLOR_MAP_NAMES {
    println!("{name}");
}

// Volume grids: set the colormap of a node or cell scalar quantity
grid.set_color_map("sdf", "coolwarm");
```

For the other structures, select the colormap in the quantity panel. polyscope-rs has no function to load a user colormap.

### Transparency

Transparency modes (`TransparencyMode`):

1. **Simple** - Basic alpha blending (fast but may have ordering artifacts). This is the default.
2. **Pretty** - Depth peeling (correct transparency, more expensive). It applies to surface meshes.
3. **None** - Renders like `Simple` in this release.

```rust
// Structure transparency (surface meshes only)
mesh.set_transparency(0.5);  // 50% transparent

// Global transparency mode
with_context_mut(|ctx| {
    ctx.options.transparency_mode = TransparencyMode::Pretty;
    ctx.options.transparency_render_passes = 8;
});
```

The Appearance panel shows and edits the same mode. Pretty mode can merge layers that are closer than about 0.002 in depth.

### Render Look (Options)

`Options` holds the render look of the viewer, of screenshots and of headless renders. Change its fields with `with_context_mut`. The next frame uses the new values, and the UI panels show them.

```rust
with_context_mut(|ctx| {
    let options = &mut ctx.options;
    options.background_color = Vec3::new(1.0, 1.0, 1.0); // linear RGB
    options.ground_plane.mode = GroundPlaneMode::TileReflection;
    options.ground_plane.shadow_darkness = 0.25;
    options.transparency_mode = TransparencyMode::Pretty;
    options.ssaa_factor = 1; // 1, 2 or 4
    options.ssao.enabled = true;
    options.max_fps = 0; // 0 = unlimited
});
```

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `background_color` | `Vec3` | (1.0, 1.0, 1.0), white | Background color (linear RGB) |
| `ground_plane` | `GroundPlaneConfig` | Tile mode | Mode, height, shadow darkness, reflection intensity |
| `transparency_mode` | `TransparencyMode` | `Simple` | Transparency mode |
| `transparency_render_passes` | `u32` | 8 | Number of depth-peeling passes in Pretty mode |
| `ssaa_factor` | `u32` | 1 | Supersampling factor: 1, 2 or 4 |
| `ssao` | `SsaoConfig` | disabled | Ambient occlusion: `enabled`, `radius`, `intensity`, `bias`, `sample_count` |
| `max_fps` | `u32` | 60 | Frame rate limit of the window (0 = unlimited) |
| `auto_compute_scene_extents` | `bool` | `true` | Compute the scene bounds from the structures |

SSAO has no effect when `ssaa_factor` is above 1. Headless rendering ignores `max_fps`. Tone mapping (exposure, white level, gamma) is not in `Options`; the Tone Mapping panel sets it.

---

## Scene Features

### Ground Plane

The ground plane provides visual grounding for the scene. It lies below the scene, perpendicular to the up direction.

**Modes** (`GroundPlaneMode`, UI label in parentheses):
- **None** (None) - No ground plane
- **Tile** (Tile) - Tiled ground. This is the default.
- **ShadowOnly** (Shadow Only) - Shadows on an invisible plane
- **TileReflection** (Tile + Reflection) - Tiled ground with mirror reflections

Shadows show in the Tile, ShadowOnly and TileReflection modes. Only surface meshes cast shadows.

```rust
with_context_mut(|ctx| {
    let ground = &mut ctx.options.ground_plane;
    ground.mode = GroundPlaneMode::TileReflection;
    ground.height_is_relative = false; // use the height below
    ground.height = -1.0;              // coordinate on the up axis
    ground.reflection_intensity = 0.4; // 0.0 to 1.0
});
```

The Ground Plane panel edits the same values.

### Slice Planes

Slice planes cut through geometry to reveal interiors. At most 4 slice planes act at a time.

```rust
// Add a slice plane at the scene center
let plane = add_slice_plane("plane");

// Configure the plane: origin, normal, and what to draw
plane
    .set_pose(Vec3::ZERO, Vec3::Y)
    .set_enabled(true)
    .set_draw_plane(true)
    .set_draw_widget(true);

// Other ways to add a plane
add_slice_plane_with_pose("x cut", Vec3::ZERO, Vec3::X);
let auto = add_slice_plane_auto(); // named "Scene Slice Plane N"

// Remove planes
auto.remove();
remove_slice_plane("x cut");
```

Click a slice plane to move it with the gizmo. `add_slice_plane` with a name in use moves that plane to the scene center.

### Groups

Groups organize structures hierarchically and control visibility.

```rust
// Create a group and add structures to it
let group = create_group("geometry");
group.add_surface_mesh("mesh1").add_surface_mesh("mesh2");

// Create a nested group
create_group("subgroup");
group.add_child_group("subgroup");

// Hide all structures in the group and in its child groups
group.set_enabled(false);
```

The handle also has `add_point_cloud`, `add_curve_network`, `add_volume_mesh`, `add_volume_grid` and `add_camera_view`. `set_enabled` sets the flag of the group only; it does not change the flags of child groups and structures.

### Gizmos

Gizmos allow interactive manipulation of structure transforms. One gizmo exists. It acts on the selected structure or slice plane.

To use the gizmo in the viewer:

1. Click a structure to select it.
2. Open the Transform panel.
3. Make sure that the "Gizmo" box is checked.
4. Select the mode: T (translate), R (rotate) or S (scale).
5. Select W (world space) or L (local space).
6. Optional: enter the snap increments. A value of 0 turns snapping off.

The gizmo shows the handles of one mode at a time. The Transform panel also has fields for position, rotation, scale and a Reset button. For a slice plane, the scale mode shows the translate and rotate handles.

The same settings are available in code:

```rust
select_structure("SurfaceMesh", "mesh");
set_gizmo_visible(true);
set_gizmo_mode(GizmoMode::Rotate);
set_gizmo_space(GizmoSpace::Local);
set_gizmo_snap_rotate(15.0);    // degrees
set_gizmo_snap_translate(0.1);  // scene units
set_gizmo_snap_scale(0.1);

// Read or set the transform of the selected structure
let transform = get_selected_transform();
set_selected_transform(Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)));
```

The type names for `select_structure` are `PointCloud`, `SurfaceMesh`, `CurveNetwork`, `VolumeMesh`, `VolumeGrid` and `CameraView`. The transform applies to the model matrix of the structure.

---

## Save and Load a View

A view holds the camera (position, target, up vector, field of view, clip planes, projection, navigation style, up and front direction) and the render look of `Options` (background, ground plane, transparency, SSAO, SSAA).

In the viewer, open the View panel and click "Save View…" or "Load View…". The load moves the camera with a short flight.

In code:

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;
    register_point_cloud("pts", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

    // Arrange the view in the window, then close the window
    show();

    // The view of the last frame stays available after show() returns
    save_view_to_file("view.json")?;

    // Load the view and render it without a window
    load_view_from_file("view.json", ViewTransition::Instant)?;
    render_to_file("figure.png", 1280, 720)?;

    Ok(())
}
```

Other view functions:

```rust
// The view as a JSON string
let json = save_view_to_json()?;
load_view_from_json(&json, ViewTransition::FlyTo)?;

// The view as a Rust value
let mut state = current_view_state()?;
state.render.background_color = [1.0, 1.0, 1.0];
apply_view_state(&state, ViewTransition::Instant)?;
```

Rules:

- `current_view_state`, `save_view_to_json` and `save_view_to_file` return `NoActiveView` until a frame is rendered. A frame in the window or a headless render publishes the view.
- A loaded or applied view acts on the next frame. A headless render after a load uses the loaded camera and does not fit the camera to the scene.
- `ViewTransition::Instant` jumps to the view. `ViewTransition::FlyTo` moves the camera in about 0.4 s.
- In a JSON file, a missing field keeps its current value. The file must have `"version": 1`.
- The JSON format is Rust-native. It stores the field of view in radians. C++ Polyscope view files do not load.
- The front direction is the direction that the camera looks along. In C++ Polyscope, it is the side where the camera sits.

---

## Screenshots and Headless Rendering

### Screenshots

Take screenshots while the viewer is running:

```rust
// Auto-generated filename (screenshot_0000.png, screenshot_0001.png, ...)
screenshot();

// Specific filename (.png or .jpg)
screenshot_to_file("my_scene.png");

// With options: transparent background (PNG only)
screenshot_with_options(ScreenshotOptions {
    transparent_background: true,
});
screenshot_to_file_with_options("clear.png", ScreenshotOptions { transparent_background: true });
```

The viewer takes the screenshot on the next frame. The F12 key and the Screenshot button in the View panel also take a screenshot. A screenshot does not include the UI.

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

With screenshot options:

```rust
let options = ScreenshotOptions { transparent_background: true };
let pixels = render_to_image_with_options(800, 600, &options)?;
render_to_file_with_options("clear.png", 800, 600, &options)?;
```

A headless render uses the same frame passes as the window: ground plane, shadows, reflections, curve network tubes, volume grid gridcubes and isosurfaces.

`render_to_file` saves the same RGBA image that `render_to_image` returns, as PNG or JPEG.

The pixel rows go from top to bottom. With `transparent_background`, background pixels get alpha 0. A JPEG file has no alpha channel.

---

## UI Controls

### Mouse Controls

| Action | Input |
|--------|-------|
| Rotate/orbit | Left drag (Turntable, Free and Arcball navigation) |
| Look around | Left drag (First Person navigation) |
| Pan | Right drag, or Shift + left drag |
| Zoom | Scroll wheel |
| Select | Left click without drag and without Shift, Ctrl or Alt |
| Clear the selection | Right click without drag, or left click on the background |
| Set view center | Double-click, or Ctrl + Shift + left click (Cmd + Shift on macOS) |

Details:

- If nothing is selected, a single click selects after 0.35 s. Thus the first click of a double-click does not select. If a selection exists, the click applies at once.
- A double-click sets the view center on the point under the cursor, on any structure. The view flies there.
- A held Shift, Ctrl or Alt key stops the pick of a click.
- A release is a click if the mouse moved less than about 1% of the window size.
- Planar navigation does not rotate. First Person navigation does not pan or zoom. None navigation ignores all camera input.

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Esc | Exit |
| F12 | Screenshot |
| W, A, S, D | Move forward, left, backward, right (First Person navigation) |
| Q, E | Move up, down (First Person navigation) |

The "Reset View" button in the View panel resets the camera to the home view.

### UI Panels

The left panel has these sections:

- **View** - Background color, Reset View, Screenshot, Save View, Load View
- **Camera** - Navigation style, projection, up direction, field of view, clip planes, move speed
- **Scene Extents** - Automatic or manual scene bounds and length scale
- **Appearance** - Transparency mode, SSAA, frame rate limit, SSAO
- **Tone Mapping** - Exposure, white level, gamma
- **Materials** - Load a material from image files
- **Slice Planes** - Add, remove and edit slice planes
- **Transform** - Transform and gizmo of the selected structure
- **Ground Plane** - Mode, height, shadow darkness, reflection intensity
- **Groups** - Group visibility (only if groups exist)
- **Structures** - Registered structures, with their settings and quantities

The **Selection** panel on the right shows the selected structure and element.

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

The handles have fewer setters than the structure types. For the other setters, use the `with_*` functions:

```rust
// Mutable access
with_surface_mesh("mesh", |mesh| {
    mesh.set_surface_color(Vec3::new(1.0, 0.0, 0.0));
    // Access internal state, call multiple methods, etc.
});

// Immutable access
let vertex_count = with_surface_mesh_ref("mesh", |mesh| {
    mesh.num_vertices()
});
```

### Error Handling

polyscope-rs uses `Result<T, PolyscopeError>` for fallible operations:

```rust
use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;  // Returns Result

    let vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    let faces = vec![[0, 1, 2]];

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
cargo run --example volume_mesh_mixed_cells_demo
cargo run --example volume_grid_demo
cargo run --example camera_view_demo
cargo run --example slice_plane_demo
cargo run --example groups_and_gizmos_demo
cargo run --example ground_plane_demo
cargo run --example polygon_mesh_demo
cargo run --example materials_demo
cargo run --example transparency_demo
cargo run --example view_state_demo
```

---

## Next Steps

- Explore the [API documentation](https://docs.rs/polyscope-rs) (rustdoc)
- Check the [Feature Status](feature-status.md) for supported and missing features
- Read the [Development Guide](development-guide.md) for contributing
- See [Architecture Differences](architecture-differences.md) for C++ vs Rust details
