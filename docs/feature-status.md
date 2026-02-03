# Feature Status & Roadmap

Feature parity tracking between polyscope-rs and C++ Polyscope 2.x.

## Structures

| Structure | C++ Polyscope | polyscope-rs | Notes |
|-----------|--------------|--------------|-------|
| Point Cloud | Full | Full | Sphere impostors via instanced rendering |
| Surface Mesh | Full | Full | Triangles + arbitrary polygons, full quantity support |
| Curve Network | Full | Full | Lines + tubes via compute shaders |
| Volume Mesh | Full | Full | Tet/hex, interior face detection, slice capping |
| Volume Grid | Full | Full | Node/cell scalars, gridcube + isosurface (marching cubes) |
| Camera View | Full | Full | Frustum visualization |
| Floating Quantities | Full | Full | Scalar/color images, depth/color/raw render images |

## Quantities

| Quantity Type | C++ Polyscope | polyscope-rs |
|---------------|--------------|--------------|
| Scalar | Yes | Yes |
| Vector | Yes | Yes |
| Color (RGB) | Yes | Yes |
| Color (RGBA) | Yes | Yes |
| Parameterization | Yes | Yes |
| Intrinsic Vector | Yes | Yes |
| One Form | Yes | Yes |

## Scene Features

| Feature | C++ Polyscope | polyscope-rs | Notes |
|---------|--------------|--------------|-------|
| Ground Plane | Yes | Yes | Tile/Shadow/Reflection modes |
| Ground Shadows | Yes | Yes | Shadow map with blur |
| Ground Reflections | Yes | Yes | Stencil-based |
| Tone Mapping | Yes | Yes | HDR pipeline |
| SSAO | No | Yes | polyscope-rs only |
| Transparency | Yes | Yes | Depth peeling (Pretty) + alpha blending (Simple) |
| Slice Planes | Yes | Yes | Max 4, with volume mesh capping |
| Groups | Yes | Yes | Hierarchical, visibility propagation, cascading toggle |
| Gizmos | Yes | Yes | Via egui (transform-gizmo-egui) |
| Picking | Yes | Yes | GPU-based, element-level |
| Screenshots | Yes | Yes | PNG/JPEG, transparent background |
| Headless Rendering | Yes | Yes | `render_to_image()` / `render_to_file()` without a window |
| RGBA Colors | Yes | Yes | Per-element alpha on all structures |
| Camera Flight Animation | Yes | Yes | Smooth "fly to" via quaternion interpolation, 0.4s default |
| Camera Navigation | Yes | Yes | Turntable, Free, Planar, Arcball, First-person |
| Ortho/Perspective | Yes | Yes | Toggle between projection modes |
| Double-click View Center | Yes | Yes | Set view center at click position |
| Reset View | Yes | Yes | Recompute home view from scene extents |
| Auto Scene Extents | Yes | Yes | Auto-compute with manual override |

## Materials & Color Maps

- All 8 built-in matcap materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal)
- Custom material loading (`load_blendable_material` / `load_static_material`)
- Per-structure material selection via `Structure` trait
- 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.)

## Completed Features

- [x] wgpu-based rendering engine (Vulkan/Metal/DX12/WebGPU)
- [x] egui UI integration (pure Rust, no native dependencies)
- [x] Global state management with thread-safe context
- [x] Structure registry with unique IDs
- [x] Quantity system (scalars, vectors, colors, parameterization, intrinsic vectors, one-forms)
- [x] Vector arrow rendering (fully capped, 120 verts/instance, auto-scaling)
- [x] Arbitrary polygon mesh support (`IntoFaceList` trait, fan triangulation)
- [x] Volume Grid isosurface (marching cubes) and gridcube rendering
- [x] Custom material loading (user-provided matcap textures)
- [x] Double-click to set view center (upstream commit 61fc32a)
- [x] Turntable orbit drift prevention (upstream commit 129c680)
- [x] Drag & drop file callback (upstream commit 0ff26c2)
- [x] Camera flight animation (smooth "fly to" on CameraView structures)
- [x] FOV-aware auto-fit camera (proper bounding sphere framing)
- [x] CameraView frustum visibility fixes (length_scale regeneration, color/thickness invalidation)

---

## Known Issues

- **Intermittent SIGSEGV on WSL2** — Under WSL2 with GPU passthrough, the application may occasionally crash with SIGSEGV inside the GPU driver. This is a WSL2/GPU driver issue, not a polyscope-rs bug. Native platforms are unaffected.
- **Pretty mode non-linear opacity** — Depth peeling renders both faces of closed meshes, giving effective alpha = `2a - a^2`. Matches C++ Polyscope.
- **Pretty mode f16 depth precision** — Min-depth uses `Rgba16Float` (WebGPU `R32Float` not blendable without `float32-blendable` feature). Requires epsilon `2e-3` vs C++'s `1e-6`. Closely spaced layers within 0.002 NDC depth may not be distinguished.

## Planned Work

### Polish
- [x] More examples and documentation (13 demos, getting-started guide)
- [ ] Platform testing (macOS, WebGPU targets) — postponed
- [x] API documentation (rustdoc with examples)

---

## Mouse Controls

| Action | Input |
|--------|-------|
| Rotate/orbit | Left drag |
| Pan | Shift + Left drag, or Right drag |
| Zoom | Scroll wheel |
| Select | Left click (no drag) |
| Set view center | Double-click |
| Deselect | Right click (no drag) |

## Technology Stack

| Component | C++ Polyscope | polyscope-rs |
|-----------|---------------|--------------|
| Graphics API | OpenGL 3.3+ | wgpu (Vulkan/Metal/DX12/WebGPU) |
| Shader Language | GLSL | WGSL |
| Windowing | GLFW | winit |
| Math | GLM | glam |
| UI | Dear ImGui | egui |
| Image Loading | stb_image | image |
| JSON | nlohmann/json | serde_json |
| Build | CMake | Cargo |

## Platform Support

| Platform | C++ Polyscope | polyscope-rs |
|----------|---------------|--------------|
| Windows | OpenGL | Vulkan/DX12 |
| macOS | OpenGL (deprecated) | Metal |
| Linux | OpenGL | Vulkan |
| Web | No | WebGPU (future) |
