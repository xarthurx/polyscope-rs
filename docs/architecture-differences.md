# polyscope-rs vs C++ Polyscope: Architecture Differences

Key architectural differences between C++ Polyscope (OpenGL) and polyscope-rs (wgpu).

## Rendering Backend

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Graphics API** | OpenGL 3.3+ / OpenGL ES | wgpu (Vulkan/Metal/DX12/WebGPU) |
| **Shader Language** | GLSL | WGSL |
| **Geometry Shaders** | Yes (used extensively) | Not available in wgpu |
| **Compute Shaders** | Optional | Available |

### Uniform Buffer Binding Validation

All uniform buffer bindings set explicit `min_binding_size` (via `NonZeroU64`) rather than `None` (deferred validation). This works around [wgpu Issue #7359](https://github.com/gfx-rs/wgpu/issues/7359), where late validation can cross-contaminate between pipelines sharing a command encoder.

**WGSL struct alignment caveat**: WGSL `vec3<T>` aligns to 16 bytes, not 12. Padding fields in WGSL uniform structs must use scalar types (e.g., `_pad0: u32`) rather than `vec3<u32>` to match Rust `#[repr(C)]` struct sizes.

## Point Cloud Rendering

**C++ (geometry shader)**: Vertex shader transforms to view space -> geometry shader expands to billboard quad -> fragment shader ray-casts sphere.

**polyscope-rs (instanced rendering)**: Vertex shader uses instance ID to fetch point data from storage buffer and generates billboard vertices directly -> same ray-casting fragment shader. More portable (WebGPU compatible) and often faster on modern GPUs.

## Curve Network Tube Rendering

**C++ (geometry shader)**: Vertex shader passes through endpoints -> geometry shader expands to bounding cube (14 verts triangle strip) -> fragment shader ray-casts cylinder.

**polyscope-rs (compute shader)**: Compute shader generates bounding box geometry (36 verts/edge = 12 triangles) -> vertex shader transforms -> same ray-cylinder fragment shader. Better separation of concerns and more flexible for enhancements.

## Vector Arrow Rendering

**C++**: Geometry shader generates cylinder + cone from line segments, then ray-casts in fragment shader.

**polyscope-rs**: Instanced rendering with procedurally generated arrow geometry in the vertex shader. Each arrow = 120 vertices (8-segment cross-section): shaft cylinder (48), cone sides (24), cone cap (24), shaft cap (24). Arrow proportions: cone = 30% of total length, cone radius = 2x shaft radius.

All vector-like quantities (vertex/face vectors, intrinsic vectors, one-forms) share `vector_arrow.wgsl` and `VectorRenderData`.

## GPU Picking

**C++**: OpenGL pick buffer with color IDs, readback via `glReadPixels`.

**polyscope-rs**: Similar approach with wgpu. Offscreen RGBA8Unorm texture, 12-bit structure ID + 12-bit element ID packed into RGB (24 bits). Single pixel readback via `copy_texture_to_buffer` + `buffer.map_async()`. Pick pass only runs on click.

Encoding: `R[7:0]=struct[11:4], G[7:4]=struct[3:0], G[3:0]=elem[11:8], B[7:0]=elem[7:0]`. Supports 4,096 structures x 4,096 elements.

## Transparency Rendering

Both implement front-to-back **depth peeling** (Pretty mode) for order-independent transparency.

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Min-depth format** | `DEPTH24` (24-bit integer) | `Rgba16Float` (half precision) |
| **Min-depth update** | Native `DepthMode::Greater` | Max blend on color attachment |
| **Depth epsilon** | `1e-6` (exact with DEPTH24) | `2e-3` (compensates for f16 quantization) |
| **Peel loop contents** | ALL structures + ground plane | Surface meshes only |

**Why `Rgba16Float`**: WebGPU's `R32Float` is not blendable without the optional `float32-blendable` feature. `Rgba16Float` is always blendable, at the cost of reduced precision (~10-bit mantissa vs 24-bit).

## Groups & Visibility

**C++**: `createGroup()` + `addChildStructure()`, per-structure toggle.

**polyscope-rs**: Same API. Differences: `is_structure_visible()` combines structure enabled + group ancestry chain. Cascading toggle propagates to all descendants. Recursive member count display (e.g., "All Objects (9)").

## Shader Composition

**C++**: Text-based "rules" system with string replacement to compose features.

**polyscope-rs**: Conditional compilation within WGSL shaders via feature flags and composed bind group layouts. More type-safe but less dynamic.

## UI Framework

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Library** | Dear ImGui | egui (pure Rust) |
| **Backend** | OpenGL | wgpu (via egui-wgpu) |
| **Build deps** | None (bundled) | None (pure Rust) |

egui chosen over `dear-imgui-rs` to avoid `libclang` dependency and enable `cargo build` on any platform without setup. Same functionality, slightly different appearance.

## Matcap Materials

Both use matcap textures for lighting. View-space normal mapped to UV for pre-baked lighting lookup.

- **4-channel blend** (clay, wax, candy, flat): `color.r*R + color.g*G + color.b*B + (1-r-g-b)*K`
- **Single-texture** (mud, ceramic, jade, normal): Direct lookup modulated by base color

polyscope-rs uses Group 2 bind group in all scene pipelines. Textures embedded via `include_bytes!()`.

## Memory Management

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **GPU Buffers** | `ManagedBuffer<T>` RAII | wgpu `Buffer` with Rust ownership |
| **Structure Storage** | `std::map<string, unique_ptr>` | `HashMap<String, Box<dyn Structure>>` |
| **Global State** | Singleton | `OnceLock<RwLock<Context>>` |

## polyscope-rs Only Features

### SSAO (Screen-Space Ambient Occlusion)

Classic Crytek-style hemisphere sampling (32 samples) with edge-aware improvements (normal discontinuity detection, per-sample normal weighting) and depth-aware bilateral blur. Configurable radius, intensity, bias, sample count.

### Tube-Based Curve Network Picking

Dedicated pick shader using ray-cylinder intersection for better hit detection on thin curves. Falls back to line-based picking when tube rendering is disabled.
