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
        let edges: Vec<[u32; 2]> = (0..n).map(|i| [i as u32, ((i + 1) % n) as u32]).collect();
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

        for &p in &self.node_positions {
            min = min.min(p);
            max = max.max(p);
        }

        // Apply transform
        let transform = self.transform;
        let corners = [
            transform.transform_point3(Vec3::new(min.x, min.y, min.z)),
            transform.transform_point3(Vec3::new(max.x, min.y, min.z)),
            transform.transform_point3(Vec3::new(min.x, max.y, min.z)),
            transform.transform_point3(Vec3::new(max.x, max.y, min.z)),
            transform.transform_point3(Vec3::new(min.x, min.y, max.z)),
            transform.transform_point3(Vec3::new(max.x, min.y, max.z)),
            transform.transform_point3(Vec3::new(min.x, max.y, max.z)),
            transform.transform_point3(Vec3::new(max.x, max.y, max.z)),
        ];

        let mut world_min = Vec3::splat(f32::MAX);
        let mut world_max = Vec3::splat(f32::MIN);
        for corner in corners {
            world_min = world_min.min(corner);
            world_max = world_max.max(corner);
        }

        Some((world_min, world_max))
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
        // TODO: Implement curve network rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {
            return;
        }
        // TODO: Implement curve network picking
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // TODO: Implement pick UI
    }

    fn refresh(&mut self) {
        self.recompute_geometry();
        // TODO: Refresh GPU buffers
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
}
