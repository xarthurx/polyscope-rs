# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

polyscope-rs is a Rust-native 3D visualization library for geometric data, ported from C++ [Polyscope](https://polyscope.run). **Core paradigm**: Structures (geometric objects) + Quantities (data on structures).

## Build Commands

```bash
cargo build            # Build all crates
cargo build --release  # Release mode
cargo test             # Run tests
cargo check            # Check without building
cargo fmt              # Format code
cargo clippy           # Lint
```

## Workspace Structure

```
crates/
├── polyscope-core/       # Core traits, registry, state management
├── polyscope-render/     # wgpu rendering backend
├── polyscope-ui/         # egui UI integration
├── polyscope-structures/ # Structure implementations
└── polyscope/            # Main crate, re-exports all sub-crates
```

## Key Architecture

- **State**: Global `OnceLock<RwLock<Context>>` accessed via `with_context()` / `with_context_mut()`
- **Bind group layout**: Group 0 = per-object uniforms, Group 1 = slice planes/reflection, Group 2 = matcap textures
- **Render loop**: `app.rs` — shadow map → slice planes → ground plane → depth prepass → main pass (points/curves/vectors) → surface mesh pass → depth peel → SSAO → tone mapping

## Critical: Model Transform Propagation

GPU data from structure geometry **must** be transformed by the model matrix. When adding new rendered quantities:
1. Include `model: mat4x4<f32>` in shader uniform struct
2. Apply in vertex shader: positions (w=1), directions (w=0)
3. Pass `structure.transform()` every frame via `update_uniforms()`
4. Never assume GPU-baked positions are in world space — they are local/object space

## Adding New Structures/Quantities

**Structure**: Create module in `polyscope-structures/src/` → implement `Structure` + `HasQuantities` traits → add registration function in `polyscope` crate → add tests.

**Quantity**: Implement `Quantity` trait → add marker trait → add GPU resources (`render_data`, `init_gpu_resources`, `update_uniforms`) → add active accessor on parent structure → add `auto_scale()` in registration → add init/update/draw in `app.rs` → add UI in `polyscope-ui`.

## Known Issues

- **Pretty mode non-linear opacity**: Depth peeling renders both faces of closed meshes, giving effective alpha = `2α - α²`. Matches C++ Polyscope. Transparency only becomes visible at low opacity values.
- **Pretty mode f16 depth precision**: Min-depth uses `Rgba16Float` (WebGPU `R32Float` not blendable without `float32-blendable` feature). Requires epsilon `2e-3` in `surface_mesh_peel.wgsl` vs C++'s `1e-6` (24-bit depth). Closely spaced layers within 0.002 NDC depth may not be distinguished.
## Reference

- **Local C++ Polyscope source**: `~/repo/polyscope` — always check for implementation details
- C++ Polyscope repo: https://github.com/nmwsharp/polyscope
- Polyscope docs: https://polyscope.run
- Architecture comparison: `docs/architecture-differences.md`
- Design document: `docs/plans/2026-01-21-polyscope-rs-design.md`
