# Phase 5: SurfaceMesh Implementation Design

**Date**: 2026-01-22
**Status**: Approved
**Purpose**: Full SurfaceMesh implementation with C++ Polyscope parity

---

## Overview

Phase 5 implements comprehensive SurfaceMesh support matching C++ Polyscope functionality. This includes polygon support (not just triangles), multiple shading modes, wireframe rendering, and all quantity types.

---

## 1. Data Structure

### Core Storage

```rust
pub struct SurfaceMesh {
    name: String,

    // Geometry (user-provided)
    vertices: Vec<Vec3>,
    faces: Vec<Vec<u32>>,  // Variable-size polygons

    // Computed data
    triangulation: Vec<UVec3>,       // Fan triangulation for rendering
    face_to_tri_map: Vec<Range<usize>>,  // Map face index to triangle range
    vertex_normals: Vec<Vec3>,       // Smooth shading
    face_normals: Vec<Vec3>,         // Flat shading
    corner_normals: Vec<Vec3>,       // Per-corner (tri-flat)
    edges: Vec<(u32, u32)>,          // Unique edges (sorted pairs)
    halfedges: Vec<Halfedge>,        // For halfedge quantities

    // Render options
    shade_style: ShadeStyle,
    edge_width: f32,
    edge_color: Vec3,
    backface_policy: BackfacePolicy,
    surface_color: Vec3,
    transparency: f32,

    // State
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // GPU resources
    gpu_buffers: Option<SurfaceMeshBuffers>,
}

pub struct Halfedge {
    vertex: u32,      // Vertex this halfedge points to
    face: u32,        // Face this halfedge belongs to
    twin: Option<u32>, // Twin halfedge (None if boundary)
    next: u32,        // Next halfedge in face
}
```

### Enums

```rust
pub enum ShadeStyle {
    Smooth,    // Interpolated vertex normals
    Flat,      // One normal per face
    TriFlat,   // One normal per triangle (for non-planar polygons)
}

pub enum BackfacePolicy {
    Identical,  // Same as front
    Different,  // Different color
    Custom,     // User-specified color
    Cull,       // Don't render
}
```

### Triangulation

Fan triangulation for polygons:
- Face `[v0, v1, v2, v3, v4]` becomes triangles `[v0,v1,v2]`, `[v0,v2,v3]`, `[v0,v3,v4]`
- Store mapping so face quantities can be applied to correct triangles
- Corner indices map 1:1 with triangulation vertices

---

## 2. Rendering

### Shading Modes

**Smooth shading:**
- Compute vertex normals as area-weighted average of incident face normals
- Interpolate normals across triangles
- Use `vertex_normals` buffer

**Flat shading:**
- One normal per face
- All vertices of a face use same normal
- Use `face_normals` buffer, expand to per-vertex in shader

**Tri-flat shading:**
- One normal per triangle
- For non-planar polygons, each triangle gets its own normal
- Use `corner_normals` buffer (3 normals per triangle)

### Wireframe

Barycentric coordinate approach (no geometry shader needed):
- Store barycentric coords `[1,0,0]`, `[0,1,0]`, `[0,0,1]` per triangle vertex
- Fragment shader checks distance to edge
- Alpha-blend wireframe color when near edge
- Edge width controlled by `edge_width` uniform

```wgsl
// In fragment shader
let d = min(bary.x, min(bary.y, bary.z));
let edge_factor = smoothstep(0.0, edge_width * fwidth(d), d);
color = mix(edge_color, surface_color, edge_factor);
```

### Backface Handling

In fragment shader:
- `gl_FrontFacing` equivalent in wgpu: `front_facing` builtin
- Apply `backface_policy` to determine color/culling

### GPU Buffers

```rust
pub struct SurfaceMeshBuffers {
    vertex_buffer: wgpu::Buffer,       // positions
    index_buffer: wgpu::Buffer,        // triangulation indices
    normal_buffer: wgpu::Buffer,       // normals (mode-dependent)
    barycentric_buffer: wgpu::Buffer,  // for wireframe
    uniform_buffer: wgpu::Buffer,      // transforms, colors, options
}
```

---

## 3. Quantities

### Vertex Quantities
| Type | Description |
|------|-------------|
| `VertexScalarQuantity` | Scalar per vertex, color-mapped |
| `VertexColorQuantity` | RGB color per vertex |
| `VertexVectorQuantity` | 3D vector per vertex (arrows) |
| `VertexTangentVectorQuantity` | Tangent vector in surface (ribbon/arrow) |
| `VertexParameterizationQuantity` | 2D UV coords, checkerboard visualization |

