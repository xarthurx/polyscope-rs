# Polyscope-rs TODO

## Current Status

polyscope-rs has reached substantial feature parity with C++ Polyscope for core functionality.

### Completed Features

#### Core Infrastructure
- [x] wgpu-based rendering engine (Vulkan/Metal/DX12/WebGPU)
- [x] egui UI integration (pure Rust, no native dependencies)
- [x] Global state management with thread-safe context
- [x] Structure registry with unique IDs
- [x] Quantity system (scalars, vectors, colors, parameterization, intrinsic vectors, one-forms)
- [x] Vector arrow rendering — Fully capped arrows (shaft + cone caps, 120 verts/instance), auto-scaling at registration

#### Structures
- [x] **PointCloud** - Full feature parity with sphere impostors
- [x] **SurfaceMesh** - Triangles with vertex/face scalar/color/vector quantities, parameterization (checker/grid/local), intrinsic vectors (tangent-space, n-fold symmetry), one-forms (edge arrows), flat/smooth shading
- [x] **CurveNetwork** - Lines + tubes via compute shaders, node/edge scalar/color/vector quantities
- [x] **VolumeMesh** - Tet/hex cells, interior face detection, slice capping, vertex/cell scalar/color/vector quantities
- [x] **VolumeGrid** - Node/cell scalars, wireframe bounding box visualization
- [x] **CameraView** - Frustum visualization
- [x] **FloatingQuantity** - Scalar images, color images, depth/color/raw render images

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

### Bugs / Known Issues

- [ ] **SSAA crash on factor change** — Switching SSAA from 1x to 2x/4x triggers a buffer size mismatch (`Buffer is bound with size 16 where the shader expects 32`). Root cause is in the shadow system re-implementation (commit `e7697cf`), which introduced a uniform buffer sizing bug that surfaces when textures are recreated. Blocked on engine.rs refactor.

### Tier 2 — Broader Feature Additions

- [ ] **Matcap Material System** - Current materials use parametric Phong lighting (ambient/diffuse/specular/shininess scalars). C++ Polyscope uses texture-based matcap rendering (view-space normal → HDR texture lookup) which gives its distinctive visual quality. Gaps: (1) No matcap texture loading or rendering; (2) MaterialUniforms not wired to surface mesh / point cloud shaders; (3) Only CurveNetwork exposes `set_material()`; SurfaceMesh and PointCloud do not; (4) Missing `concrete` material; (5) No custom material loading API (`loadBlendableMaterial` / `loadStaticMaterial`).
- [ ] **RGBA Color Quantities** - Currently only RGB colors supported; add alpha channel support for per-element transparency
- [ ] **Full Polygon Mesh Support** - Arbitrary n-gon faces (not just triangles); proper triangulation for rendering and quantities

### Tier 3 — Advanced Quantity Types

- [x] **Parameterization Quantities** - UV coordinates visualization with checker, grid, local check, and local radial styles. Both vertex and corner variants.
- [x] **Intrinsic Vectors** - Tangent-space vector visualization with auto-computed tangent basis and n-fold symmetry (1=vector, 2=line, 4=cross).
- [x] **One-Form Quantities** - Differential form visualization as edge-midpoint arrows with orientation support.
- [x] **Floating Quantities** - Scalar images (colormap-based), color images (direct RGB), depth render images, color render images, and raw color images for screen-space data.

All Tier 3 quantities have full GPU rendering pipelines (init/update/draw), auto-scaling, and egui UI controls.

### Upstream Follow-ups (from C++ Polyscope)

- [ ] **Double-click to set view center** — Port upstream commit 61fc32a
- [ ] **Turntable orbit drift prevention** — Port upstream commit 129c680
- [ ] **Drag & drop file callback** — Port upstream commit 0ff26c2

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
