# polyscope-rs

A Rust-native 3D visualization library for geometric data, inspired by [Polyscope](https://polyscope.run).

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

polyscope-rs is a viewer and user interface for 3D data such as meshes and point clouds. It allows you to register your data and quickly generate informative visualizations, either programmatically or via a dynamic GUI.

## Features

- **Point Clouds** - Visualize point sets with scalar, vector, and color quantities
- **Surface Meshes** - Render triangular and polygonal meshes with per-vertex/face data
- **Curve Networks** - Display networks of curves and edges
- **Volume Meshes** - Visualize tetrahedral and hexahedral meshes
- **Volume Grids** - Render implicit surfaces via marching cubes
- **Camera Views** - Visualize camera frustums and poses

## Quick Start

```rust
use polyscope::*;

fn main() -> Result<()> {
    // Initialize polyscope
    init()?;

    // Register a point cloud
    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let pc = register_point_cloud("my points", points);

    // Add a scalar quantity
    pc.add_scalar_quantity("height", vec![0.0, 0.5, 1.0]);

    // Show the viewer
    show();

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
polyscope = "0.1"
```

## Architecture

polyscope-rs uses a paradigm of **structures** and **quantities**:

- A **structure** is a geometric object in the scene (point cloud, mesh, etc.)
- A **quantity** is data associated with a structure (scalar field, vector field, colors)

## Crate Structure

- `polyscope` - Main crate with public API
- `polyscope-core` - Core traits and state management
- `polyscope-render` - wgpu rendering backend
- `polyscope-ui` - dear-imgui UI integration
- `polyscope-structures` - Structure implementations

## Technology Stack

- **Rendering**: [wgpu](https://wgpu.rs) - Cross-platform graphics (Vulkan/Metal/DX12/WebGPU)
- **UI**: [dear-imgui-rs](https://github.com/Latias94/dear-imgui-rs) - Immediate mode GUI
- **Math**: [glam](https://github.com/bitshifter/glam-rs) - Fast linear algebra
- **Windowing**: [winit](https://github.com/rust-windowing/winit) - Cross-platform windows

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

This project is inspired by the original [Polyscope](https://github.com/nmwsharp/polyscope) C++ library by Nicholas Sharp.
