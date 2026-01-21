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
