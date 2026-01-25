# polyscope-rs vs C++ Polyscope: Comparison Guide

This document provides a comprehensive comparison between polyscope-rs (the Rust implementation) and the original C++ Polyscope library for developers migrating between or choosing between the two versions.

## Quick Summary

| Aspect | C++ Polyscope | polyscope-rs |
|--------|--------------|--------------|
| Language | C++17 | Rust 2021 |
| Graphics API | OpenGL 3.3+ | wgpu (Vulkan/Metal/DX12/WebGPU) |
| UI Framework | Dear ImGui (C++) | dear-imgui-rs |
| Math Library | GLM | glam |
| Windowing | GLFW | winit |
| Build System | CMake | Cargo |
| Web Support | Emscripten | WebGPU (native) |

## Structures

### Supported Structures

| Structure | C++ Polyscope | polyscope-rs | Notes |
|-----------|--------------|--------------|-------|
| Point Cloud | ✅ Full | ✅ Full | Complete feature parity |
| Surface Mesh | ✅ Full | ✅ Basic | Triangles supported, polygons basic |
| Curve Network | ✅ Full | ✅ Full | Line, loop, segments variants |
| Volume Mesh | ✅ Full | ✅ Basic | Tet/hex cells, no cuts |
| Volume Grid | ✅ Full | ✅ Basic | Node/cell scalars, basic isosurface |
| Camera View | ✅ Full | ✅ Full | Frustum visualization |
| Floating Quantities | ✅ Full | ❌ Not yet | Screen-space quantities |

### Structure Features

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Visibility toggle | ✅ | ✅ |
| Transform (translate/rotate/scale) | ✅ | ✅ |
| Material selection | ✅ | ✅ |
| Color customization | ✅ | ✅ |
| Transparency | ✅ | ⚠️ Partial |
| Smooth shading | ✅ | ✅ |
| Edge rendering | ✅ | ✅ |
| Backface culling | ✅ | ✅ |

## Quantities

### Point Cloud Quantities

| Quantity | C++ Polyscope | polyscope-rs |
|----------|--------------|--------------|
| Scalar | ✅ | ✅ |
| Vector | ✅ | ✅ |
| Color | ✅ | ✅ |
| Color (RGBA) | ✅ | ⚠️ RGB only |
| Parameterization | ✅ | ❌ |

### Surface Mesh Quantities

| Quantity | C++ Polyscope | polyscope-rs |
|----------|--------------|--------------|
| Vertex Scalar | ✅ | ✅ |
| Face Scalar | ✅ | ✅ |
| Vertex Color | ✅ | ✅ |
| Face Color | ✅ | ✅ |
| Vertex Vector | ✅ | ⚠️ Basic |
| Face Vector | ✅ | ⚠️ Basic |
| Corner Parameterization | ✅ | ❌ |
| Intrinsic Vector | ✅ | ❌ |
| One Form | ✅ | ❌ |

### Curve Network Quantities

| Quantity | C++ Polyscope | polyscope-rs |
|----------|--------------|--------------|
| Node Scalar | ✅ | ✅ |
| Edge Scalar | ✅ | ✅ |
| Node Color | ✅ | ✅ |
| Edge Color | ✅ | ✅ |
| Node Vector | ✅ | ⚠️ Basic |
| Edge Vector | ✅ | ⚠️ Basic |

### Volume Mesh Quantities

| Quantity | C++ Polyscope | polyscope-rs |
|----------|--------------|--------------|
| Vertex Scalar | ✅ | ✅ |
| Cell Scalar | ✅ | ✅ |
| Vertex Color | ✅ | ✅ |
| Cell Color | ✅ | ✅ |
| Vertex Vector | ✅ | ⚠️ Basic |

## Rendering Features

### Scene Features

| Feature | C++ Polyscope | polyscope-rs | Notes |
|---------|--------------|--------------|-------|
| Ground Plane | ✅ | ✅ | |
| Ground Shadows | ✅ | ✅ | Shadow mapping |
| Ground Reflections | ✅ | ✅ | Planar reflections |
| Tone Mapping | ✅ | ✅ | HDR pipeline |
| SSAO | ✅ | ❌ | Not yet implemented |
| Anti-aliasing | ✅ MSAA | ⚠️ Basic | |
| Transparency | ✅ | ⚠️ Partial | |

### Materials

