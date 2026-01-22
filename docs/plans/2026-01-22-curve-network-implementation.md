# CurveNetwork Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement CurveNetwork structure for visualizing nodes connected by edges with line/tube rendering and quantities.

**Architecture:** CurveNetwork stores nodes and edges (as separate tail/tip index arrays). Rendering supports both line mode (simple lines) and tube mode (cylinder geometry). Quantities attach to nodes or edges. Pattern follows existing PointCloud and SurfaceMesh implementations.

**Tech Stack:** Rust, wgpu, WGSL shaders, egui for UI

---

## Task 1: Core CurveNetwork Structure

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network.rs`
- Test: `crates/polyscope-structures/src/curve_network.rs` (inline tests)

**Step 1: Write failing test for basic creation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_network_creation() {
        let nodes = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let edges = vec![[0, 1], [1, 2]];

        let cn = CurveNetwork::new("test", nodes.clone(), edges);

        assert_eq!(cn.name(), "test");
        assert_eq!(cn.num_nodes(), 3);
        assert_eq!(cn.num_edges(), 2);
        assert_eq!(cn.nodes(), &nodes);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures test_curve_network_creation`
Expected: FAIL with compilation errors

**Step 3: Implement CurveNetwork struct**

```rust
//! Curve network structure.

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};

/// A curve network structure (nodes connected by edges).
pub struct CurveNetwork {
    name: String,

    // Geometry
    node_positions: Vec<Vec3>,
    edge_tail_inds: Vec<u32>,
    edge_tip_inds: Vec<u32>,

    // Computed geometry
    edge_centers: Vec<Vec3>,
    node_degrees: Vec<usize>,

    // Common structure fields
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Visualization parameters
    color: Vec3,
    radius: f32,
    radius_is_relative: bool,
    material: String,

    // Variable radius
    node_radius_quantity_name: Option<String>,
    edge_radius_quantity_name: Option<String>,
    node_radius_autoscale: bool,
    edge_radius_autoscale: bool,

    // GPU resources (placeholder for now)
    render_data: Option<()>,
}

impl CurveNetwork {
    /// Creates a new curve network from nodes and edges.
    pub fn new(name: impl Into<String>, nodes: Vec<Vec3>, edges: Vec<[u32; 2]>) -> Self {
        let edge_tail_inds: Vec<u32> = edges.iter().map(|e| e[0]).collect();
        let edge_tip_inds: Vec<u32> = edges.iter().map(|e| e[1]).collect();

        let mut cn = Self {
            name: name.into(),
            node_positions: nodes,
            edge_tail_inds,
            edge_tip_inds,
            edge_centers: Vec::new(),
            node_degrees: Vec::new(),
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            color: Vec3::new(0.2, 0.5, 0.8),
            radius: 0.005,
            radius_is_relative: true,
            material: "default".to_string(),
            node_radius_quantity_name: None,
            edge_radius_quantity_name: None,
            node_radius_autoscale: true,
            edge_radius_autoscale: true,
            render_data: None,
        };
        cn.recompute_geometry();
        cn
    }

    /// Returns the structure name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of nodes.
    pub fn num_nodes(&self) -> usize {
        self.node_positions.len()
    }

    /// Returns the number of edges.
    pub fn num_edges(&self) -> usize {
        self.edge_tail_inds.len()
    }

    /// Returns the node positions.
    pub fn nodes(&self) -> &[Vec3] {
        &self.node_positions
    }

    /// Returns the edge tail indices.
    pub fn edge_tail_inds(&self) -> &[u32] {
        &self.edge_tail_inds
    }

    /// Returns the edge tip indices.
    pub fn edge_tip_inds(&self) -> &[u32] {
        &self.edge_tip_inds
    }

    /// Returns the edge centers.
    pub fn edge_centers(&self) -> &[Vec3] {
        &self.edge_centers
    }

    /// Returns the node degrees.
    pub fn node_degrees(&self) -> &[usize] {
        &self.node_degrees
    }

    /// Recomputes edge centers and node degrees.
    fn recompute_geometry(&mut self) {
        // Compute edge centers
        self.edge_centers.clear();
        for i in 0..self.edge_tail_inds.len() {
            let tail = self.node_positions[self.edge_tail_inds[i] as usize];
            let tip = self.node_positions[self.edge_tip_inds[i] as usize];
            self.edge_centers.push((tail + tip) * 0.5);
        }

        // Compute node degrees
        self.node_degrees = vec![0; self.node_positions.len()];
        for &tail in &self.edge_tail_inds {
            self.node_degrees[tail as usize] += 1;
        }
        for &tip in &self.edge_tip_inds {
            self.node_degrees[tip as usize] += 1;
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures test_curve_network_creation`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/curve_network.rs
git commit -m "feat(curve_network): add core CurveNetwork struct with geometry"
```

---

## Task 2: Connectivity Helper Constructors

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_curve_network_line_connectivity() {
    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
    ];

    let cn = CurveNetwork::new_line("line", nodes);

    assert_eq!(cn.num_edges(), 3); // 0-1, 1-2, 2-3
    assert_eq!(cn.edge_tail_inds(), &[0, 1, 2]);
    assert_eq!(cn.edge_tip_inds(), &[1, 2, 3]);
}

#[test]
fn test_curve_network_loop_connectivity() {
    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
    ];

    let cn = CurveNetwork::new_loop("loop", nodes);

    assert_eq!(cn.num_edges(), 3); // 0-1, 1-2, 2-0
    assert_eq!(cn.edge_tail_inds(), &[0, 1, 2]);
    assert_eq!(cn.edge_tip_inds(), &[1, 2, 0]);
}

#[test]
fn test_curve_network_segments_connectivity() {
    let nodes = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
    ];

    let cn = CurveNetwork::new_segments("segments", nodes);

    assert_eq!(cn.num_edges(), 2); // 0-1, 2-3
    assert_eq!(cn.edge_tail_inds(), &[0, 2]);
    assert_eq!(cn.edge_tip_inds(), &[1, 3]);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures curve_network`
