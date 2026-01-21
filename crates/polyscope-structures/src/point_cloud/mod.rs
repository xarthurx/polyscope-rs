//! Point cloud structure.

mod quantities;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{ColorMapRegistry, PointCloudRenderData, PointUniforms};

pub use quantities::*;

/// A point cloud structure.
pub struct PointCloud {
    name: String,
    points: Vec<Vec3>,
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,
    render_data: Option<PointCloudRenderData>,
    point_radius: f32,
    base_color: Vec3,
}

impl PointCloud {
    /// Creates a new point cloud.
    pub fn new(name: impl Into<String>, points: Vec<Vec3>) -> Self {
        Self {
            name: name.into(),
            points,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            render_data: None,
            point_radius: 0.01,
            base_color: Vec3::new(0.2, 0.5, 0.8),
        }
    }

    /// Returns the number of points.
    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    /// Returns the points.
    pub fn points(&self) -> &[Vec3] {
        &self.points
    }

    /// Updates the point positions.
    pub fn update_points(&mut self, points: Vec<Vec3>) {
        self.points = points;
        self.refresh();
    }

    /// Adds a scalar quantity to this point cloud.
    pub fn add_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = PointCloudScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vector quantity to this point cloud.
    pub fn add_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = PointCloudVectorQuantity::new(name, self.name.clone(), vectors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a color quantity to this point cloud.
    pub fn add_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = PointCloudColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Initializes GPU resources for this point cloud.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        self.render_data = Some(PointCloudRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            &self.points,
            None, // No per-point colors yet
        ));
    }

    /// Returns the render data if initialized.
    pub fn render_data(&self) -> Option<&PointCloudRenderData> {
        self.render_data.as_ref()
    }

    /// Sets the point radius.
    pub fn set_point_radius(&mut self, radius: f32) {
        self.point_radius = radius;
    }

    /// Gets the point radius.
    pub fn point_radius(&self) -> f32 {
        self.point_radius
    }

    /// Sets the base color.
    pub fn set_base_color(&mut self, color: Vec3) {
        self.base_color = color;
    }

    /// Gets the base color.
    pub fn base_color(&self) -> Vec3 {
        self.base_color
    }

    /// Returns the currently active color quantity, if any.
    pub fn active_color_quantity(&self) -> Option<&PointCloudColorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<PointCloudColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active scalar quantity, if any.
    pub fn active_scalar_quantity(&self) -> Option<&PointCloudScalarQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<PointCloudScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active vector quantity, if any.
    pub fn active_vector_quantity(&self) -> Option<&PointCloudVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any().downcast_ref::<PointCloudVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns a mutable reference to the active vector quantity.
    pub fn active_vector_quantity_mut(&mut self) -> Option<&mut PointCloudVectorQuantity> {
        use polyscope_core::quantity::QuantityKind;

        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any_mut().downcast_mut::<PointCloudVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Updates GPU buffers based on current state.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
        let Some(render_data) = &self.render_data else {
            return;
        };

        let mut uniforms = PointUniforms {
            point_radius: self.point_radius,
            use_per_point_color: 0,
            _padding: [0.0; 2],
            base_color: [self.base_color.x, self.base_color.y, self.base_color.z, 1.0],
        };

        // Priority: color quantity > scalar quantity > base color
        if let Some(color_q) = self.active_color_quantity() {
            uniforms.use_per_point_color = 1;
            color_q.apply_to_render_data(queue, render_data);
        } else if let Some(scalar_q) = self.active_scalar_quantity() {
            if let Some(colormap) = color_maps.get(scalar_q.colormap_name()) {
                uniforms.use_per_point_color = 1;
                let colors = scalar_q.compute_colors(colormap);
                render_data.update_colors(queue, &colors);
            }
        }

        render_data.update_uniforms(queue, &uniforms);
    }
}

impl Structure for PointCloud {
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
        "PointCloud"
    }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        if self.points.is_empty() {
            return None;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for &p in &self.points {
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
        // TODO: Implement point cloud rendering
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        if !self.enabled {
            return;
        }
        // TODO: Implement point cloud picking
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // TODO: Implement pick UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU buffers
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for PointCloud {
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
        self.quantities
            .iter_mut()
            .find(|q| q.name() == name)
    }

    fn remove_quantity(&mut self, name: &str) -> Option<Box<dyn Quantity>> {
        let idx = self.quantities.iter().position(|q| q.name() == name)?;
        Some(self.quantities.remove(idx))
    }

    fn quantities(&self) -> &[Box<dyn Quantity>] {
        &self.quantities
    }
}
