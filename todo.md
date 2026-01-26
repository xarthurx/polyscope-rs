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
- [x] **SurfaceMesh** - Triangles with vertex/face quantities, flat/smooth shading
- [x] **CurveNetwork** - Lines + tubes via compute shaders
- [x] **VolumeMesh** - Tet/hex cells, interior face detection, slice capping
- [x] **VolumeGrid** - Node/cell scalars, basic isosurface (marching cubes)
- [x] **CameraView** - Frustum visualization

#### Scene Features
- [x] Ground plane with shadows
- [x] Ground reflections
- [x] SSAO (Screen-Space Ambient Occlusion)
- [x] Tone mapping (multiple modes)
- [x] Slice planes (up to 4) with gizmo manipulation
- [x] Volume mesh slice capping with quantity interpolation
- [x] Groups (structure organization)
- [x] Transform gizmo (translate, rotate, scale - all modes combined)
- [x] GPU picking (pixel-perfect, element-level)
- [x] Screenshots (PNG export)

#### Materials & Color Maps
- [x] All 8 built-in materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal)
- [x] 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.)

---

## In Progress

### Transparency Rendering
Order-independent transparency for translucent surfaces. Required for:
- Semi-transparent meshes
- Overlapping structures visualization
- Alpha-blended quantities

---

## Planned Features (Priority Order)

### High Priority

1. **Transparency Rendering**
   - Depth peeling or weighted blended OIT
   - Per-structure transparency setting
   - Proper sorting for correct visual appearance

### Medium Priority

2. **RGBA Color Quantities**
   - Currently only RGB colors supported
   - Add alpha channel support for per-element transparency

3. **Floating Quantities**
   - Screen-space data visualization
   - Annotations, labels, billboards

### Low Priority

4. **Parameterization Quantities**
   - UV coordinates visualization
   - Checker pattern, grid lines on UV space

5. **Intrinsic Vectors**
   - Tangent-space vector visualization
   - Requires local coordinate frame computation

6. **One-Form Quantities**
   - Differential form visualization
   - Edge-based data display

7. **Full Polygon Mesh Support**
   - Arbitrary n-gon faces (not just triangles)
   - Proper triangulation for rendering

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
2. **Transparency**: No proper transparency rendering yet (simple alpha blending only)
3. **Color Quantities**: RGB only, no alpha channel support

---

## Notes

- All shaders written in WGSL (WebGPU Shading Language)
- No geometry shader support in wgpu - using compute shaders and instancing instead
- egui used instead of Dear ImGui for pure Rust, zero native dependencies