Expected: FAIL

**Step 3: Implement helper constructors**

```rust
impl CurveNetwork {
    /// Creates a curve network as a connected line (0-1-2-3-...).
    pub fn new_line(name: impl Into<String>, nodes: Vec<Vec3>) -> Self {
        let n = nodes.len();
        let edges: Vec<[u32; 2]> = (0..n.saturating_sub(1))
            .map(|i| [i as u32, (i + 1) as u32])
            .collect();
        Self::new(name, nodes, edges)
    }

    /// Creates a curve network as a closed loop (0-1-2-...-n-0).
    pub fn new_loop(name: impl Into<String>, nodes: Vec<Vec3>) -> Self {
        let n = nodes.len();
        let edges: Vec<[u32; 2]> = (0..n)
            .map(|i| [i as u32, ((i + 1) % n) as u32])
            .collect();
        Self::new(name, nodes, edges)
    }

    /// Creates a curve network as separate segments (0-1, 2-3, 4-5, ...).
    pub fn new_segments(name: impl Into<String>, nodes: Vec<Vec3>) -> Self {
        let n = nodes.len();
        let edges: Vec<[u32; 2]> = (0..n / 2)
            .map(|i| [(i * 2) as u32, (i * 2 + 1) as u32])
            .collect();
        Self::new(name, nodes, edges)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p polyscope-structures curve_network`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/curve_network.rs
git commit -m "feat(curve_network): add line/loop/segments connectivity helpers"
```

---

## Task 3: Implement Structure Trait

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_curve_network_bounding_box() {
    let nodes = vec![
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let edges = vec![[0, 1], [1, 2]];

    let cn = CurveNetwork::new("test", nodes, edges);

    let (min, max) = cn.bounding_box().unwrap();
    assert_eq!(min, Vec3::new(-1.0, 0.0, 0.0));
    assert_eq!(max, Vec3::new(1.0, 2.0, 0.0));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures test_curve_network_bounding_box`
Expected: FAIL

**Step 3: Implement Structure trait**

