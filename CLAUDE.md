# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

polyscope-rs is a Rust-native 3D visualization library for geometric data, ported from C++ [Polyscope](https://polyscope.run). **Core paradigm**: Structures (geometric objects) + Quantities (data on structures). Version 0.5.4, ~100% feature parity with C++ Polyscope 2.x.

## Build Commands

```bash
cargo build            # Build all crates
cargo build --release  # Release mode
cargo test             # Run tests
cargo clippy           # Lint (must be zero warnings)
cargo fmt              # Format code
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

- **State**: Global `OnceLock<RwLock<Context>>` via `with_context()` / `with_context_mut()`
- **Bind groups**: Group 0 = per-object uniforms, Group 1 = slice planes/reflection, Group 2 = matcap textures
- **Render loop** (`app/render.rs`): shadow map -> slice planes -> ground plane -> depth prepass -> main pass -> surface mesh -> depth peel -> SSAO -> tone mapping
- **Headless rendering** (`headless.rs`): `render_to_image()` / `render_to_file()` create a throwaway `App` + headless `RenderEngine`, clear stale GPU resources via `Structure::clear_gpu_resources()`, render one frame, and capture pixels

## Critical Rules

1. **Model transform propagation**: All GPU data must include `model: mat4x4<f32>` in shader uniforms, applied in vertex shader (positions w=1, directions w=0), updated every frame via `structure.transform()`. Never assume GPU positions are in world space.

2. **UI layout**: Use `egui::Grid` with 2 columns for label+widget rows. Use `ui.add_sized` for standalone buttons.

3. **Clippy**: Must be zero warnings at all times.

4. **File size limit (2000 lines)**: No single source file should exceed 2,000 lines. When adding functionality causes a file to approach or exceed this limit, **stop and notify the user** before proceeding. Propose a split strategy (e.g., extract a submodule, move quantities into their own file, separate rendering logic from data logic) and get approval before restructuring. This keeps files navigable and avoids monolithic modules that are hard to review and maintain.

## Documentation

| Document | Contents |
|----------|----------|
| [Feature Status & Roadmap](docs/feature-status.md) | Feature comparison tables, completed/planned work, known issues |
| [Development Guide](docs/development-guide.md) | Adding structures/quantities, shader dev, UI conventions, API patterns, migration tips |
| [Architecture Differences](docs/architecture-differences.md) | C++ vs Rust rendering implementation differences |
| [Design Document](docs/plans/2026-01-21-polyscope-rs-design.md) | Original design plan |

## Reference

- **Local C++ Polyscope source**: `~/repo/polyscope` — always check for implementation details
- C++ Polyscope repo: https://github.com/nmwsharp/polyscope
- Polyscope docs: https://polyscope.run
