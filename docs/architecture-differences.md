# polyscope-rs vs C++ Polyscope: Architecture Differences

This document outlines key architectural differences between the original C++ Polyscope and this Rust implementation.

## Rendering Backend

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Graphics API** | OpenGL 3.3+ / OpenGL ES | wgpu (Vulkan/Metal/DX12/WebGPU) |
| **Shader Language** | GLSL | WGSL |
| **Geometry Shaders** | Yes (used extensively) | Not available in wgpu |
| **Compute Shaders** | Optional | Available |

### Uniform Buffer Binding Validation

All uniform buffer bindings in polyscope-rs set explicit `min_binding_size` (via `NonZeroU64`) rather than using `None` (deferred validation). This is a deliberate design choice to work around [wgpu Issue #7359](https://github.com/gfx-rs/wgpu/issues/7359), where late buffer binding size validation can cross-contaminate between pipelines sharing a command encoder. By specifying sizes at bind group layout creation time, validation errors surface immediately at pipeline/bind-group creation rather than intermittently at draw time.

**WGSL struct alignment caveat**: WGSL `vec3<T>` aligns to 16 bytes, not 12. Padding fields in WGSL uniform structs must use scalar types (e.g., `_pad0: u32, _pad1: u32, _pad2: u32`) rather than `vec3<u32>` to match Rust `#[repr(C)]` struct sizes.

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

## Curve Network Tube Rendering

### C++ Polyscope Approach
Uses a **geometry shader pipeline** for cylinder impostors:

1. **Vertex Shader**: Passes through cylinder endpoints (tail, tip)
2. **Geometry Shader**: Expands each edge into a bounding cube (8 corners → 14 vertices as triangle strip)
3. **Fragment Shader**: Ray-casts against cylinder to find intersection point, computes normal and depth for lighting

### polyscope-rs Approach
Since wgpu does not support geometry shaders, we use **compute shaders** for geometry generation:

1. **Compute Shader**: Generates bounding box geometry (36 vertices per edge = 12 triangles) from edge endpoints
2. **Vertex Shader**: Transforms generated box vertices, passes edge ID to fragment shader
3. **Fragment Shader**: Same ray-cylinder intersection logic as C++ version

**Trade-offs:**
- Compute shaders are the modern replacement for geometry shaders in WebGPU
- Explicit geometry buffer management (compute output → vertex input)
- Better separation of concerns (geometry generation vs. rendering)
- More flexible for future enhancements (variable radius, caps)

**Features:**
- Render mode toggle (Lines / Tubes) in UI
- Proper depth output for correct occlusion
- Per-edge colors supported
- Adjustable radius

## Vector Arrow Rendering

### C++ Polyscope Approach
Uses geometry shader to generate cylinder + cone geometry from line segments, then ray-casts in fragment shader.

### polyscope-rs Approach
Uses instanced rendering with procedurally generated arrow geometry in the vertex shader (`vector_arrow.wgsl`). Each arrow instance is 120 vertices (8-segment cross-section):

- **Shaft cylinder**: 48 vertices (8 quads = 16 triangles)
- **Cone sides**: 24 vertices (8 triangles)
- **Cone bottom cap**: 24 vertices (8 fan triangles)
- **Shaft bottom cap**: 24 vertices (8 fan triangles)

Arrow proportions: cone takes 30% of total length, cone radius = 2× shaft radius. An orthonormal basis is built per-instance from the vector direction, then local coordinates are transformed to world space.

**Shared infrastructure**: All vector-like quantities (vertex/face vectors, intrinsic vectors, one-forms) share the same shader and `VectorRenderData` GPU resources. Registration methods call `auto_scale()` to set length and radius proportional to the structure's bounding box diagonal.

## GPU Picking

### C++ Polyscope Approach
Uses OpenGL's pick buffer with unique color IDs per element, read back via `glReadPixels`.

### polyscope-rs Approach
Uses a similar GPU pick buffer approach with wgpu:
1. **Pick Buffer**: Offscreen RGBA8Unorm texture at viewport resolution
2. **Encoding**: 12-bit structure ID + 12-bit element ID packed into RGB (24 bits)
3. **Readback**: Single pixel copied to staging buffer via `copy_texture_to_buffer`
4. **Async Mapping**: `buffer.map_async()` for CPU access

**Features:**
- Pixel-perfect element-level picking (point #42, face #127)
- Proper depth testing for overlapping structures
- Pick pass only runs on click (not every frame)
- Structure ID assignment via registry

**Encoding scheme:**
```
R[7:0] = struct_id[11:4]
G[7:4] = struct_id[3:0]
G[3:0] = elem_id[11:8]
B[7:0] = elem_id[7:0]
```

Supports up to 4,096 structures and 4,096 elements per structure.

## Transparency Rendering

### C++ Polyscope Approach
Uses depth peeling or sorted rendering for order-independent transparency.

### polyscope-rs Approach
Uses **Weighted Blended Order-Independent Transparency (OIT)**:

1. **Accumulation Pass**: Transparent fragments write weighted color and alpha to accumulation/reveal textures
2. **Composite Pass**: Full-screen pass blends accumulated transparency over the opaque scene

**Trade-offs:**
- Single-pass approach (no multi-pass depth peeling)
- Approximate but fast and artifact-free for most cases
- Surface meshes support per-structure transparency via `set_transparency()`

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
| Surface Mesh | ✅ Full | ✅ Most | Triangles full (vertex/face scalar/color/vector/parameterization/intrinsic vector/one-form), polygons basic |
| Curve Network | ✅ Full | ✅ Full | Line, loop, segments; tube rendering via compute shaders; node/edge scalar/color/vector quantities |
| Volume Mesh | ✅ Full | ✅ Full | Tet/hex cells, quantities, interior face detection, slice capping |
| Volume Grid | ✅ Full | ✅ Basic | Node scalars only. Missing: cell quantities, isosurface rendering |
| Camera View | ✅ Full | ✅ Full | Frustum visualization |
| Floating Quantities | ✅ Full | ✅ Most | Scalar/color images, depth/color render images |

### Quantities

| Quantity Type | C++ Polyscope | polyscope-rs |
|---------------|--------------|--------------|
| Scalar | ✅ | ✅ |
| Vector | ✅ | ✅ |
| Color (RGB) | ✅ | ✅ |
| Color (RGBA) | ✅ | ⚠️ RGB only |
| Parameterization | ✅ | ✅ |
| Intrinsic Vector | ✅ | ✅ |
| One Form | ✅ | ✅ |

### Scene Features

| Feature | C++ Polyscope | polyscope-rs | Notes |
|---------|--------------|--------------|-------|
| Ground Plane | ✅ | ✅ | Tile/Shadow/Reflection modes |
| Ground Shadows | ✅ | ✅ | Shadow map with blur |
| Ground Reflections | ✅ | ✅ | Stencil-based |
| Tone Mapping | ✅ | ✅ | HDR pipeline |
| SSAO | ❌ | ✅ | polyscope-rs only feature |
| Transparency | ✅ | ✅ | Weighted Blended OIT |
| Slice Planes | ✅ | ✅ | Max 4, with volume mesh capping |
| Groups | ✅ | ✅ | Hierarchical |
| Gizmos | ✅ | ✅ | Via egui (transform-gizmo-egui), not GPU-rendered |
| Picking | ✅ | ✅ | GPU-based, element-level |
| Screenshots | ✅ | ✅ | PNG/JPEG, transparent background |

### Materials & Color Maps

All 8 built-in materials (Clay, Wax, Candy, Flat, Mud, Ceramic, Jade, Normal) and 10+ color maps (Viridis, Blues, Reds, Coolwarm, etc.) are implemented with equivalent behavior.

**Matcap rendering implementation:**

Both C++ Polyscope and polyscope-rs use **matcap (material capture)** textures for lighting. The view-space normal is mapped to UV coordinates to sample pre-baked lighting from matcap textures, eliminating the need for runtime light source calculations.

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Texture format** | Embedded via `bindata` | Embedded via `include_bytes!()` |
| **4-channel materials** | `color.r*R + color.g*G + color.b*B + (1-r-g-b)*K` | Same formula |
| **Single-texture materials** | Direct lookup modulated by color | Same approach |
| **Bind group slot** | N/A (OpenGL textures) | Group 2 in all scene pipelines |
| **Per-structure material** | `structure->setMaterial(...)` | `structure.set_material(...)` via `Structure` trait |

**Material types:**
- **4-channel blend** (clay, wax, candy, flat): Four HDR textures (R/G/B/K channels) blended by the object's base color
- **Single-texture** (mud, ceramic, jade, normal): One JPEG texture, modulated by base color luminance

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
| Dear ImGui (C++) | UI | egui |
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

1. **Full Polygon Mesh Support** - Arbitrary polygons (not just triangles)
2. **Color RGBA** - Currently only RGB; alpha channel not supported

All major quantity types are now implemented, including parameterization, intrinsic vectors, and one-forms with full GPU rendering.

---

## Additional Features (polyscope-rs only)

The following features are unique to polyscope-rs and not available in C++ Polyscope:

### SSAO (Screen-Space Ambient Occlusion)

Real-time ambient occlusion post-processing effect that adds soft shadows in corners and crevices for improved depth perception.

**Implementation:**
- Classic Crytek-style hemisphere sampling with 32 well-distributed samples
- Edge-aware improvements to handle sharp geometry (cube corners, hard edges):
  - Normal discontinuity detection to reduce AO at geometric edges
  - Per-sample normal weighting to prevent AO bleeding across different surfaces
- Depth-aware bilateral blur for smooth results while preserving edges
- Configurable parameters: radius, intensity, bias, sample count

**Files:**
- `crates/polyscope-render/src/shaders/ssao.wgsl` - Main SSAO shader
- `crates/polyscope-render/src/shaders/ssao_blur.wgsl` - Bilateral blur shader
- `crates/polyscope-render/src/ssao_pass.rs` - Render pass setup
- `crates/polyscope-core/src/ssao.rs` - Configuration struct

**Usage:**
```rust
// Enable SSAO
set_ssao_enabled(true);

// Adjust parameters (optional)
set_ssao_radius(0.5);      // Sampling radius
set_ssao_intensity(1.5);   // Effect strength
```

### Tube-Based Curve Network Picking

Enhanced picking for curve networks using ray-cylinder intersection, making thin curves easier to select.

**Implementation:**
- Dedicated pick shader (`pick_curve_tube.wgsl`) using ray-cylinder intersection
- Automatically uses tube radius or a minimum pick radius for better hit detection
- Falls back to line-based picking when tube rendering is disabled

**Files:**
- `crates/polyscope-render/src/shaders/pick_curve_tube.wgsl` - Tube pick shader
- `crates/polyscope-render/src/pick.rs` - `TubePickUniforms` struct