```rust
impl Structure for CurveNetwork {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn type_name(&self) -> &'static str {
        "CurveNetwork"
    }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        if self.node_positions.is_empty() {
            return None;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for &v in &self.node_positions {
            min = min.min(v);
            max = max.max(v);
        }

        Some((min, max))
    }

    fn length_scale(&self) -> f32 {
        self.bounding_box()
            .map(|(min, max)| (max - min).length())
            .unwrap_or(1.0)
    }

    fn transform(&self) -> Mat4 {
        self.transform
    }

    fn set_transform(&mut self, transform: Mat4) {
        self.transform = transform;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {
            return;
        }
        // TODO: Implement rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {
            return;
        }
        // TODO: Implement picking
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // TODO: Implement pick UI
    }

    fn refresh(&mut self) {
        self.recompute_geometry();
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for CurveNetwork {
    fn add_quantity(&mut self, quantity: Box<dyn Quantity>) {
        self.quantities.push(quantity);
    }

    fn get_quantity(&self, name: &str) -> Option<&dyn Quantity> {
        self.quantities
            .iter()
            .find(|q| q.name() == name)
            .map(|q| q.as_ref())
    }

    fn get_quantity_mut(&mut self, name: &str) -> Option<&mut Box<dyn Quantity>> {
        self.quantities.iter_mut().find(|q| q.name() == name)
    }

    fn remove_quantity(&mut self, name: &str) -> Option<Box<dyn Quantity>> {
        let idx = self.quantities.iter().position(|q| q.name() == name)?;
        Some(self.quantities.remove(idx))
    }

    fn quantities(&self) -> &[Box<dyn Quantity>] {
        &self.quantities
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures test_curve_network_bounding_box`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/curve_network.rs
git commit -m "feat(curve_network): implement Structure and HasQuantities traits"
```

---

## Task 4: Visualization Parameter Accessors

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_curve_network_visualization_params() {
    let nodes = vec![Vec3::ZERO, Vec3::X];
    let edges = vec![[0, 1]];
    let mut cn = CurveNetwork::new("test", nodes, edges);

    // Test defaults
    assert_eq!(cn.color(), Vec3::new(0.2, 0.5, 0.8));
    assert_eq!(cn.radius(), 0.005);
    assert!(cn.radius_is_relative());

    // Test setters
    cn.set_color(Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(cn.color(), Vec3::new(1.0, 0.0, 0.0));

    cn.set_radius(0.1, false);
    assert_eq!(cn.radius(), 0.1);
    assert!(!cn.radius_is_relative());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures test_curve_network_visualization`
Expected: FAIL

**Step 3: Implement accessors**

```rust
impl CurveNetwork {
    // ... existing methods ...

    /// Gets the base color.
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the base color.
    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        self
    }

    /// Gets the radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Returns whether the radius is relative to scene scale.
    pub fn radius_is_relative(&self) -> bool {
        self.radius_is_relative
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, radius: f32, is_relative: bool) -> &mut Self {
        self.radius = radius;
        self.radius_is_relative = is_relative;
        self
    }

    /// Gets the material name.
    pub fn material(&self) -> &str {
        &self.material
    }

    /// Sets the material name.
    pub fn set_material(&mut self, material: impl Into<String>) -> &mut Self {
        self.material = material.into();
        self
    }

    /// Updates the node positions.
    pub fn update_node_positions(&mut self, nodes: Vec<Vec3>) {
        self.node_positions = nodes;
        self.recompute_geometry();
        self.refresh();
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures test_curve_network_visualization`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/curve_network.rs
git commit -m "feat(curve_network): add visualization parameter accessors"
```

---

## Task 5: Registration Functions in Main Crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs`

**Step 1: Write the registration functions**

Add after `register_surface_mesh`:

```rust
/// Registers a curve network with polyscope.
pub fn register_curve_network(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
    edges: Vec<[u32; 2]>,
) -> CurveNetworkHandle {
    let name = name.into();
    let curve_network = CurveNetwork::new(name.clone(), nodes, edges);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(curve_network))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a connected line.
pub fn register_curve_network_line(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let curve_network = CurveNetwork::new_line(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(curve_network))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a closed loop.
pub fn register_curve_network_loop(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let curve_network = CurveNetwork::new_loop(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(curve_network))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as separate segments.
pub fn register_curve_network_segments(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let curve_network = CurveNetwork::new_segments(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(curve_network))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Handle for a registered curve network.
#[derive(Clone)]
pub struct CurveNetworkHandle {
    name: String,
}

impl CurveNetworkHandle {
    /// Returns the name of this curve network.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Executes a closure with mutable access to a registered curve network.
pub fn with_curve_network<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut CurveNetwork) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("CurveNetwork", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<CurveNetwork>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered curve network.
pub fn with_curve_network_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&CurveNetwork) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .and_then(|s| s.as_any().downcast_ref::<CurveNetwork>())
            .map(f)
    })
}
```

**Step 2: Run build to verify it compiles**

Run: `cargo build -p polyscope`
Expected: Success

**Step 3: Commit**

```bash
git add crates/polyscope/src/lib.rs
git commit -m "feat(polyscope): add CurveNetwork registration functions"
```

---

## Task 6: CurveNetwork Render Data Structure

**Files:**
- Create: `crates/polyscope-render/src/curve_network_render.rs`
- Modify: `crates/polyscope-render/src/lib.rs`

**Step 1: Create render data structure**

```rust
//! Curve network GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// Uniforms for curve network rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CurveNetworkUniforms {
    pub color: [f32; 4],
    pub radius: f32,
    pub radius_is_relative: u32,
    pub render_mode: u32,  // 0 = line, 1 = tube
    pub _padding: f32,
}

impl Default for CurveNetworkUniforms {
    fn default() -> Self {
        Self {
            color: [0.2, 0.5, 0.8, 1.0],
            radius: 0.005,
            radius_is_relative: 1,
            render_mode: 0,
            _padding: 0.0,
        }
    }
}

/// GPU resources for rendering a curve network.
pub struct CurveNetworkRenderData {
    // Node data (for sphere rendering)
    pub node_buffer: wgpu::Buffer,
    pub node_color_buffer: wgpu::Buffer,
    pub num_nodes: u32,

    // Edge data (for line/tube rendering)
    pub edge_vertex_buffer: wgpu::Buffer,
    pub edge_color_buffer: wgpu::Buffer,
    pub num_edge_vertices: u32,

    // Uniforms
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl CurveNetworkRenderData {
    /// Creates new render data from curve network geometry.
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        nodes: &[Vec3],
        edge_tail_inds: &[u32],
        edge_tip_inds: &[u32],
    ) -> Self {
        let num_nodes = nodes.len() as u32;
        let num_edges = edge_tail_inds.len();

        // Node buffer (positions as vec4)
        let node_data: Vec<f32> = nodes
            .iter()
            .flat_map(|v| [v.x, v.y, v.z, 1.0])
            .collect();
        let node_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network nodes"),
            contents: bytemuck::cast_slice(&node_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Node colors (default white)
        let node_color_data: Vec<f32> = vec![1.0; nodes.len() * 4];
        let node_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network node colors"),
            contents: bytemuck::cast_slice(&node_color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Edge vertices (2 vertices per edge for line rendering)
        let mut edge_vertex_data: Vec<f32> = Vec::with_capacity(num_edges * 2 * 4);
        for i in 0..num_edges {
            let tail = nodes[edge_tail_inds[i] as usize];
            let tip = nodes[edge_tip_inds[i] as usize];
            edge_vertex_data.extend_from_slice(&[tail.x, tail.y, tail.z, 1.0]);
            edge_vertex_data.extend_from_slice(&[tip.x, tip.y, tip.z, 1.0]);
        }
        let edge_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network edge vertices"),
            contents: bytemuck::cast_slice(&edge_vertex_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Edge colors (default white)
        let edge_color_data: Vec<f32> = vec![1.0; num_edges * 2 * 4];
        let edge_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network edge colors"),
            contents: bytemuck::cast_slice(&edge_color_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Uniform buffer
        let uniforms = CurveNetworkUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("curve network bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: node_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: node_color_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            node_buffer,
            node_color_buffer,
            num_nodes,
            edge_vertex_buffer,
            edge_color_buffer,
            num_edge_vertices: (num_edges * 2) as u32,
            uniform_buffer,
            bind_group,
        }
    }

    /// Updates the uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CurveNetworkUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_network_uniforms_size() {
        let size = std::mem::size_of::<CurveNetworkUniforms>();
        assert_eq!(size % 16, 0, "Uniforms must be 16-byte aligned");
        assert_eq!(size, 32, "Expected 32 bytes");
    }
}
```

