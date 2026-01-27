# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

polyscope-rs is a Rust-native 3D visualization library for geometric data. It's a port/reimagining of the C++ [Polyscope](https://polyscope.run) library, targeting the Rust ecosystem.

**Core paradigm**: Structures (geometric objects) + Quantities (data associated with structures)

## Build Commands

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Workspace Structure

```
polyscope-rs/
├── crates/
│   ├── polyscope-core/       # Core traits, registry, state management
│   ├── polyscope-render/     # wgpu rendering backend
│   ├── polyscope-ui/         # egui UI integration
│   ├── polyscope-structures/ # Structure implementations (mesh, points, etc.)
│   └── polyscope/            # Main crate, re-exports all sub-crates
├── examples/
└── tests/
```

## Architecture

### Core Traits (polyscope-core)

- `Structure` - Base trait for geometric objects (meshes, point clouds)
- `Quantity` - Base trait for data attached to structures (scalars, vectors, colors)
- `HasQuantities` - Trait for structures that can have quantities attached

### State Management

Global state is managed via `OnceLock<RwLock<Context>>`:
- `with_context(|ctx| ...)` - Read access
- `with_context_mut(|ctx| ...)` - Write access

### Rendering (polyscope-render)

- `RenderEngine` - wgpu-based renderer (windowed or headless)
- `Camera` - 3D camera with orbit/pan/zoom controls
- `ShaderBuilder` - WGSL shader compilation
- `MaterialRegistry` / `ColorMapRegistry` - Built-in materials and color maps

### Structures (polyscope-structures)

- `PointCloud` - Point set with scalar/vector/color quantities (full feature parity)
- `SurfaceMesh` - Triangle mesh with vertex/face scalar/color/vector quantities
- `CurveNetwork` - Edge network with node/edge scalar/color/vector quantities, tube rendering via compute shaders
- `VolumeMesh` - Tet/hex mesh with vertex/cell scalar/color/vector quantities, slice plane capping (full)
- `VolumeGrid` - Regular 3D grid with node scalar quantities (missing: cell quantities, isosurface)
- `CameraView` - Camera frustum visualization (full)

## Technology Stack

| Component | Library |
|-----------|---------|
| Rendering | wgpu |
| UI | egui (pure Rust, no native dependencies) |
| Math | glam |
| Windowing | winit |
| Serialization | serde + serde_json |

## Development Notes

### Adding a New Structure

1. Create module in `polyscope-structures/src/`
2. Implement `Structure` trait
3. Implement `HasQuantities` if it supports quantities
4. Add registration function in main `polyscope` crate
5. Add tests

### Adding a New Quantity

1. Create quantity struct implementing `Quantity` trait
2. Add appropriate marker trait (`VertexQuantity`, `FaceQuantity`, etc.)
3. Add convenience method on parent structure
4. Add UI controls in `polyscope-ui`

### Shader Development

Shaders are written in WGSL (WebGPU Shading Language). Implemented shaders:
- Point sphere impostor (instanced rendering, no geometry shaders)
- Mesh surface (flat/smooth shading)
- Vector arrows (instanced with precomputed mesh template)
- Ground plane with shadows and reflections
- Curve network (line mode + tube mode via compute shaders)
- GPU picking (point, mesh, curve, volume)
- Tone mapping
- Shadow map + blur
- SSAO (Screen-Space Ambient Occlusion)
- Slice plane visualization (grid pattern)
- Volume mesh slice capping

### Transparency (polyscope-render)

Weighted Blended Order-Independent Transparency (OIT) is implemented via `OitPass` and `oit_composite.wgsl`. Surface meshes support `set_transparency()`.

## Current Status

- **Version:** 0.2.0
- **Clippy:** Clean (zero warnings)
- **Tests:** Passing
- **Feature parity:** ~92% of C++ Polyscope 2.x

### Missing Features (vs C++ Polyscope)
- Full polygon mesh support (arbitrary polygons beyond triangles)
- Color RGBA (currently RGB only)

## Reference

- Original C++ Polyscope: https://github.com/nmwsharp/polyscope
- **Local C++ Polyscope source**: `~/repo/polyscope` - Always check this for implementation details
- Polyscope documentation: https://polyscope.run
- Design document: `docs/plans/2026-01-21-polyscope-rs-design.md`
