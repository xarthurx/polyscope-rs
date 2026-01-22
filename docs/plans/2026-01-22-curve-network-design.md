# CurveNetwork Implementation Design

**Date**: 2026-01-22
**Status**: Approved
**Purpose**: Implement CurveNetwork structure for visualizing nodes connected by edges

---

## 1. Overview

CurveNetwork is a structure for visualizing collections of nodes (3D points) connected by edges. It supports rendering as lines or tubes, with quantities attachable to both nodes and edges.

---

## 2. Core Data Structure

```rust
pub struct CurveNetwork {
    name: String,

    // Geometry
    node_positions: Vec<Vec3>,
    edge_tail_inds: Vec<u32>,    // Start node of each edge
    edge_tip_inds: Vec<u32>,     // End node of each edge

    // Computed geometry
    edge_centers: Vec<Vec3>,     // Midpoint of each edge
    node_degrees: Vec<usize>,    // Degree of each node

    // Common structure fields
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Visualization parameters
    color: Vec3,
    radius: f32,
    radius_is_relative: bool,
    material: String,

    // Variable radius (references scalar quantities by name)
    node_radius_quantity_name: Option<String>,
    edge_radius_quantity_name: Option<String>,
    node_radius_autoscale: bool,
    edge_radius_autoscale: bool,

    // GPU resources
    render_data: Option<CurveNetworkRenderData>,
}
```

---

## 3. Quantities

### Scalar Quantities

```rust
pub struct CurveNodeScalarQuantity {
    name: String,
    parent_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    data_range: (f32, f32),
    viz_range: (f32, f32),
    isolines_enabled: bool,
    isoline_period: f32,
    isoline_darkness: f32,
}

pub struct CurveEdgeScalarQuantity {
    // Same as node scalar, plus:
    values: Vec<f32>,
    node_average_values: Vec<f32>,  // For smooth edge rendering
    // ... other fields
}
```

### Color Quantities

```rust
pub struct CurveNodeColorQuantity {
    name: String,
    parent_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

pub struct CurveEdgeColorQuantity {
    colors: Vec<Vec3>,
    node_average_colors: Vec<Vec3>,  // For smooth edge rendering
    // ... other fields
}
```

### Vector Quantities

```rust
pub struct CurveNodeVectorQuantity {
    name: String,
    parent_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_type: VectorType,
    length_scale: f32,
    length_is_relative: bool,
    length_range: f32,
    radius: f32,
    radius_is_relative: bool,
    color: Vec3,
    material: String,
    render_data: Option<VectorRenderData>,
}

pub struct CurveEdgeVectorQuantity {
    // Same fields, vectors rooted at edge_centers
}

#[derive(Clone, Copy)]
pub enum VectorType {
    Standard,  // Auto-scaled to scene
    Ambient,   // World-space length
}
```

---

## 4. Rendering

### GPU Resources

```rust
pub struct CurveNetworkRenderData {
    // Node rendering (sphere impostors)
    node_buffer: wgpu::Buffer,
    node_color_buffer: wgpu::Buffer,
    node_radius_buffer: wgpu::Buffer,

    // Edge rendering (line or tube mode)
    edge_vertex_buffer: wgpu::Buffer,
    edge_index_buffer: wgpu::Buffer,
    edge_color_buffer: wgpu::Buffer,
    edge_radius_buffer: wgpu::Buffer,

    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    num_nodes: u32,
    num_edges: u32,
}

pub struct CurveNetworkUniforms {
    color: [f32; 4],
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,  // 0 = line, 1 = tube
    _padding: f32,
}
```

### Shaders

1. `curve_network_node.wgsl` - Sphere impostors for nodes (reuse PointCloud logic)
2. `curve_network_edge_line.wgsl` - Simple line rendering
3. `curve_network_edge_tube.wgsl` - Cylinder/tube rendering with lighting

---

## 5. Public API

### Registration Functions