**Step 2: Add to lib.rs exports**

Add to `crates/polyscope-render/src/lib.rs`:

```rust
mod curve_network_render;
pub use curve_network_render::{CurveNetworkRenderData, CurveNetworkUniforms};
```

**Step 3: Run build and test**

Run: `cargo test -p polyscope-render`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/curve_network_render.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add CurveNetworkRenderData structure"
```

---

## Task 7: Edge Line Shader

**Files:**
- Create: `crates/polyscope-render/src/shaders/curve_network_edge.wgsl`

**Step 1: Create edge line shader**

```wgsl
// Curve network edge shader (line rendering)

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position.xyz, 1.0);

    // Use per-vertex color if non-zero, otherwise use uniform color
    let color_sum = in.color.r + in.color.g + in.color.b;
    if (color_sum > 0.001) {
        out.color = in.color;
    } else {
        out.color = uniforms.color;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/shaders/curve_network_edge.wgsl
git commit -m "feat(render): add curve network edge line shader"
```

---

## Task 8: Edge Pipeline in RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine.rs`

**Step 1: Add curve network edge pipeline**

Add fields to `RenderEngine`:

```rust
pub curve_network_edge_pipeline: Option<wgpu::RenderPipeline>,
pub curve_network_bind_group_layout: Option<wgpu::BindGroupLayout>,
```

Add initialization method:

```rust
fn create_curve_network_edge_pipeline(&mut self) {
    let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("curve network bind group layout"),
        entries: &[
            // Camera uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Curve network uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Node positions (storage)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Node colors (storage)
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("curve network edge shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/curve_network_edge.wgsl").into()),
    });

    let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("curve network edge pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("curve network edge pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[
                // Position buffer
                wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    }],
                },
                // Color buffer
                wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 1,
                    }],
                },
            ],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: self.surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    self.curve_network_bind_group_layout = Some(bind_group_layout);
    self.curve_network_edge_pipeline = Some(pipeline);
}

pub fn curve_network_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    self.curve_network_bind_group_layout.as_ref().expect("curve network not initialized")
}
```

Call in `new_windowed` after other pipelines.

**Step 2: Run build**

Run: `cargo build -p polyscope-render`
Expected: Success

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/engine.rs
git commit -m "feat(render): add curve network edge pipeline"
```

---

## Task 9: Integrate CurveNetwork into App Rendering

**Files:**
- Modify: `crates/polyscope/src/app.rs`
- Modify: `crates/polyscope-structures/src/curve_network.rs`

**Step 1: Add GPU init to CurveNetwork**

In `curve_network.rs`, update render_data type and add init method:

```rust
use polyscope_render::{CurveNetworkRenderData, CurveNetworkUniforms};

// Change render_data type
render_data: Option<CurveNetworkRenderData>,

// Add methods
pub fn init_gpu_resources(
    &mut self,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
) {
    self.render_data = Some(CurveNetworkRenderData::new(
        device,
        bind_group_layout,
        camera_buffer,
        &self.node_positions,
        &self.edge_tail_inds,
        &self.edge_tip_inds,
    ));
}

pub fn render_data(&self) -> Option<&CurveNetworkRenderData> {
    self.render_data.as_ref()
}

pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
    let Some(render_data) = &self.render_data else {
        return;
    };

    let uniforms = CurveNetworkUniforms {
        color: [self.color.x, self.color.y, self.color.z, 1.0],
        radius: self.radius,
        radius_is_relative: u32::from(self.radius_is_relative),
        render_mode: 0, // Line mode for now
        _padding: 0.0,
    };
    render_data.update_uniforms(queue, &uniforms);
}
```

**Step 2: Add CurveNetwork rendering to app.rs**

In render() method, add after surface mesh rendering:

```rust
// Draw curve networks (edges)
if let Some(pipeline) = &engine.curve_network_edge_pipeline {
    render_pass.set_pipeline(pipeline);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !structure.is_enabled() {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    if let Some(render_data) = cn.render_data() {
                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, render_data.edge_vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, render_data.edge_color_buffer.slice(..));
                        render_pass.draw(0..render_data.num_edge_vertices, 0..1);
                    }
                }
            }
        }
    });
}
```

Add GPU init and update in the appropriate sections (similar to SurfaceMesh).

**Step 3: Build and verify**

Run: `cargo build --workspace`
Expected: Success

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs crates/polyscope-structures/src/curve_network.rs
git commit -m "feat: integrate CurveNetwork rendering into app"
```

