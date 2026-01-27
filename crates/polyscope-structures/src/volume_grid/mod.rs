//! Volume grid structure for visualizing regular 3D grids.

mod scalar_quantity;

pub use scalar_quantity::*;

use glam::{Mat4, UVec3, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::CurveNetworkRenderData;

/// A regular 3D grid structure.
///
/// `VolumeGrid` represents a regular axis-aligned 3D grid defined by:
/// - Grid dimensions (number of nodes in X, Y, Z)
/// - Bounding box (min and max corners in world space)
///
/// Node values are at grid vertices, cell values are at grid cell centers.
#[allow(dead_code)]
pub struct VolumeGrid {
    name: String,

    // Grid parameters
    node_dim: UVec3, // Number of nodes in each dimension
    bound_min: Vec3, // Minimum corner of the grid
    bound_max: Vec3, // Maximum corner of the grid

    // Common structure fields
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Visualization parameters
    color: Vec3,
    edge_color: Vec3,
    edge_width: f32,
    cube_size_factor: f32,

    // GPU resources (bounding box wireframe)
    render_data: Option<CurveNetworkRenderData>,
}

impl VolumeGrid {
    /// Creates a new volume grid.
    ///
    /// # Arguments
    /// * `name` - The name of the grid
    /// * `node_dim` - Number of nodes in each dimension (X, Y, Z)
    /// * `bound_min` - Minimum corner of the grid bounding box
    /// * `bound_max` - Maximum corner of the grid bounding box
    pub fn new(name: impl Into<String>, node_dim: UVec3, bound_min: Vec3, bound_max: Vec3) -> Self {
        Self {
            name: name.into(),
            node_dim,
            bound_min,
            bound_max,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            color: Vec3::new(0.5, 0.5, 0.5),
            edge_color: Vec3::new(0.0, 0.0, 0.0),
            edge_width: 1.0,
            cube_size_factor: 0.0,
            render_data: None,
        }
    }

    /// Creates a volume grid with uniform dimensions.
    pub fn new_uniform(
        name: impl Into<String>,
        dim: u32,
        bound_min: Vec3,
        bound_max: Vec3,
    ) -> Self {
        Self::new(name, UVec3::splat(dim), bound_min, bound_max)
    }

    /// Returns the number of nodes in each dimension.
    #[must_use]
    pub fn node_dim(&self) -> UVec3 {
        self.node_dim
    }

    /// Returns the number of cells in each dimension.
    #[must_use]
    pub fn cell_dim(&self) -> UVec3 {
        self.node_dim.saturating_sub(UVec3::ONE)
    }

    /// Returns the total number of nodes.
    #[must_use]
    pub fn num_nodes(&self) -> u64 {
        u64::from(self.node_dim.x) * u64::from(self.node_dim.y) * u64::from(self.node_dim.z)
    }

    /// Returns the total number of cells.
    #[must_use]
    pub fn num_cells(&self) -> u64 {
        let cell_dim = self.cell_dim();
        u64::from(cell_dim.x) * u64::from(cell_dim.y) * u64::from(cell_dim.z)
    }

    /// Returns the minimum bound.
    #[must_use]
    pub fn bound_min(&self) -> Vec3 {
        self.bound_min
    }

    /// Returns the maximum bound.
    #[must_use]
    pub fn bound_max(&self) -> Vec3 {
        self.bound_max
    }

    /// Returns the grid spacing (distance between adjacent nodes).
    #[must_use]
    pub fn grid_spacing(&self) -> Vec3 {
        let cell_dim = self.cell_dim().as_vec3();
        (self.bound_max - self.bound_min) / cell_dim.max(Vec3::ONE)
    }

    /// Flattens a 3D node index to a linear index.
    #[must_use]
    pub fn flatten_node_index(&self, i: u32, j: u32, k: u32) -> u64 {
        u64::from(i)
            + (u64::from(j) * u64::from(self.node_dim.x))
            + (u64::from(k) * u64::from(self.node_dim.x) * u64::from(self.node_dim.y))
    }

    /// Unflattens a linear node index to a 3D index.
    #[must_use]
    pub fn unflatten_node_index(&self, idx: u64) -> UVec3 {
        let x = idx % u64::from(self.node_dim.x);
        let y = (idx / u64::from(self.node_dim.x)) % u64::from(self.node_dim.y);
        let z = idx / (u64::from(self.node_dim.x) * u64::from(self.node_dim.y));
        UVec3::new(x as u32, y as u32, z as u32)
    }

    /// Returns the world position of a node at the given 3D index.
    #[must_use]
    pub fn position_of_node(&self, i: u32, j: u32, k: u32) -> Vec3 {
        let cell_dim = self.cell_dim().as_vec3().max(Vec3::ONE);
        let t = Vec3::new(i as f32, j as f32, k as f32) / cell_dim;
        self.bound_min + t * (self.bound_max - self.bound_min)
    }

    /// Gets the grid color.
    #[must_use]
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the grid color.
    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        self
    }

    /// Gets the edge color.
    #[must_use]
    pub fn edge_color(&self) -> Vec3 {
        self.edge_color
    }

    /// Sets the edge color.
    pub fn set_edge_color(&mut self, color: Vec3) -> &mut Self {
        self.edge_color = color;
        self
    }

    /// Gets the edge width.
    #[must_use]
    pub fn edge_width(&self) -> f32 {
        self.edge_width
    }

    /// Sets the edge width.
    pub fn set_edge_width(&mut self, width: f32) -> &mut Self {
        self.edge_width = width;
        self
    }

    /// Generates the bounding box wireframe geometry.
    fn generate_bbox_wireframe(&self) -> (Vec<Vec3>, Vec<[u32; 2]>) {
        let min = self.bound_min;
        let max = self.bound_max;

        // 8 corners of the bounding box
        let nodes = vec![
            Vec3::new(min.x, min.y, min.z), // 0
            Vec3::new(max.x, min.y, min.z), // 1
            Vec3::new(max.x, max.y, min.z), // 2
            Vec3::new(min.x, max.y, min.z), // 3
            Vec3::new(min.x, min.y, max.z), // 4
            Vec3::new(max.x, min.y, max.z), // 5
            Vec3::new(max.x, max.y, max.z), // 6
            Vec3::new(min.x, max.y, max.z), // 7
        ];

        // 12 edges of the bounding box
        let edges = vec![
            // Bottom face
            [0, 1],
            [1, 2],
            [2, 3],
            [3, 0],
            // Top face
            [4, 5],
            [5, 6],
            [6, 7],
            [7, 4],
            // Vertical edges
            [0, 4],
            [1, 5],
            [2, 6],
            [3, 7],
        ];

        (nodes, edges)
    }

    /// Initializes GPU render data.
    pub fn init_render_data(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        queue: &wgpu::Queue,
    ) {
        let (nodes, edges) = self.generate_bbox_wireframe();

        let edge_tail_inds: Vec<u32> = edges.iter().map(|e| e[0]).collect();
        let edge_tip_inds: Vec<u32> = edges.iter().map(|e| e[1]).collect();

        let render_data = CurveNetworkRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &nodes,
            &edge_tail_inds,
            &edge_tip_inds,
        );

        // Update uniforms with edge color
        let uniforms = polyscope_render::CurveNetworkUniforms {
            color: [self.edge_color.x, self.edge_color.y, self.edge_color.z, 1.0],
            radius: 0.002,
            radius_is_relative: 1,
            render_mode: 0,
            _padding: 0.0,
        };
        render_data.update_uniforms(queue, &uniforms);

        self.render_data = Some(render_data);
    }

    /// Returns the render data if available.
    #[must_use]
    pub fn render_data(&self) -> Option<&CurveNetworkRenderData> {
        self.render_data.as_ref()
    }

    /// Adds a node scalar quantity to the grid.
    pub fn add_node_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity =
            VolumeGridNodeScalarQuantity::new(name, self.name.clone(), values, self.node_dim);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a cell scalar quantity to the grid.
    pub fn add_cell_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity =
            VolumeGridCellScalarQuantity::new(name, self.name.clone(), values, self.cell_dim());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Builds the egui UI for this volume grid.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        // Grid info
        ui.label(format!(
            "Nodes: {}x{}x{} ({})",
            self.node_dim.x,
            self.node_dim.y,
            self.node_dim.z,
            self.num_nodes()
        ));

        // Color picker
        ui.horizontal(|ui| {
            ui.label("Edge Color:");
            let mut color = [self.edge_color.x, self.edge_color.y, self.edge_color.z];
            if ui.color_edit_button_rgb(&mut color).changed() {
                self.set_edge_color(Vec3::new(color[0], color[1], color[2]));
            }
        });

        // Edge width
        ui.horizontal(|ui| {
            ui.label("Edge Width:");
            let mut width = self.edge_width;
            if ui
                .add(
                    egui::DragValue::new(&mut width)
                        .speed(0.01)
                        .range(0.0..=5.0),
                )
                .changed()
            {
                self.set_edge_width(width);
            }
        });

        // Show quantities
        if !self.quantities.is_empty() {
            ui.separator();
            ui.label("Quantities:");
            for quantity in &mut self.quantities {
                if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<VolumeGridCellScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                }
            }
        }
    }
}

