# polyscope-rs Design Document

**Date**: 2026-01-21
**Status**: Approved
**Purpose**: Rust-native 3D visualization library for geometric data, inspired by C++ Polyscope

---

## 1. Overview & Motivation

**polyscope-rs** is a Rust-native 3D visualization library for geometric data, serving the Rust community with a modern, safe, and performant viewer for meshes, point clouds, curves, and volumetric data.

### Primary Goals

1. **AI-driven development experiment** - Test feasibility of AI-assisted Rust development for mid-scale software
2. **Performance** - Graphics viewer where performance matters
3. **Safety** - Leverage Rust's memory safety guarantees
4. **Modernization** - Clean codebase with modern tooling
5. **Cross-platform** - Support desktop and potentially web (via wgpu/WebGPU)

### Non-Goals

- Python bindings (Rust-only for now)
- Direct C++ interop

---

## 2. Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Rendering** | wgpu | Cross-platform (Vulkan/Metal/DX12/WebGPU), pure Rust, future-proof |
| **UI** | dear-imgui-rs | Mature ImGui ecosystem, includes ImPlot/ImGuizmo, wgpu backend |
| **Math** | glam | SIMD-optimized, fast, idiomatic Rust, standard in graphics ecosystem |
| **Windowing** | winit | Pure Rust, cross-platform, pairs with wgpu and dear-imgui-winit |
| **Serialization** | serde + serde_json | Preferences, state persistence |
| **Image I/O** | image | Screenshots, texture loading |
| **Mesh I/O** | ply-rs | PLY format support |
| **Testing** | built-in + proptest | Unit tests + property-based testing |

---

## 3. Workspace Structure

```
polyscope-rs/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── polyscope-core/           # Core abstractions
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── structure.rs      # Structure trait
│   │   │   ├── quantity.rs       # Quantity trait
│   │   │   ├── registry.rs       # Structure/group management
│   │   │   ├── state.rs          # Global context
│   │   │   ├── options.rs        # Configuration
│   │   │   └── pick.rs           # Selection/picking system
│   │   └── Cargo.toml
│   │
│   ├── polyscope-render/         # Rendering backend
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── engine.rs         # wgpu engine abstraction
│   │   │   ├── shader.rs         # Shader management (WGSL)
│   │   │   ├── buffer.rs         # GPU buffer management
│   │   │   ├── materials.rs      # Material definitions
│   │   │   ├── color_maps.rs     # Color map definitions
│   │   │   └── ground_plane.rs   # Ground plane rendering
│   │   └── Cargo.toml
│   │
│   ├── polyscope-ui/             # UI layer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── imgui_integration.rs
│   │   │   ├── structure_ui.rs   # Structure tree UI
│   │   │   ├── quantity_ui.rs    # Quantity controls
│   │   │   ├── color_bar.rs      # Color legend widget
│   │   │   └── gizmo.rs          # Transformation gizmo wrapper
│   │   └── Cargo.toml
│   │
│   ├── polyscope-structures/     # All structure implementations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── point_cloud/
│   │   │   ├── surface_mesh/
│   │   │   ├── curve_network/
│   │   │   ├── volume_mesh/
│   │   │   ├── volume_grid/
│   │   │   └── camera_view/
│   │   └── Cargo.toml
│   │
│   └── polyscope/                # Top-level re-export crate
│       ├── src/lib.rs            # pub use all sub-crates
│       └── Cargo.toml
│
├── examples/
│   └── demo_app.rs               # Port of C++ demo
│
└── tests/                        # Integration tests
    ├── basics_test.rs
    ├── surface_mesh_test.rs
    ├── point_cloud_test.rs
    └── ...
```

---

## 4. Core Abstractions

### Structure Trait

```rust
pub trait Structure: Send + Sync {
    fn name(&self) -> &str;
    fn type_name(&self) -> &'static str;
    fn bounding_box(&self) -> Option<(Vec3, Vec3)>;
    fn length_scale(&self) -> f32;
    fn transform(&self) -> Mat4;
    fn set_transform(&mut self, transform: Mat4);
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    fn draw(&self, ctx: &mut RenderContext);
    fn draw_pick(&self, ctx: &mut RenderContext);
    fn build_ui(&mut self, ui: &imgui::Ui);
    fn build_pick_ui(&self, ui: &imgui::Ui, pick: &PickResult);
    fn refresh(&mut self);
}
```

### Quantity Trait

```rust
pub trait Quantity: Send + Sync {
    fn name(&self) -> &str;
    fn structure_name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    fn build_ui(&mut self, ui: &imgui::Ui);
    fn refresh(&mut self);
}

pub trait HasQuantities: Structure {
    fn add_quantity(&mut self, quantity: Box<dyn Quantity>);
    fn get_quantity(&self, name: &str) -> Option<&dyn Quantity>;
    fn remove_quantity(&mut self, name: &str);
    fn quantities(&self) -> &[Box<dyn Quantity>];
}
```

