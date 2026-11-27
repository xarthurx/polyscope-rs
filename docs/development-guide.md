# Development Guide

Conventions and patterns for developing polyscope-rs.

## Build Commands

```bash
cargo build            # Build all crates
cargo build --release  # Release mode
cargo test             # Run tests
cargo check            # Check without building
cargo fmt              # Format code
cargo clippy           # Lint (must give zero warnings)
```

Rust edition 2024, minimum Rust version 1.96 (`rust-version` in the workspace `Cargo.toml`).

The engine requests only the Vulkan, Metal, DirectX 12 and WebGPU backends of wgpu (`SUPPORTED_BACKENDS` in `polyscope-render/src/engine/mod.rs`). The OpenGL backend is not a target: some shaders sample depth textures in a way that it cannot compile. Without a supported backend, engine creation returns `RenderError::AdapterCreationFailed`. The GPU tests skip when no adapter is available.

## Workspace Structure

```
crates/
├── polyscope-core/       # Core traits, registry, state management, options, view state
├── polyscope-render/     # wgpu rendering backend, shaders, colormaps, materials
├── polyscope-ui/         # egui UI integration
├── polyscope-structures/ # Structure implementations
└── polyscope/            # Main crate (package polyscope-rs); re-exports the public types of the sub-crates
```

The main crate re-exports `Vec2`, `Vec3`, `Vec4` and `Mat4` from glam, but not the `glam` crate. Users who need other glam types add `glam = "0.34"` to their own `Cargo.toml`.

## Adding a New Structure

1. Create module in `polyscope-structures/src/`
2. Implement `Structure` trait (with `as_any`, `as_any_mut`, `bounding_box`, `length_scale`, `transform`, etc.)
3. Implement `HasQuantities` if it supports quantities
4. Add registration function and handle in the main `polyscope` crate (`polyscope/src/<structure>.rs`)
5. Add GPU code in `polyscope/src/app/`:
   - GPU initialization and buffer updates: `render_init.rs` (shared by the window and the headless path)
   - Draw calls: `render_scene.rs`
   - Pick resources and the pick pass: `render.rs`
   - Screenshot and headless frame: `render_capture.rs`
6. Add UI in `polyscope-ui/src/structure_ui.rs`
7. Add tests

## Adding a New Quantity

1. Implement the `Quantity` trait: `as_any`, `as_any_mut`, `name`, `structure_name`, `kind`, `is_enabled`, `set_enabled`, `build_ui`, `refresh`, `data_size`. Override `clear_gpu_resources` if the quantity holds GPU data.
2. Add marker trait if needed (`VertexQuantity`, `FaceQuantity`, `EdgeQuantity`, `CellQuantity`)
3. Add GPU rendering fields: `render_data`, `init_gpu_resources()`, `update_uniforms()`, `render_data()`
4. Add active accessor on parent structure (e.g., `active_vertex_vector_quantity()`)
5. Call `auto_scale()` for vector-like quantities in the method of the structure that adds them
6. Add init/update/draw code in `polyscope/src/app/` (see the file list above)
7. Add the add method on the parent structure (e.g., `mesh.add_vertex_vector_quantity(...)`) and, if useful, on its handle
8. Add UI controls in `polyscope-ui`

A new quantity starts disabled, as in C++ Polyscope.

## Critical: Model Transform Propagation

GPU data from structure geometry **must** be transformed by the model matrix. When you add a rendered quantity:

1. Include `model: mat4x4<f32>` in shader uniform struct
2. Apply in vertex shader: positions (w=1), directions (w=0)
3. Pass `structure.transform()` every frame via `update_uniforms()`
4. Never assume GPU-baked positions are in world space — they are local/object space

If a quantity ignores the model matrix, it stays in place when the user moves or rotates the structure with the gizmo. The pick shaders need the model matrix too.

## Shader Development

All shaders are WGSL (WebGPU Shading Language). No geometry shader support in wgpu — use compute shaders and instancing instead.

### Bind Group Layout Convention

- **Group 0**: Per-object uniforms (model matrix, colors, parameters)
- **Group 1**: Slice planes / reflection uniforms
- **Group 2**: Matcap textures (4-channel blend or single-texture)

The lit scene shaders define and call `light_surface_matcap(normal, color)`, with the matcap textures of Group 2, for consistent lighting. WGSL has no include mechanism, so each shader has its own copy of this function.

### Uniform Buffer Binding

