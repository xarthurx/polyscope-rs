# Development Guide

Conventions and patterns for developing polyscope-rs.

## Build Commands

```bash
cargo build            # Build all crates
cargo build --release  # Release mode
cargo test             # Run tests
cargo check            # Check without building
cargo fmt              # Format code
cargo clippy           # Lint
```

Rust edition 2024, MSRV 1.85.

## Workspace Structure

```
crates/
├── polyscope-core/       # Core traits, registry, state management
├── polyscope-render/     # wgpu rendering backend
├── polyscope-ui/         # egui UI integration
├── polyscope-structures/ # Structure implementations
└── polyscope/            # Main crate, re-exports all sub-crates
```

## Adding a New Structure

1. Create module in `polyscope-structures/src/`
2. Implement `Structure` trait (with `as_any`, `as_any_mut`, `bounding_box`, `length_scale`, `transform`, etc.)
3. Implement `HasQuantities` if it supports quantities
4. Add registration function in main `polyscope` crate
5. Add GPU init/draw/pick code in `polyscope/src/app/render.rs`
6. Add UI in `polyscope-ui/src/structure_ui.rs`
7. Add tests

## Adding a New Quantity

1. Implement `Quantity` trait (with `name`, `kind`, `is_enabled`, `set_enabled`, `as_any`, `as_any_mut`, `refresh`)
2. Add marker trait if needed (`VertexQuantity`, `FaceQuantity`, etc.)
3. Add GPU rendering fields: `render_data`, `init_gpu_resources()`, `update_uniforms()`, `render_data()`
4. Add active accessor on parent structure (e.g., `active_vertex_vector_quantity()`)
5. Add `auto_scale()` call in the structure's registration method
6. Add init/update/draw code in `polyscope/src/app/render.rs`
7. Add convenience method on parent structure (e.g., `mesh.add_vector_quantity(...)`)
8. Add UI controls in `polyscope-ui`

## Critical: Model Transform Propagation

GPU data from structure geometry **must** be transformed by the model matrix. When adding new rendered quantities:

1. Include `model: mat4x4<f32>` in shader uniform struct
2. Apply in vertex shader: positions (w=1), directions (w=0)
3. Pass `structure.transform()` every frame via `update_uniforms()`
4. Never assume GPU-baked positions are in world space — they are local/object space

Failure to do this causes quantities to stay frozen when the user moves/rotates the structure via gizmos.

## Shader Development

All shaders are WGSL (WebGPU Shading Language). No geometry shader support in wgpu — use compute shaders and instancing instead.

### Bind Group Layout Convention

- **Group 0**: Per-object uniforms (model matrix, colors, parameters)
- **Group 1**: Slice planes / reflection uniforms
- **Group 2**: Matcap textures (4-channel blend or single-texture)

All scene shaders call `light_surface_matcap(view_normal, base_color)` via Group 2 for consistent lighting.

### Uniform Buffer Binding

Always set explicit `min_binding_size` (via `NonZeroU64`) rather than `None`. This works around [wgpu Issue #7359](https://github.com/gfx-rs/wgpu/issues/7359) where late validation can cross-contaminate between pipelines.

**WGSL alignment caveat**: `vec3<T>` aligns to 16 bytes, not 12. Padding fields must use scalar types (e.g., `_pad0: u32`) rather than `vec3<u32>` to match Rust `#[repr(C)]` struct sizes.

## UI Layout Convention

For label+widget rows (sliders, drag values, color pickers), always use `egui::Grid` with 2 columns instead of `ui.horizontal` + `ui.add_sized`. Grid auto-sizes the label column and left-aligns labels. For buttons, `ui.add_sized` with a fixed width is fine.

## State Management

Global state is managed via `OnceLock<RwLock<Context>>`:
- `with_context(|ctx| ...)` — Read access
- `with_context_mut(|ctx| ...)` — Write access

## Render Pipeline Overview

The render loop in `app/render.rs` follows this order:

1. Shadow map pass
2. Slice plane visualization
3. Ground plane (with shadows/reflections)
4. Depth prepass
5. Main geometry pass (points, curves, vectors)
6. Surface mesh pass
7. Depth peel transparency (multi-pass)
8. SSAO
9. Tone mapping
10. egui overlay

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

## Reference

- **Local C++ Polyscope source**: `~/repo/polyscope` — check for implementation details
- C++ Polyscope repo: https://github.com/nmwsharp/polyscope
- Polyscope docs: https://polyscope.run
- Design document: `docs/plans/2026-01-21-polyscope-rs-design.md`