### Marker Traits for Quantity Domains

```rust
pub trait VertexQuantity: Quantity {}
pub trait FaceQuantity: Quantity {}
pub trait EdgeQuantity: Quantity {}
pub trait CellQuantity: Quantity {}
```

---

## 5. State Management

```rust
use std::sync::{Arc, RwLock, OnceLock};
use std::collections::HashMap;

static CONTEXT: OnceLock<RwLock<Context>> = OnceLock::new();

pub struct Context {
    pub initialized: bool,
    pub structures: HashMap<String, HashMap<String, Box<dyn Structure>>>,
    pub groups: HashMap<String, Group>,
    pub slice_planes: Vec<SlicePlane>,
    pub length_scale: f32,
    pub bounding_box: (Vec3, Vec3),
    pub engine: Option<RenderEngine>,
    pub user_callback: Option<Box<dyn FnMut(&imgui::Ui) + Send>>,
}

pub fn with_context<F, R>(f: F) -> R
where F: FnOnce(&Context) -> R;

pub fn with_context_mut<F, R>(f: F) -> R
where F: FnOnce(&mut Context) -> R;
```

### Handle-Based API

```rust
pub struct PointCloudHandle { name: String }
pub struct SurfaceMeshHandle { name: String }

impl PointCloudHandle {
    pub fn add_scalar_quantity(&self, name: &str, values: &[f32]) -> &Self;
    pub fn add_vector_quantity(&self, name: &str, vectors: &[Vec3]) -> &Self;
    pub fn add_color_quantity(&self, name: &str, colors: &[Vec3]) -> &Self;
}
```

---

## 6. Rendering Architecture

```rust
pub struct RenderEngine {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface>,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: wgpu::Texture,
    pub materials: MaterialRegistry,
    pub color_maps: ColorMapRegistry,
    pub imgui_renderer: dear_imgui_wgpu::Renderer,
}

impl RenderEngine {
    pub fn new_windowed(window: &winit::window::Window) -> Result<Self, RenderError>;
    pub fn new_headless(width: u32, height: u32) -> Result<Self, RenderError>;
    pub fn resize(&mut self, width: u32, height: u32);
    pub fn render_frame(&mut self, scene: &Scene, ui_callback: impl FnOnce(&imgui::Ui));
    pub fn screenshot(&self) -> image::RgbaImage;
}

pub struct RenderContext<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub depth_view: &'a wgpu::TextureView,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub camera: &'a Camera,
    pub materials: &'a MaterialRegistry,
}
```

---

## 7. Testing Strategy

### Layer 1: Unit Tests
Per-crate unit tests for individual components.

### Layer 2: Ported C++ Tests
Direct translations from polyscope `test/src/*.cpp` to define success criteria.

### Layer 3: Property-Based Tests
Using `proptest` to generate random inputs and verify invariants.

### Layer 4: Headless Render Tests
Render to headless wgpu context and verify output.

---

## 8. Implementation Phases

### Phase 1: Foundation
- Workspace setup, crate scaffolding
- Core traits, registry, state management
- wgpu engine initialization
- Basic window with event loop
- `init()`, `show()`, `shutdown()`

### Phase 2: First Structure (PointCloud)
- PointCloud structure
- Sphere impostor shader (WGSL)
- Basic camera controls
- Point rendering

### Phase 3: PointCloud Quantities
- ScalarQuantity with color mapping
- VectorQuantity with arrows
- ColorQuantity
- Color bar widget

### Phase 4: UI Integration
- dear-imgui-rs setup
- Structure tree panel
- Quantity controls
- Picking system

### Phase 5: SurfaceMesh
- SurfaceMesh structure
- Surface shaders
- Vertex/Face/Edge quantities
- Parameterization quantity

### Phase 6: Remaining Structures
- CurveNetwork
- VolumeMesh
- VolumeGrid
- CameraView

### Phase 7: Advanced Features
- Slice planes
- Transformation gizmos
- Groups
- Screenshots
- Ground plane
- Materials system

### Phase 8: Polish & Documentation
- API documentation
- Examples
- Performance optimization
- Publishing prep

---

## 9. Dependencies

```toml
[workspace.dependencies]
# Math
glam = "0.29"

# Rendering
wgpu = "24"
winit = "0.30"

# UI
dear-imgui-rs = "0.8"
dear-imgui-wgpu = "0.8"
dear-imgui-winit = "0.8"
dear-imguizmo = "0.8"
dear-implot = "0.8"

# Utilities
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
image = "0.25"
ply-rs = "0.1"
log = "0.4"
thiserror = "2.0"

# Testing
proptest = "1.5"
```

---

## 10. Reference

- Original C++ Polyscope: https://github.com/nmwsharp/polyscope
- Polyscope documentation: https://polyscope.run
- wgpu: https://wgpu.rs
- dear-imgui-rs: https://github.com/Latias94/dear-imgui-rs
- glam: https://github.com/bitshifter/glam-rs
