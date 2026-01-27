//! Curve network structure.

mod quantities;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{
    ColorMapRegistry, CurveNetworkRenderData, CurveNetworkUniforms, PointUniforms,
};

pub use quantities::*;

/// A curve network structure (nodes connected by edges).
#[allow(dead_code)]
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
    /// Render mode: 0 = line, 1 = tube (cylinder)
    render_mode: u32,

    // Variable radius
    node_radius_quantity_name: Option<String>,
    edge_radius_quantity_name: Option<String>,
    node_radius_autoscale: bool,
    edge_radius_autoscale: bool,

    // GPU resources
    render_data: Option<CurveNetworkRenderData>,
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
            render_mode: 0, // Default to line rendering
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn num_nodes(&self) -> usize {
        self.node_positions.len()
    }

    /// Returns the number of edges.
    #[must_use]
    pub fn num_edges(&self) -> usize {
        self.edge_tail_inds.len()
    }

    /// Returns the node positions.
    #[must_use]
    pub fn nodes(&self) -> &[Vec3] {
        &self.node_positions
    }

    /// Returns the edge tail indices.
    #[must_use]
    pub fn edge_tail_inds(&self) -> &[u32] {
        &self.edge_tail_inds
    }

    /// Returns the edge tip indices.
    #[must_use]
    pub fn edge_tip_inds(&self) -> &[u32] {
        &self.edge_tip_inds
    }

    /// Returns the edge centers.
    #[must_use]
    pub fn edge_centers(&self) -> &[Vec3] {
        &self.edge_centers
    }

    /// Returns the node degrees.
    #[must_use]
    pub fn node_degrees(&self) -> &[usize] {
        &self.node_degrees
    }

    /// Gets the base color.
    #[must_use]
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the base color.
    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        self
    }

    /// Gets the radius.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Returns whether the radius is relative to scene scale.
    #[must_use]
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
    #[must_use]
    pub fn material(&self) -> &str {
        &self.material
    }

    /// Sets the material name.
    pub fn set_material(&mut self, material: impl Into<String>) -> &mut Self {
        self.material = material.into();
        self
    }

    /// Gets the render mode (0 = line, 1 = tube).
    #[must_use]
    pub fn render_mode(&self) -> u32 {
        self.render_mode
    }

    /// Sets the render mode (0 = line, 1 = tube).
    pub fn set_render_mode(&mut self, mode: u32) -> &mut Self {
        self.render_mode = mode.min(1); // Clamp to valid values
        self
    }

    /// Updates the node positions.
    pub fn update_node_positions(&mut self, nodes: Vec<Vec3>) {
        self.node_positions = nodes;
        self.recompute_geometry();
        self.refresh();
    }

    /// Adds a node scalar quantity to this curve network.
    pub fn add_node_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = CurveNodeScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds an edge scalar quantity to this curve network.
    pub fn add_edge_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = CurveEdgeScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a node color quantity to this curve network.
    pub fn add_node_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = CurveNodeColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds an edge color quantity to this curve network.
    pub fn add_edge_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = CurveEdgeColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a node vector quantity to this curve network.
    pub fn add_node_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = CurveNodeVectorQuantity::new(name, self.name.clone(), vectors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds an edge vector quantity to this curve network.
    pub fn add_edge_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = CurveEdgeVectorQuantity::new(name, self.name.clone(), vectors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Returns the currently active node scalar quantity, if any.
    #[must_use]
    pub fn active_node_scalar_quantity(&self) -> Option<&CurveNodeScalarQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<CurveNodeScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active edge scalar quantity, if any.
    #[must_use]
    pub fn active_edge_scalar_quantity(&self) -> Option<&CurveEdgeScalarQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<CurveEdgeScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active node color quantity, if any.
    #[must_use]
    pub fn active_node_color_quantity(&self) -> Option<&CurveNodeColorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<CurveNodeColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active edge color quantity, if any.
    #[must_use]
    pub fn active_edge_color_quantity(&self) -> Option<&CurveEdgeColorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<CurveEdgeColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Builds the egui UI for this curve network.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        let mut color = [self.color.x, self.color.y, self.color.z];
        let mut radius = self.radius;
        let mut radius_is_relative = self.radius_is_relative;
        let mut render_mode = self.render_mode;

        if polyscope_ui::build_curve_network_ui(
            ui,
            self.node_positions.len(),
            self.edge_tail_inds.len(),
            &mut radius,
            &mut radius_is_relative,
            &mut color,
            &mut render_mode,
        ) {
            self.color = Vec3::new(color[0], color[1], color[2]);
            self.radius = radius;
            self.radius_is_relative = radius_is_relative;
            self.render_mode = render_mode;
        }

        // Show quantities
        if !self.quantities.is_empty() {
            ui.separator();
            ui.label("Quantities:");
            for quantity in &mut self.quantities {
                // Cast to concrete types and call build_egui_ui
                if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveNodeScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveEdgeScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(cq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveNodeColorQuantity>()
                {
                    cq.build_egui_ui(ui);
                } else if let Some(cq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveEdgeColorQuantity>()
                {
                    cq.build_egui_ui(ui);
                } else if let Some(vq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveNodeVectorQuantity>()
                {
                    vq.build_egui_ui(ui);
                } else if let Some(vq) = quantity
                    .as_any_mut()
                    .downcast_mut::<CurveEdgeVectorQuantity>()
                {
                    vq.build_egui_ui(ui);
                }
            }
        }
    }

    /// Initializes GPU resources for this curve network.
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

    /// Returns the render data if initialized.
    #[must_use]
    pub fn render_data(&self) -> Option<&CurveNetworkRenderData> {
        self.render_data.as_ref()
    }

    /// Initializes tube rendering resources.
    pub fn init_tube_resources(
        &mut self,
        device: &wgpu::Device,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        render_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        if let Some(render_data) = &mut self.render_data {
            render_data.init_tube_resources(
                device,
                compute_bind_group_layout,
                render_bind_group_layout,
                camera_buffer,
            );
        }
    }

    /// Initializes node sphere rendering resources for tube mode joints.
    pub fn init_node_render_resources(
        &mut self,
        device: &wgpu::Device,
        point_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        if let Some(render_data) = &mut self.render_data {
            render_data.init_node_render_resources(device, point_bind_group_layout, camera_buffer);
        }
    }

    /// Updates GPU buffers based on current state.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
        let Some(render_data) = &self.render_data else {
            return;
        };

        let uniforms = CurveNetworkUniforms {
            color: [self.color.x, self.color.y, self.color.z, 1.0],
            radius: self.radius,
            radius_is_relative: u32::from(self.radius_is_relative),
            render_mode: self.render_mode,
            _padding: 0.0,
        };

        render_data.update_uniforms(queue, &uniforms);

        // Update node sphere uniforms for tube mode (slightly larger than tube radius to fill gaps)
        if self.render_mode == 1 && render_data.has_node_render_resources() {
            let model_matrix = self.transform.to_cols_array_2d();
            let node_uniforms = PointUniforms {
                model_matrix,
                // Make spheres slightly larger than tubes to ensure they fill gaps at joints
                point_radius: self.radius * 1.02,
                use_per_point_color: 0, // Use base color
                _padding: [0.0; 2],
                base_color: [self.color.x, self.color.y, self.color.z, 1.0],
            };
            render_data.update_node_uniforms(queue, &node_uniforms);
        }

        // Apply node color quantities
        if let Some(color_q) = self.active_node_color_quantity() {
            color_q.apply_to_render_data(queue, render_data);
        } else if let Some(scalar_q) = self.active_node_scalar_quantity() {
            if let Some(colormap) = color_maps.get(scalar_q.colormap_name()) {
                let colors = scalar_q.compute_colors(colormap);
                render_data.update_node_colors(queue, &colors);
            }
        }

        // Apply edge color quantities
        if let Some(color_q) = self.active_edge_color_quantity() {
            color_q.apply_to_render_data(queue, render_data);
        } else if let Some(scalar_q) = self.active_edge_scalar_quantity() {
            if let Some(colormap) = color_maps.get(scalar_q.colormap_name()) {
                let colors = scalar_q.compute_colors(colormap);
                render_data.update_edge_colors(queue, &colors);
            }
        }
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
            .map_or(1.0, |(min, max)| (max - min).length())
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
        if !self.enabled {}
        // TODO: Implement curve network rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {}
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
            .map(std::convert::AsRef::as_ref)
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

        // Test material getter/setter
        assert_eq!(cn.material(), "default");
        cn.set_material("clay");
        assert_eq!(cn.material(), "clay");
    }

    #[test]
    fn test_curve_network_vector_quantities() {
        use polyscope_core::quantity::QuantityKind;

        let nodes = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
        let edges = vec![[0, 1], [1, 2]];
        let mut cn = CurveNetwork::new("test", nodes, edges);

        cn.add_node_vector_quantity("node_vecs", vec![Vec3::X, Vec3::Y, Vec3::Z]);
        cn.add_edge_vector_quantity("edge_vecs", vec![Vec3::X, Vec3::Y]);

        let nq = cn.get_quantity("node_vecs").expect("node vector not found");
        assert_eq!(nq.data_size(), 3);
        assert_eq!(nq.kind(), QuantityKind::Vector);

        let eq = cn.get_quantity("edge_vecs").expect("edge vector not found");
        assert_eq!(eq.data_size(), 2);
        assert_eq!(eq.kind(), QuantityKind::Vector);
    }
}