Always set explicit `min_binding_size` (via `NonZeroU64`) rather than `None`. This works around [wgpu Issue #7359](https://github.com/gfx-rs/wgpu/issues/7359) where late validation can cross-contaminate between pipelines.

**WGSL alignment caveat**: `vec3<T>` aligns to 16 bytes, not 12. Padding fields must use scalar types (e.g., `_pad0: u32`) rather than `vec3<u32>` to match Rust `#[repr(C)]` struct sizes.

## Colormap Data

The built-in colormaps come from the C++ Polyscope tables. The data flow:

1. `scripts/extract_colormaps.py` reads the `CM_*` tables in `src/render/color_maps.cpp` of the C++ source (`~/repo/polyscope`).
2. The script writes one file per colormap to `crates/polyscope-render/data/colormaps/<name>.bin`. Each file holds RGB triples of little-endian `f32` values.
3. `crates/polyscope-render/src/color_maps.rs` embeds the files with `include_bytes!` in `BUILTIN_COLOR_MAP_TABLES`, in the C++ order. `BUILTIN_COLOR_MAP_NAMES` holds the names.
4. `ColorMapRegistry::new()` decodes the tables. The `RenderEngine` owns one registry (`engine.color_maps`).
5. Each frame, the structures receive the registry in `update_gpu_buffers`. A scalar quantity computes one color per element on the CPU with `compute_colors`. Volume grid gridcubes upload the colormap as a 1D texture instead.

The quantity panels offer `BUILTIN_COLOR_MAP_NAMES`. The volume grid panel offers the names of the registry.

To add a built-in colormap:

1. Add the C++ table name and the user name to `COLORMAPS` in `scripts/extract_colormaps.py`.
2. Run `python3 scripts/extract_colormaps.py`.
3. Add the entry to `BUILTIN_COLOR_MAP_TABLES` and change the array length.
4. Run `cargo test -p polyscope-render`.

## UI Layout Convention

For label+widget rows (sliders, drag values, color pickers), always use `egui::Grid` with 2 columns instead of `ui.horizontal` + `ui.add_sized`. Grid auto-sizes the label column and left-aligns labels. For buttons, `ui.add_sized` with a fixed width is fine.

## State Management

Global state is managed via `OnceLock<RwLock<Context>>`:
- `with_context(|ctx| ...)` — Read access
- `with_context_mut(|ctx| ...)` — Write access

`Context::options` (`Options`) is the source of truth for the render look: background, ground plane, transparency, SSAA, SSAO and the frame rate limit. The UI panels read these values each frame and write their edits back. A saved view (`ViewState`) copies the render-look fields, but not `max_fps`.

## Render Pipeline Overview

The windowed frame in `app/render.rs` follows this order:

1. Compute pass for curve network tubes
2. Shadow map pass (surface meshes)
3. Slice plane visualization
4. Ground plane (stencil, ground, reflected structures)
5. Depth prepass of surface meshes (Pretty transparency only)
6. Main geometry pass (points, curves, vectors, camera views)
7. Surface mesh and volume mesh pass (HDR color + normal G-buffer), with volume grid isosurfaces and gridcubes
8. Depth peel transparency (multi-pass, Pretty transparency only)
9. SSAO (only with an SSAA factor of 1)
10. Tone mapping (with the SSAA downsample)
11. egui overlay
12. Pick pass (GPU picking; a click reads one pixel of it)

The pick pass comes last, so it uses the uniforms and the tube geometry of the image on the screen. The frame preparation writes the pick uniforms together with the render uniforms; the pick pass writes only the first pick index of each structure.

## API Patterns

### C++ vs Rust

```cpp
// C++ Polyscope
polyscope::PointCloud* pc = polyscope::registerPointCloud("my points", points);
pc->addScalarQuantity("height", heights);
pc->setPointRadius(0.01);
```

```rust
// polyscope-rs
register_point_cloud("my points", points);
with_point_cloud("my points", |pc| {
    pc.add_scalar_quantity("height", heights);
    pc.set_point_radius(0.01);
});
```

### Migration Notes

- **Pointers to handles**: C++ uses raw pointers; Rust uses handle structs or closure-based `with_*` access
- **Error handling**: C++ exceptions → Rust `Result<T, E>`
- **Vector types**: `glm::vec3` → `glam::Vec3`
- **Face format**: polyscope-rs accepts `UVec3` (triangles), `[u32; 3]`, or `Vec<Vec<u32>>` (polygons) via `IntoFaceList`
- **Initialization**: `init()` returns `Result`, use `?` or `unwrap()`
- **Thread safety**: Global state uses `RwLock`
- **Camera**: no global camera functions; use `apply_view_state` or the UI
- **Front direction**: the direction that the camera looks along (C++: the side where the camera sits)

[Feature Status & Roadmap](feature-status.md) lists the C++ API that has no Rust equivalent yet.

## Reference

- **Local C++ Polyscope source**: `~/repo/polyscope` — check for implementation details
- C++ Polyscope repo: https://github.com/nmwsharp/polyscope
- Polyscope docs: https://polyscope.run
- Design document: `docs/plans/2026-01-21-polyscope-rs-design.md`