| Material | C++ Polyscope | polyscope-rs |
|----------|--------------|--------------|
| Clay | ✅ | ✅ |
| Wax | ✅ | ✅ |
| Candy | ✅ | ✅ |
| Flat | ✅ | ✅ |
| Mud | ✅ | ✅ |
| Ceramic | ✅ | ✅ |
| Jade | ✅ | ✅ |
| Normal | ✅ | ✅ |
| Custom Materials | ✅ | ⚠️ Limited |

### Color Maps

| Color Map | C++ Polyscope | polyscope-rs |
|-----------|--------------|--------------|
| Viridis | ✅ | ✅ |
| Blues | ✅ | ✅ |
| Reds | ✅ | ✅ |
| Coolwarm | ✅ | ✅ |
| Pink-Green | ✅ | ✅ |
| Phase | ✅ | ✅ |
| Spectral | ✅ | ✅ |
| Rainbow | ✅ | ✅ |
| Jet | ✅ | ✅ |
| Turbo | ✅ | ✅ |

## Camera & Navigation

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Turntable Navigation | ✅ | ✅ |
| Free Navigation | ✅ | ✅ |
| Planar Navigation | ✅ | ✅ |
| First Person | ✅ | ✅ |
| Perspective Projection | ✅ | ✅ |
| Orthographic Projection | ✅ | ✅ |
| Up Direction Control | ✅ | ✅ |
| Front Direction Control | ✅ | ✅ |
| FOV Control | ✅ | ✅ |
| Near/Far Planes | ✅ | ✅ |
| Reset Camera View | ✅ | ✅ |
| Home to Geometry | ✅ | ✅ |
| Screenshot | ✅ | ✅ |
| Headless Rendering | ✅ | ✅ |

## UI Features

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Main Control Panel | ✅ | ✅ |
| Structure Panel | ✅ | ✅ |
| Appearance Settings | ✅ | ✅ |
| Camera Settings | ✅ | ✅ |
| Ground Plane Controls | ✅ | ✅ |
| Slice Plane Controls | ✅ | ✅ |
| Groups Panel | ✅ | ✅ |
| Gizmo Controls | ✅ | ✅ |
| Messages/Errors | ✅ | ⚠️ Basic |
| User Callbacks | ✅ | ⚠️ Basic |
| Custom UI Widgets | ✅ | ⚠️ Limited |

## Slice Planes

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Multiple Planes | ✅ | ✅ (max 4) |
| Plane Visualization | ✅ | ✅ |
| Widget Visualization | ✅ | ✅ |
| Interactive Dragging | ✅ | ⚠️ Basic |
| Volume Cutting | ✅ | ✅ |
| Capping | ✅ | ❌ |

## Groups

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Create/Delete Groups | ✅ | ✅ |
| Hierarchical Groups | ✅ | ✅ |
| Group Enable/Disable | ✅ | ✅ |
| Add/Remove Structures | ✅ | ✅ |
| Collapse Details | ✅ | ✅ |

## Picking & Selection

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Structure Picking | ✅ | ✅ |
| Element Picking | ✅ | ✅ |
| Selection Highlight | ✅ | ⚠️ Basic |
| Hover Highlight | ✅ | ⚠️ Basic |

## Gizmos

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Translate Gizmo | ✅ | ✅ |
| Rotate Gizmo | ✅ | ✅ |
| Scale Gizmo | ✅ | ✅ |
| World/Local Space | ✅ | ✅ |
| Snap to Grid | ✅ | ✅ |

## API Differences

### Initialization

**C++ Polyscope:**
```cpp
#include "polyscope/polyscope.h"

int main() {
    polyscope::init();
    // ... register structures ...
    polyscope::show();
    return 0;
}
```

**polyscope-rs:**
```rust
use polyscope::*;

fn main() -> Result<()> {
    init()?;
    // ... register structures ...
    show();
    Ok(())
}
```

### Registering Structures

**C++ Polyscope:**
```cpp
std::vector<glm::vec3> points = { ... };
polyscope::PointCloud* pc = polyscope::registerPointCloud("my points", points);
pc->addScalarQuantity("height", heights);
```

**polyscope-rs:**
```rust
let points = vec![Vec3::new(0.0, 0.0, 0.0), ...];
let pc = register_point_cloud("my points", points);
pc.add_scalar_quantity("height", heights);
```

### Surface Mesh Registration

**C++ Polyscope:**
```cpp
std::vector<glm::vec3> vertices = { ... };
std::vector<std::vector<size_t>> faces = { ... };  // Polygon soup
polyscope::SurfaceMesh* mesh = polyscope::registerSurfaceMesh("my mesh", vertices, faces);
```