### Face Quantities
| Type | Description |
|------|-------------|
| `FaceScalarQuantity` | Scalar per face |
| `FaceColorQuantity` | RGB color per face |
| `FaceVectorQuantity` | Vector at face centroid |
| `FaceTangentVectorQuantity` | Tangent vector in face plane |

### Edge Quantities
| Type | Description |
|------|-------------|
| `EdgeScalarQuantity` | Scalar per unique edge |
| `EdgeColorQuantity` | RGB per edge |
| `EdgeVectorQuantity` | Vector at edge midpoint |

### Halfedge/Corner Quantities
| Type | Description |
|------|-------------|
| `HalfedgeScalarQuantity` | Scalar per halfedge |
| `HalfedgeVectorQuantity` | Vector per halfedge |
| `CornerParameterizationQuantity` | UV per corner (more flexible than vertex) |

### Quantity Rendering

**Scalars:** Same as point cloud - 1D color map texture lookup
**Colors:** Direct RGB, interpolated or constant per element
**Vectors:** Instanced arrow rendering (reuse from PointCloud)
**Tangent vectors:** Project onto surface, render as ribbons or surface arrows
**Parameterization:** Checkerboard pattern based on UV, configurable scale

---

## 4. UI Integration

### Structure Controls (Left Panel)

```
SurfaceMesh: bunny
├── [x] Enabled
├── Vertices: 34,834
├── Faces: 69,451
├── Shade style: [Smooth ▼]
├── Surface color: [███]
├── Transparency: [====|====] 1.0
├── Backface: [Identical ▼]
├── Wireframe
│   ├── [x] Show edges
│   ├── Width: [==|======] 1.0
│   └── Color: [███]
└── Quantities
    ├── [x] curvature (scalar)
    ├── [ ] normals (vector)
    └── [x] texture_coords (param)
```

### Quantity Controls

**Scalar:** Colormap selector, range min/max, isolines toggle
**Color:** Just enabled checkbox
**Vector:** Length scale, radius, color
**Tangent Vector:** Length, width, style (arrow/ribbon)
**Parameterization:** Checker scale, checker colors, style (checker/grid)

### Picking

Element types for pick result:
- `Vertex(index)` - click on/near vertex
- `Face(index)` - click on face interior
- `Edge(index)` - click on/near edge

Pick buffer encodes:
- Structure ID (8 bits)
- Element type (2 bits): 0=face, 1=vertex, 2=edge
- Element index (22 bits)

Selection panel shows:
- Element type and index
- Position (vertex pos, face centroid, edge midpoint)
- All quantity values at that element

---

## 5. Implementation Summary

### Tasks (~12-15)

1. **Data structure refactor** - Polygon support, computed fields
2. **Triangulation** - Fan triangulation with face mapping
3. **Normal computation** - Vertex, face, corner normals
4. **Edge/halfedge extraction** - Build connectivity
5. **GPU buffer management** - Create/update buffers
6. **Surface shader** - Basic rendering with shading modes
7. **Wireframe shader** - Barycentric edge rendering
8. **Backface handling** - Policy implementation
9. **Vertex quantities** - Scalar, color, vector
10. **Face quantities** - Scalar, color, vector
11. **Edge quantities** - Scalar, color, vector
12. **Tangent vectors** - Surface-projected rendering
13. **Parameterization** - UV checkerboard visualization
14. **UI controls** - Structure and quantity panels
15. **Picking** - Vertex/face/edge selection

### Dependencies

- Reuse `ColorMapRegistry` from Phase 3
- Reuse arrow instancing from `PointCloudVectorQuantity`
- Extend `PickResult` for mesh element types

### Testing

- Unit tests for triangulation, normal computation
- Visual tests for shading modes
- Quantity rendering verification
- Pick accuracy tests

---

## Files

| File | Action |
|------|--------|
| `crates/polyscope-structures/src/surface_mesh/mod.rs` | Rewrite for polygon support |
| `crates/polyscope-structures/src/surface_mesh/triangulation.rs` | New - fan triangulation |
| `crates/polyscope-structures/src/surface_mesh/normals.rs` | New - normal computation |
| `crates/polyscope-structures/src/surface_mesh/connectivity.rs` | New - edge/halfedge |
| `crates/polyscope-structures/src/surface_mesh/quantities/*.rs` | New - all quantity types |
| `crates/polyscope-render/src/shaders/surface_mesh.wgsl` | New - mesh shader |
| `crates/polyscope-render/src/shaders/surface_mesh_pick.wgsl` | New - pick shader |
| `crates/polyscope-render/src/surface_mesh_renderer.rs` | New - GPU management |
| `crates/polyscope-ui/src/structure_ui.rs` | Extend for SurfaceMesh |
| `crates/polyscope-ui/src/quantity_ui.rs` | Extend for mesh quantities |
| `crates/polyscope-core/src/pick.rs` | Extend PickResult |