impl Structure for VolumeGrid {
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
        "VolumeGrid"
    }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        Some((self.bound_min, self.bound_max))
    }

    fn length_scale(&self) -> f32 {
        (self.bound_max - self.bound_min).length()
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
        // Drawing is handled externally
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        // Picking not implemented
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is built via build_egui_ui
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // Pick UI not implemented
    }

    fn refresh(&mut self) {
        self.render_data = None;
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for VolumeGrid {
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
    fn test_volume_grid_creation() {
        let grid = VolumeGrid::new("test", UVec3::new(10, 20, 30), Vec3::ZERO, Vec3::ONE);
        assert_eq!(grid.node_dim(), UVec3::new(10, 20, 30));
        assert_eq!(grid.cell_dim(), UVec3::new(9, 19, 29));
        assert_eq!(grid.num_nodes(), 10 * 20 * 30);
        assert_eq!(grid.num_cells(), 9 * 19 * 29);
    }

    #[test]
    fn test_index_conversion() {
        let grid = VolumeGrid::new("test", UVec3::new(5, 6, 7), Vec3::ZERO, Vec3::ONE);

        let idx = grid.flatten_node_index(2, 3, 4);
        let uvec = grid.unflatten_node_index(idx);
        assert_eq!(uvec, UVec3::new(2, 3, 4));
    }

    #[test]
    fn test_node_position() {
        let grid = VolumeGrid::new(
            "test",
            UVec3::new(3, 3, 3),
            Vec3::ZERO,
            Vec3::new(2.0, 2.0, 2.0),
        );

        let p = grid.position_of_node(0, 0, 0);
        assert!((p - Vec3::ZERO).length() < 1e-6);

        let p = grid.position_of_node(2, 2, 2);
        assert!((p - Vec3::new(2.0, 2.0, 2.0)).length() < 1e-6);

        let p = grid.position_of_node(1, 1, 1);
        assert!((p - Vec3::ONE).length() < 1e-6);
    }
}