```rust
/// Register a curve network with explicit edges
pub fn register_curve_network(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
    edges: Vec<[u32; 2]>,
) -> &'static mut CurveNetwork;

/// Register as connected line (0-1-2-3-...)
pub fn register_curve_network_line(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> &'static mut CurveNetwork;

/// Register as closed loop (0-1-2-...-n-0)
pub fn register_curve_network_loop(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> &'static mut CurveNetwork;

/// Register as separate segments (0-1, 2-3, 4-5, ...)
pub fn register_curve_network_segments(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> &'static mut CurveNetwork;

/// Access a registered curve network
pub fn with_curve_network<F, R>(name: &str, f: F) -> Option<R>
where F: FnOnce(&mut CurveNetwork) -> R;
```

### CurveNetwork Methods

```rust
impl CurveNetwork {
    // Quantities
    pub fn add_node_scalar_quantity(&mut self, name: &str, values: Vec<f32>) -> &mut Self;
    pub fn add_edge_scalar_quantity(&mut self, name: &str, values: Vec<f32>) -> &mut Self;
    pub fn add_node_color_quantity(&mut self, name: &str, colors: Vec<Vec3>) -> &mut Self;
    pub fn add_edge_color_quantity(&mut self, name: &str, colors: Vec<Vec3>) -> &mut Self;
    pub fn add_node_vector_quantity(&mut self, name: &str, vectors: Vec<Vec3>) -> &mut Self;
    pub fn add_edge_vector_quantity(&mut self, name: &str, vectors: Vec<Vec3>) -> &mut Self;

    // Variable radius
    pub fn set_node_radius_quantity(&mut self, name: &str, autoscale: bool);
    pub fn set_edge_radius_quantity(&mut self, name: &str, autoscale: bool);
    pub fn clear_node_radius_quantity(&mut self);
    pub fn clear_edge_radius_quantity(&mut self);

    // Visualization options
    pub fn set_color(&mut self, color: Vec3) -> &mut Self;
    pub fn set_radius(&mut self, radius: f32, is_relative: bool) -> &mut Self;
    pub fn set_material(&mut self, name: &str) -> &mut Self;

    // Update geometry
    pub fn update_node_positions(&mut self, nodes: Vec<Vec3>);
}
```

---

## 6. Testing

### Unit Tests

- `test_curve_network_creation` - Basic construction
- `test_curve_network_line_connectivity` - Line connectivity (0-1-2-3)
- `test_curve_network_loop_connectivity` - Loop connectivity (0-1-2-0)
- `test_curve_network_segments_connectivity` - Segments (0-1, 2-3)
- `test_edge_centers_computation` - Edge midpoint calculation
- `test_node_degrees_computation` - Node degree calculation
- `test_node_scalar_quantity` - Node scalar attachment
- `test_edge_scalar_quantity` - Edge scalar attachment
- `test_node_average_values` - Edge->node value averaging

### Demo

```rust
// examples/curve_network_demo.rs
fn main() {
    polyscope::init();

    // Create a helix curve
    let nodes: Vec<Vec3> = (0..100)
        .map(|i| {
            let t = i as f32 * 0.1;
            Vec3::new(t.cos(), t * 0.1, t.sin())
        })
        .collect();

    polyscope::register_curve_network_line("helix", nodes);

    polyscope::with_curve_network("helix", |c| {
        c.set_radius(0.02, true);
        c.set_color(Vec3::new(0.2, 0.8, 0.4));
    });

    polyscope::show();
}
```

---

## 7. Implementation Order

1. Core CurveNetwork struct with geometry storage
2. Registration functions and connectivity helpers
3. Node rendering (reuse PointCloud sphere impostor shader)
4. Edge line rendering (simple lines)
5. Edge tube rendering (cylinder geometry)
6. Node/Edge scalar quantities
7. Node/Edge color quantities
8. Node/Edge vector quantities
9. Variable radius support
10. UI integration
