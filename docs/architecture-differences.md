# polyscope-rs vs C++ Polyscope: Architecture Differences

Key architectural differences between C++ Polyscope (OpenGL) and polyscope-rs (wgpu).

## Rendering Backend

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Graphics API** | OpenGL 3.3+ / OpenGL ES | wgpu: Vulkan, Metal, DirectX 12, WebGPU |
| **Shader Language** | GLSL | WGSL |
| **Geometry Shaders** | Yes (used extensively) | Not available in wgpu |
| **Compute Shaders** | Optional | Available |

The engine requests only the Vulkan, Metal, DirectX 12 and WebGPU backends of wgpu. It does not use the OpenGL backend of wgpu, because some shaders sample depth textures in a way that this backend cannot compile. Without a supported backend, engine creation fails with "no graphics adapter found (Vulkan, Metal or DirectX 12 is necessary)".

### Uniform Buffer Binding Validation

All uniform buffer bindings set explicit `min_binding_size` (via `NonZeroU64`) rather than `None` (deferred validation). This works around [wgpu Issue #7359](https://github.com/gfx-rs/wgpu/issues/7359), where late validation can cross-contaminate between pipelines sharing a command encoder.

**WGSL struct alignment caveat**: WGSL `vec3<T>` aligns to 16 bytes, not 12. Padding fields in WGSL uniform structs must use scalar types (e.g., `_pad0: u32`) rather than `vec3<u32>` to match Rust `#[repr(C)]` struct sizes.

## Point Cloud Rendering

**C++ (geometry shader)**: Vertex shader transforms to view space -> geometry shader expands to billboard quad -> fragment shader ray-casts sphere.

**polyscope-rs (instanced rendering)**: Vertex shader uses instance ID to fetch point data from storage buffer and generates billboard vertices directly -> same ray-casting fragment shader. This approach also works on WebGPU, which has no geometry shaders.

## Curve Network Tube Rendering

**C++ (geometry shader)**: Vertex shader passes through endpoints -> geometry shader expands to bounding cube (14 verts triangle strip) -> fragment shader ray-casts cylinder.

**polyscope-rs (compute shader)**: Compute shader generates bounding box geometry (36 verts/edge = 12 triangles) -> vertex shader transforms -> same ray-cylinder fragment shader. The compute pass runs once per frame, before the shadow pass.

## Vector Arrow Rendering

**C++**: Geometry shader generates cylinder + cone from line segments, then ray-casts in fragment shader.

**polyscope-rs**: Instanced rendering with procedurally generated arrow geometry in the vertex shader. Each arrow = 120 vertices (8-segment cross-section): shaft cylinder (48), cone sides (24), cone cap (24), shaft cap (24). Arrow proportions: cone = 30% of total length, cone radius = 2x shaft radius.

All vector-like quantities (vertex/face vectors, intrinsic vectors, one-forms, curve network and volume mesh vectors) share `vector_arrow.wgsl` and `VectorRenderData`.

## Scalar Colormaps

**C++**: The shaders sample the colormap texture per pixel.

**polyscope-rs**: Point clouds, surface meshes, curve networks and volume meshes compute one color per element on the CPU (`compute_colors`). The GPU interpolates vertex colors across faces. Volume grid gridcubes sample a 1D colormap texture per cube on the GPU.

The colormap tables are the C++ tables. `scripts/extract_colormaps.py` extracts them to binary files, which `ColorMapRegistry` embeds.

## GPU Picking

**C++**: OpenGL pick buffer with color IDs, readback via `glReadPixels`.

**polyscope-rs**: Similar approach with wgpu. Offscreen `Rgba8Unorm` color texture and a `Depth32Float` depth texture. The pick pass renders on each frame of the window. A click reads one pixel with `copy_texture_to_buffer` + `buffer.map_async()`.

Encoding: one 24-bit global element index in RGB: `R = bits 16-23`, `G = bits 8-15`, `B = bits 0-7`. Index 0 is the background. Each structure owns the range `[global_start, global_start + element count)`. A volume grid quantity owns a range of its own. The pick buffer holds about 16.7 million elements in total over all structures. If the scene has more elements, the pick shaders draw the elements past the limit as the background, and a warning is logged once.

C++ Polyscope gives a structure a new range each time it prepares its pick program. It takes the range from 64-bit indices and does not use an index again. polyscope-rs has only 24 bits. Thus each window frame gives all ranges again, one after the other, from the current element counts. A count change, a removed structure or an empty structure leaves no unused indices. The pick pass draws a structure only with the range of its frame, so a structure that joins the registry during a frame waits for the next frame.

The pick depth gives the world position of the hit. A double-click uses it to set the view center on any structure type.

A click reports one element type per structure: a point, a face of a surface mesh, an edge of a curve network, a cell of a volume mesh. C++ can also pick vertices, edges, halfedges and corners of a surface mesh, and the nodes of a curve network.

## Transparency Rendering

Both implement front-to-back **depth peeling** (Pretty mode) for order-independent transparency.

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Min-depth format** | `DEPTH24` (24-bit integer) | `Rgba16Float` (half precision) |
| **Min-depth update** | Native `DepthMode::Greater` | Max blend on color attachment |
| **Depth epsilon** | `1e-6` (exact with DEPTH24) | `2e-3` (compensates for f16 quantization) |
| **Peel loop contents** | ALL structures + ground plane | Surface meshes only |
| **Default mode** | `None` (transparency off) | `Simple` |
| **`None` mode** | No transparency | Renders like `Simple` |

**Why `Rgba16Float`**: WebGPU's `R32Float` is not blendable without the optional `float32-blendable` feature. `Rgba16Float` is always blendable, at the cost of reduced precision (~10-bit mantissa vs 24-bit).

## Ground Plane

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Orientation** | Follows the up direction | Follows the up direction |
| **Shadows** | ShadowOnly mode, from all structures, with blur | Tile, ShadowOnly and TileReflection modes, from surface meshes (shadow map) |
| **Reflections** | TileReflection mode | TileReflection mode: surface meshes, volume meshes, point clouds, curve networks |
| **Default mode** | TileReflection | Tile |

## Camera and View State

**C++**: The `polyscope::view` namespace holds the camera state and has global functions for it (`lookAt`, `setViewCenter`, `setUpDir`, and more).

**polyscope-rs**: The camera lives in the `RenderEngine` of the private `App`. User code reaches it only through a `ViewState`:

- After each frame, the app publishes a snapshot in the global `Context`. `current_view_state()` reads it. Before the first frame, it returns `NoActiveView`.
- `apply_view_state()` and the `load_view_*` functions queue a view. The app applies it at the start of the next frame, also in a headless render.

The view JSON format is Rust-native and versioned. It also stores the render look of `Options`. C++ view JSON does not load.

The front direction has a different meaning. In polyscope-rs, the home view looks along the front direction. In C++, the camera sits on the front side of the scene. The home view distance comes from the field of view and the bounding sphere; C++ uses 1.5 times the length scale.

## Groups & Visibility

**C++**: `createGroup()` + `addChildStructure()`. `Group::setEnabled` sets the enabled flag of each child group and structure.

**polyscope-rs**: `create_group()` + `GroupHandle::add_point_cloud()` (and the other `add_*` methods). `GroupHandle::set_enabled` sets the flag of the group only. `is_structure_visible()` combines the structure flag with the flags of all ancestor groups. In the Groups panel, a toggle also sets the flags of the descendant groups. The panel shows recursive member counts (e.g., "All Objects (9)").

## Shader Composition

**C++**: Text-based "rules" system with string replacement to compose features.

**polyscope-rs**: Conditional compilation within WGSL shaders via feature flags and composed bind group layouts. More type-safe but less dynamic.

## UI Framework

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **Library** | Dear ImGui | egui (pure Rust) |
| **Backend** | OpenGL | wgpu (via egui-wgpu) |
| **Gizmo** | ImGuizmo | transform-gizmo-egui |
| **Build deps** | None (bundled) | None (pure Rust) |

egui avoids the `libclang` dependency of `dear-imgui-rs`, so `cargo build` works on any platform without setup. The panels follow the C++ layout, but the appearance differs, and some C++ panels and options are missing (see [Feature Status](feature-status.md)).

## Matcap Materials

Both use matcap textures for lighting. View-space normal mapped to UV for pre-baked lighting lookup.

- **4-channel blend** (clay, wax, candy, flat): `color.r*R + color.g*G + color.b*B + (1-r-g-b)*K`
- **Single-texture** (mud, ceramic, jade, normal): Direct lookup modulated by base color

polyscope-rs uses Group 2 bind group in all scene pipelines. Textures embedded via `include_bytes!()`.

## Memory Management

| Aspect | C++ Polyscope | polyscope-rs |
|--------|---------------|--------------|
| **GPU Buffers** | `ManagedBuffer<T>` RAII | wgpu `Buffer` with Rust ownership |
| **Structure Storage** | `std::map<string, unique_ptr>` | `HashMap<String, HashMap<String, Box<dyn Structure>>>` (type name, then structure name) |
| **Global State** | Singleton | `OnceLock<RwLock<Context>>` |

## polyscope-rs Only Features

### SSAO (Screen-Space Ambient Occlusion)

Classic Crytek-style hemisphere sampling with edge-aware improvements (normal discontinuity detection, per-sample normal weighting) and depth-aware bilateral blur. The kernel holds 32 samples; the default `sample_count` is 16, and the shader uses at most 32. `SsaoConfig` sets the radius, intensity, bias and sample count. The noise texture uses a fixed seed, so the result is the same on every run. SSAO has no effect when the SSAA factor is above 1.

### Tube-Based Curve Network Picking

Dedicated pick shader using ray-cylinder intersection for better hit detection on thin curves. The pick pass uses it for all curve networks, also in line mode. It falls back to line-based picking when the tube pick pipeline does not exist.

### RGBA Color Quantities

Surface mesh vertex and face color quantities accept `Vec4` colors with per-element alpha (`add_vertex_color_quantity_with_alpha`, `add_face_color_quantity_with_alpha`). C++ color quantities are RGB.