---

## Task 10: Demo Example

**Files:**
- Create: `examples/curve_network_demo.rs`

**Step 1: Create demo**

```rust
//! Curve network demonstration.
//!
//! Run with: cargo run --example curve_network_demo

use glam::Vec3;
use std::f32::consts::PI;

fn main() {
    env_logger::init();
    polyscope::init().expect("Failed to initialize polyscope");

    // Create a helix curve
    let nodes: Vec<Vec3> = (0..100)
        .map(|i| {
            let t = i as f32 * 0.1;
            Vec3::new(t.cos(), t * 0.05, t.sin())
        })
        .collect();

    polyscope::register_curve_network_line("helix", nodes);

    polyscope::with_curve_network("helix", |cn| {
        cn.set_color(Vec3::new(0.2, 0.8, 0.4));
        cn.set_radius(0.01, true);
    });

    // Create a triangle loop
    let triangle_nodes = vec![
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(-1.5, 1.0, 0.5),
    ];

    polyscope::register_curve_network_loop("triangle", triangle_nodes);

    polyscope::with_curve_network("triangle", |cn| {
        cn.set_color(Vec3::new(0.8, 0.2, 0.2));
    });

    // Create some random segments
    let segment_nodes: Vec<Vec3> = (0..10)
        .map(|i| {
            let x = 2.0 + (i as f32 * 0.3);
            let y = (i as f32 * 0.5).sin();
            Vec3::new(x, y, 0.0)
        })
        .collect();

    polyscope::register_curve_network_segments("segments", segment_nodes);

    polyscope::with_curve_network("segments", |cn| {
        cn.set_color(Vec3::new(0.8, 0.8, 0.2));
    });

    println!("Curve network demo running...");
    println!("Controls:");
    println!("  - Left drag: Orbit camera");
    println!("  - Right drag: Pan camera");
    println!("  - Scroll: Zoom");
    println!("  - ESC: Exit");

    polyscope::show();
}
```

**Step 2: Build and run**

Run: `cargo run --example curve_network_demo`
Expected: Window opens showing helix, triangle loop, and segments

**Step 3: Commit**

```bash
git add examples/curve_network_demo.rs
git commit -m "feat: add curve network demo example"
```

---

## Task 11: Node Scalar Quantity

**Files:**
- Create: `crates/polyscope-structures/src/curve_network/quantities.rs`
- Modify: `crates/polyscope-structures/src/curve_network.rs` (convert to module)

**Step 1: Convert curve_network.rs to module**

Rename `curve_network.rs` to `curve_network/mod.rs` and create `quantities.rs`.

**Step 2: Implement CurveNodeScalarQuantity**

```rust
// quantities.rs
use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind};

/// Scalar quantity defined on curve network nodes.
pub struct CurveNodeScalarQuantity {
    name: String,
    parent_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    data_range: (f32, f32),
    viz_range: (f32, f32),
}

impl CurveNodeScalarQuantity {
    pub fn new(name: impl Into<String>, parent_name: impl Into<String>, values: Vec<f32>) -> Self {
        let data_range = values
            .iter()
            .fold((f32::MAX, f32::MIN), |(min, max), &v| (min.min(v), max.max(v)));

        Self {
            name: name.into(),
            parent_name: parent_name.into(),
            values,
            enabled: false,
            colormap_name: "viridis".to_string(),
            data_range,
            viz_range: data_range,
        }
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    pub fn set_colormap(&mut self, name: impl Into<String>) {
        self.colormap_name = name.into();
    }

    pub fn data_range(&self) -> (f32, f32) {
        self.data_range
    }

    pub fn viz_range(&self) -> (f32, f32) {
        self.viz_range
    }

    pub fn set_viz_range(&mut self, range: (f32, f32)) {
        self.viz_range = range;
    }
}

impl Quantity for CurveNodeScalarQuantity {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.parent_name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Scalar
    }

    fn data_size(&self) -> usize {
        self.values.len()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn refresh(&mut self) {}
}
```