**polyscope-rs:**
```rust
let vertices = vec![Vec3::new(0.0, 0.0, 0.0), ...];
let faces = vec![UVec3::new(0, 1, 2), ...];  // Triangle only variant
let mesh = register_surface_mesh("my mesh", vertices, faces);
```

### Accessing Structures

**C++ Polyscope:**
```cpp
polyscope::PointCloud* pc = polyscope::getPointCloud("my points");
if (pc != nullptr) {
    pc->setPointRadius(0.01);
}
```

**polyscope-rs:**
```rust
// Option 1: Get handle
if let Some(pc) = get_point_cloud("my points") {
    // Use handle methods
}

// Option 2: Closure-based access
with_point_cloud("my points", |pc| {
    pc.set_point_radius(0.01);
});
```

## Framework Replacements

| C++ Library | Purpose | Rust Replacement |
|-------------|---------|------------------|
| OpenGL | Graphics API | wgpu |
| GLFW | Windowing | winit |
| GLM | Math | glam |
| Dear ImGui (C++) | UI | dear-imgui-rs |
| stb_image | Image loading | image |
| nlohmann/json | JSON | serde_json |
| happly | PLY loading | (custom) |
| CMake | Build | Cargo |

### Graphics Backend Comparison

| OpenGL Feature | wgpu Equivalent |
|----------------|-----------------|
| Vertex Buffer Objects | `wgpu::Buffer` |
| Shaders (GLSL) | Shaders (WGSL) |
| Framebuffer Objects | `wgpu::Texture` + `wgpu::TextureView` |
| Uniforms | Uniform Buffers + Bind Groups |
| Textures | `wgpu::Texture` |
| Render States | Pipeline State |
| glDrawArrays/Elements | `RenderPass::draw`/`draw_indexed` |

## Missing Features (Planned)

The following C++ Polyscope features are not yet implemented but planned:

1. **Floating Quantities** - Screen-space data visualization
2. **SSAO (Screen Space Ambient Occlusion)** - Enhanced depth perception
3. **Parameterization Quantities** - UV coordinates visualization
4. **Intrinsic Vectors** - Tangent-space vector visualization
5. **One-Form Quantities** - Differential form visualization
6. **Slice Plane Capping** - Fill exposed faces when slicing
7. **Implicit Surface Rendering** - Ray marching for volume grids
8. **Custom User Widgets** - Full ImGui widget API access

## Performance Notes

| Aspect | C++ Polyscope | polyscope-rs |
|--------|--------------|--------------|
| Startup Time | Fast | Slightly slower (wgpu initialization) |
| Memory Usage | Lower | Slightly higher |
| Rendering Speed | Very fast (OpenGL) | Fast (wgpu overhead) |
| Cross-platform | Excellent | Excellent |
| WebGPU Support | Via Emscripten | Native |

## Migration Tips

### From C++ to Rust

1. **Pointers to Handles**: C++ uses raw pointers; Rust uses handle structs with methods
2. **Error Handling**: C++ uses exceptions; Rust uses `Result<T, E>`
3. **Vector Types**: GLM types → glam types (`glm::vec3` → `Vec3`)
4. **Memory Management**: Manual in C++; automatic in Rust
5. **Closures**: Use `with_*` functions for mutable access to structures

### Common Gotchas

1. **Face Format**: polyscope-rs uses `UVec3` for triangles; C++ uses nested vectors
2. **Initialization**: polyscope-rs returns `Result`, so use `?` or `unwrap()`
3. **Thread Safety**: polyscope-rs uses `RwLock` for global state
4. **Shader Language**: GLSL → WGSL syntax differences

## Version Compatibility

| polyscope-rs Version | C++ Polyscope Feature Parity |
|---------------------|------------------------------|
| 0.1.x | ~70% of C++ 2.x features |

## Contributing

Both projects welcome contributions. Key areas where polyscope-rs needs work:

1. Completing quantity types (parameterization, intrinsic vectors)
2. Adding SSAO support
3. Improving transparency handling
4. Adding slice plane capping
5. Full polygon mesh support
6. Documentation and examples

## References

- C++ Polyscope: https://polyscope.run
- C++ Polyscope GitHub: https://github.com/nmwsharp/polyscope
- polyscope-rs GitHub: (this repository)
- wgpu: https://wgpu.rs
- dear-imgui-rs: https://github.com/Latias94/dear-imgui-rs
- glam: https://github.com/bitshifter/glam-rs
