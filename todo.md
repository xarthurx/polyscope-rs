# Polyscope-rs TODO

## Current Status

polyscope-rs has reached full feature parity with C++ Polyscope 2.x for all core functionality.

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
- [x] **SurfaceMesh** - Triangles + arbitrary polygons via `IntoFaceList` trait, full quantity support (vertex/face scalar/color/vector, parameterization, intrinsic vectors, one-forms), RGBA alpha colors
- [x] **CurveNetwork** - Lines + tubes via compute shaders, node/edge scalar/color/vector quantities
- [x] **VolumeMesh** - Tet/hex cells, interior face detection, slice capping, vertex/cell scalar/color/vector quantities
- [x] **VolumeGrid** - Node/cell scalars, wireframe bounding box visualization
- [x] **CameraView** - Frustum visualization
- [x] **FloatingQuantity** - Scalar images, color images, depth/color/raw render images

#### Scene Features
- [x] Ground plane with shadows
- [x] Ground reflections (with slice plane clipping)
- [x] SSAO (Screen-Space Ambient Occlusion)
- [x] Tone mapping (multiple modes)
- [x] Transparency (Depth Peeling / Pretty + Simple + None)
- [x] Slice planes (up to 4) with gizmo manipulation
- [x] Volume mesh slice capping with quantity interpolation
- [x] Groups (structure organization, hierarchical)
- [x] Transform gizmo (translate, rotate, scale via egui)
- [x] GPU picking (pixel-perfect, element-level)
- [x] Screenshots (PNG/JPEG export, transparent background)
- [x] Reset View button (recomputes home view from scene extents)
- [x] Auto-compute scene extents with manual override (editable bbox when disabled)

#### Camera Navigation
- [x] Turntable orbit (gimbal-lock protected, view-matrix based)
- [x] Free orbit (unconstrained rotation)
- [x] Planar navigation (2D pan/zoom)
- [x] Arcball rotation
- [x] First-person navigation
- [x] Double-click to set view center
- [x] Orthographic / Perspective projection toggle

#### Materials & Color Maps
- [x] All 8 built-in matcap materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal)
- [x] Per-structure material selection via `Structure` trait
- [x] 4-channel blend materials (R/G/B/K textures)
- [x] Single-texture materials
- [x] 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.)

#### RGBA Color Support
- [x] Per-vertex RGBA colors on SurfaceMesh
- [x] Per-face RGBA colors on SurfaceMesh
- [x] Per-element alpha on PointCloud, CurveNetwork, VolumeMesh

---

## Planned Features (Tiered Priority)

### Bugs / Known Issues

- [ ] **Intermittent SIGSEGV on WSL2** — Under Windows Subsystem for Linux 2 with GPU passthrough, the application may occasionally crash with SIGSEGV inside the GPU driver. This is a WSL2/GPU driver instability issue, not a polyscope-rs bug. Native platforms are unaffected.

### Tier 2 — Remaining Gaps

- [ ] **Volume Grid Isosurface** - Marching cubes isosurface extraction (currently node/cell scalars only)
- [ ] **Custom Material Loading** - User-provided matcap textures (`loadBlendableMaterial` / `loadStaticMaterial`)

### Tier 3 — Polish

- [ ] More examples and documentation
- [ ] Platform testing — macOS and WebGPU targets
- [ ] Integration tests — visual regression testing beyond unit tests
- [ ] API documentation (rustdoc)

---

## Upstream Follow-ups (from C++ Polyscope)

- [x] **Double-click to set view center** — Port upstream commit 61fc32a
- [x] **Turntable orbit drift prevention** — Port upstream commit 129c680
- [x] **Drag & drop file callback** — Port upstream commit 0ff26c2

---

## Mouse Controls (matching C++ Polyscope)

- **Left drag** (no modifiers): Rotate/orbit camera
- **Shift + Left drag**: Pan camera
- **Right drag**: Pan camera
- **Scroll wheel**: Zoom
- **Left click** (no drag): Select structure at click position
- **Double-click**: Set view center at click position
- **Right click** (no drag): Deselect

---

## Notes

- All shaders written in WGSL (WebGPU Shading Language)
- No geometry shader support in wgpu - using compute shaders and instancing instead
- egui used instead of Dear ImGui for pure Rust, zero native dependencies
- Rust edition 2024, MSRV 1.85