**Step 3: Add quantity methods to CurveNetwork**

```rust
pub fn add_node_scalar_quantity(&mut self, name: impl Into<String>, values: Vec<f32>) -> &mut Self {
    let quantity = CurveNodeScalarQuantity::new(name, self.name.clone(), values);
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 4: Write test**

```rust
#[test]
fn test_node_scalar_quantity() {
    let nodes = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    let edges = vec![[0, 1], [1, 2]];
    let mut cn = CurveNetwork::new("test", nodes, edges);

    cn.add_node_scalar_quantity("height", vec![0.0, 0.5, 1.0]);

    let q = cn.get_quantity("height").expect("quantity not found");
    assert_eq!(q.data_size(), 3);
}
```

**Step 5: Run test and commit**

Run: `cargo test -p polyscope-structures test_node_scalar`
Expected: PASS

```bash
git add crates/polyscope-structures/src/curve_network/
git commit -m "feat(curve_network): add CurveNodeScalarQuantity"
```

---

## Task 12: Remaining Quantities (Edge Scalar, Node/Edge Color, Node/Edge Vector)

Follow the same pattern as Task 11 for:
- `CurveEdgeScalarQuantity` (with `node_average_values` computed)
- `CurveNodeColorQuantity`
- `CurveEdgeColorQuantity` (with `node_average_colors` computed)
- `CurveNodeVectorQuantity`
- `CurveEdgeVectorQuantity`

Each quantity follows the pattern:
1. Create struct implementing `Quantity` trait
2. Add convenience method to `CurveNetwork`
3. Add test
4. Commit

---

## Task 13: UI Integration

**Files:**
- Modify: `crates/polyscope-ui/src/lib.rs`
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add CurveNetwork UI builder**

```rust
pub fn build_curve_network_ui(
    ui: &mut egui::Ui,
    num_nodes: usize,
    num_edges: usize,
    color: &mut [f32; 3],
    radius: &mut f32,
) -> bool {
    let mut changed = false;

    ui.label(format!("Nodes: {}", num_nodes));
    ui.label(format!("Edges: {}", num_edges));

    if ui.color_edit_button_rgb(color).changed() {
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label("Radius:");
        if ui.add(egui::DragValue::new(radius).speed(0.001).range(0.001..=1.0)).changed() {
            changed = true;
        }
    });

    changed
}
```

**Step 2: Add CurveNetwork UI building in app.rs**

```rust
if type_name == "CurveNetwork" {
    if let Some(cn) = s.as_any_mut().downcast_mut::<CurveNetwork>() {
        cn.build_egui_ui(ui);
    }
}
```

**Step 3: Implement `build_egui_ui` on CurveNetwork**

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/lib.rs crates/polyscope/src/app.rs crates/polyscope-structures/src/curve_network/mod.rs
git commit -m "feat(ui): add CurveNetwork UI controls"
```

---

## Summary

**Total Tasks:** 13

**Key Files:**
- `crates/polyscope-structures/src/curve_network/mod.rs` - Main structure
- `crates/polyscope-structures/src/curve_network/quantities.rs` - Quantities
- `crates/polyscope-render/src/curve_network_render.rs` - GPU resources
- `crates/polyscope-render/src/shaders/curve_network_edge.wgsl` - Edge shader
- `crates/polyscope/src/lib.rs` - Registration functions
- `crates/polyscope/src/app.rs` - Rendering integration
- `examples/curve_network_demo.rs` - Demo

**Commits:** ~13 commits following TDD pattern
