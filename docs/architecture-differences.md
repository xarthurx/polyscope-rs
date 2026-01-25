# polyscope-rs vs C++ Polyscope: Architecture Differences

This document outlines key architectural differences between the original C++ Polyscope and this Rust implementation.

## Rendering Backend

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Graphics API** | OpenGL 3.3+ / OpenGL ES | wgpu (Vulkan/Metal/DX12/WebGPU) |
| **Shader Language** | GLSL | WGSL |
| **Geometry Shaders** | Yes (used extensively) | Not available in wgpu |
| **Compute Shaders** | Optional | Available |

## Point Cloud Rendering

### C++ Polyscope Approach
The original uses a **geometry shader pipeline** for sphere impostors:

1. **Vertex Shader**: Transforms point positions to view space
2. **Geometry Shader**: Expands each point into a 4-vertex billboard quad facing the camera
3. **Fragment Shader**: Ray-casts against sphere to compute exact intersection, normal, and depth

This approach is elegant because the geometry shader handles all billboard generation automatically.

### polyscope-rs Approach
Since wgpu does not support geometry shaders, we use **instanced rendering**:

1. **Vertex Shader**: Uses instance ID to fetch point data from storage buffer, generates billboard vertices directly
2. **Fragment Shader**: Same ray-casting logic as C++ version

**Trade-offs:**
- Requires explicit vertex generation logic in vertex shader
- Storage buffers used for point data (more modern GPU pattern)
- Potentially better performance on modern GPUs (geometry shaders are often slow)
- More portable (WebGPU compatible)

## Vector Arrow Rendering

### C++ Polyscope Approach
Uses geometry shader to generate cylinder + cone geometry from line segments, then ray-casts in fragment shader.

### polyscope-rs Approach
Uses instanced rendering with a precomputed arrow mesh template, transformed per-instance by position and vector direction.

## Shader Composition

### C++ Polyscope Approach
Uses a text-based "rules" system that performs string replacement to compose shader features:
```cpp
pointProgram = engine->requestShader("RAYCAST_SPHERE",
    {"SPHERE_PROPAGATE_VALUE", "SHADE_COLORMAP_VALUE"});
```

### polyscope-rs Approach
Uses Rust's type system and conditional compilation within WGSL shaders:
- Feature flags passed as shader constants
- Bind group layouts composed based on enabled features
- More type-safe but less dynamic

## Color Maps

Both implementations use 1D texture lookups for color mapping. The colormap data and algorithms are equivalent.

## UI Framework

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Library** | Dear ImGui (C++) | egui (pure Rust) |
| **Backend** | OpenGL | wgpu (via egui-wgpu) |
| **Build Dependencies** | None (bundled) | None (pure Rust) |

### Why egui Instead of dear-imgui-rs?

The original C++ Polyscope uses Dear ImGui, and there is a Rust binding (`dear-imgui-rs`). However, we chose **egui** for polyscope-rs:

**dear-imgui-rs drawbacks:**
- Requires `libclang` system dependency for building (bindgen generates FFI bindings)
- Users must install platform-specific packages before `cargo build` works:
  - Linux: `apt install libclang-dev`
  - macOS: `brew install llvm` or Xcode
  - Windows: LLVM from llvm.org
- Adds friction for contributors and end users

**egui advantages:**
- Pure Rust - no native dependencies, no FFI
- `cargo build` just works on any platform
- WebAssembly ready out of the box (future web deployment)
- Modern Rust API with native types
- Actively maintained with good wgpu integration
- Built-in dark theme that matches Polyscope's aesthetic

**Trade-offs:**
- UI will not look pixel-identical to C++ Polyscope
- Different API patterns (though both are immediate-mode)
- Some ImGui-specific widgets may need custom implementation

**Functional parity:** The UI provides the same *functionality* as C++ Polyscope (structure tree, quantity controls, picking panel) even though the exact appearance differs slightly. Users familiar with C++ Polyscope will find the same controls in the same logical locations.

## Memory Management

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **GPU Buffers** | `ManagedBuffer<T>` RAII wrapper | wgpu `Buffer` with Rust ownership |
| **Structure Storage** | `std::map<string, unique_ptr>` | `HashMap<String, Box<dyn Structure>>` |
| **Global State** | Singleton pattern | `OnceLock<RwLock<Context>>` |

## Platform Support

| Platform | C++ Polyscope | polyscope-rs |
|----------|---------------|--------------|
| Windows | OpenGL | Vulkan/DX12 |
| macOS | OpenGL (deprecated) | Metal |
| Linux | OpenGL | Vulkan |
| Web | No | WebGPU (future) |

The wgpu backend provides better future-proofing, especially for macOS (where OpenGL is deprecated) and web deployment.

---

## Feature Comparison

### Structures

| Structure | C++ Polyscope | polyscope-rs | Notes |
|-----------|--------------|--------------|-------|
| Point Cloud | ✅ Full | ✅ Full | Complete feature parity |
| Surface Mesh | ✅ Full | ✅ Basic | Triangles supported, polygons basic |
| Curve Network | ✅ Full | ✅ Full | Line, loop, segments variants |
| Volume Mesh | ✅ Full | ✅ Basic | Tet/hex cells, no cuts |
| Volume Grid | ✅ Full | ✅ Basic | Node/cell scalars, basic isosurface |
| Camera View | ✅ Full | ✅ Full | Frustum visualization |
| Floating Quantities | ✅ Full | ❌ Not yet | Screen-space quantities |

### Quantities

| Quantity Type | C++ Polyscope | polyscope-rs |
|---------------|--------------|--------------|
| Scalar | ✅ | ✅ |
| Vector | ✅ | ✅ |
| Color (RGB) | ✅ | ✅ |
| Color (RGBA) | ✅ | ⚠️ RGB only |
| Parameterization | ✅ | ❌ |
| Intrinsic Vector | ✅ | ❌ |
| One Form | ✅ | ❌ |

### Scene Features

| Feature | C++ Polyscope | polyscope-rs |
|---------|--------------|--------------|
| Ground Plane | ✅ | ✅ |
| Ground Shadows | ✅ | ✅ |
| Ground Reflections | ✅ | ✅ |
| Tone Mapping | ✅ | ✅ |
| SSAO | ✅ | ❌ |
| Slice Planes | ✅ | ✅ (max 4) |
| Groups | ✅ | ✅ |
| Gizmos | ✅ | ✅ |
| Picking | ✅ | ✅ |
| Screenshots | ✅ | ✅ |

### Materials & Color Maps

All 8 built-in materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal) and 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.) are implemented with equivalent behavior.

---

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

---

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

---

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

---

## Missing Features (Planned)

The following C++ Polyscope features are not yet implemented but planned:

1. **Floating Quantities** - Screen-space data visualization
2. **SSAO** - Screen Space Ambient Occlusion
3. **Parameterization Quantities** - UV coordinates visualization
4. **Intrinsic Vectors** - Tangent-space vector visualization
5. **One-Form Quantities** - Differential form visualization
6. **Slice Plane Capping** - Fill exposed faces when slicing
7. **Full Polygon Mesh Support** - Arbitrary polygons (not just triangles)
