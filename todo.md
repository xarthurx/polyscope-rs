# Polyscope-rs TODO

## Current Status

polyscope-rs has reached substantial feature parity with C++ Polyscope for core functionality.

### Completed Features

#### Core Infrastructure
- [x] wgpu-based rendering engine (Vulkan/Metal/DX12/WebGPU)
- [x] egui UI integration (pure Rust, no native dependencies)
- [x] Global state management with thread-safe context
- [x] Structure registry with unique IDs
- [x] Quantity system (scalars, vectors, colors)

#### Structures
- [x] **PointCloud** - Full feature parity with sphere impostors
- [x] **SurfaceMesh** - Triangles with vertex/face scalar/color/vector quantities, flat/smooth shading
- [x] **CurveNetwork** - Lines + tubes via compute shaders, node/edge scalar/color/vector quantities
- [x] **VolumeMesh** - Tet/hex cells, interior face detection, slice capping, vertex/cell scalar/color/vector quantities
- [x] **VolumeGrid** - Node/cell scalars, wireframe bounding box visualization
- [x] **CameraView** - Frustum visualization

#### Scene Features
- [x] Ground plane with shadows
- [x] Ground reflections
- [x] SSAO (Screen-Space Ambient Occlusion)
- [x] Tone mapping (multiple modes)
- [x] Transparency (Weighted Blended OIT)
- [x] Slice planes (up to 4) with gizmo manipulation
- [x] Volume mesh slice capping with quantity interpolation
- [x] Groups (structure organization)
- [x] Transform gizmo (translate, rotate, scale via egui)
- [x] GPU picking (pixel-perfect, element-level)
- [x] Screenshots (PNG/JPEG export, transparent background)

#### Materials & Color Maps
- [x] All 8 built-in materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal)
- [x] 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.)

---

## Planned Features (Tiered Priority)

### Tier 2 — Broader Feature Additions

- [ ] **RGBA Color Quantities** - Currently only RGB colors supported; add alpha channel support for per-element transparency
- [ ] **Full Polygon Mesh Support** - Arbitrary n-gon faces (not just triangles); proper triangulation for rendering and quantities

### Tier 3 — Advanced Quantity Types

- [ ] **Parameterization Quantities** - UV coordinates visualization, checker pattern, grid lines on UV space
- [ ] **Intrinsic Vectors** - Tangent-space vector visualization, requires local coordinate frame computation
- [ ] **One-Form Quantities** - Differential form visualization, edge-based data display
- [ ] **Floating Quantities** - Screen-space data visualization, annotations, labels, billboards

### Tier 4 — Polish

- [ ] More examples and documentation
- [ ] Platform testing — macOS and WebGPU targets
- [ ] Integration tests — visual regression testing beyond unit tests

---

## Mouse Controls (matching C++ Polyscope)

- **Left drag** (no modifiers): Rotate/orbit camera
- **Shift + Left drag**: Pan camera
- **Right drag**: Pan camera
- **Scroll wheel**: Zoom
- **Left click** (no drag): Select structure at click position
- **Right click** (no drag): Deselect

---

## Known Limitations

1. **Polygon Meshes**: Only triangles fully supported; arbitrary polygons need triangulation
2. **Color Quantities**: RGB only, no alpha channel support

---

## Notes

- All shaders written in WGSL (WebGPU Shading Language)
- No geometry shader support in wgpu - using compute shaders and instancing instead
- egui used instead of Dear ImGui for pure Rust, zero native dependencies
