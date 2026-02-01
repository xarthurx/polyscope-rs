//! Point cloud structure.

mod quantities;

use glam::{Mat4, Vec3, Vec4};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::{ColorMapRegistry, PickUniforms, PointCloudRenderData, PointUniforms};
use wgpu::util::DeviceExt;

pub use quantities::*;

/// A point cloud structure.
pub struct PointCloud {
    name: String,
    points: Vec<Vec3>,
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,
    render_data: Option<PointCloudRenderData>,
    material: String,
    point_radius: f32,
    base_color: Vec4,
    // GPU picking resources
    pick_uniform_buffer: Option<wgpu::Buffer>,
    pick_bind_group: Option<wgpu::BindGroup>,
    global_start: u32,
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
            material: "clay".to_string(),
            point_radius: 0.01,
            base_color: Vec4::new(0.2, 0.5, 0.8, 1.0),
            pick_uniform_buffer: None,
            pick_bind_group: None,
            global_start: 0,
        }
    }

    /// Returns the number of points.
    #[must_use]
    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    /// Returns the points.
    #[must_use]
    pub fn points(&self) -> &[Vec3] {
        &self.points
    }

    /// Updates the point positions.
    pub fn update_points(&mut self, points: Vec<Vec3>) {
        self.points = points;
        self.refresh();
    }

    /// Adds a scalar quantity to this point cloud.
    pub fn add_scalar_quantity(&mut self, name: impl Into<String>, values: Vec<f32>) -> &mut Self {
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
    pub fn add_color_quantity(&mut self, name: impl Into<String>, colors: Vec<Vec3>) -> &mut Self {
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
    #[must_use]
    pub fn render_data(&self) -> Option<&PointCloudRenderData> {
        self.render_data.as_ref()
    }

    /// Initializes GPU resources for pick rendering.
    pub fn init_pick_resources(
        &mut self,
        device: &wgpu::Device,
        pick_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        global_start: u32,
    ) {
        self.global_start = global_start;

        // Create pick uniform buffer
        let pick_uniforms = PickUniforms {
            global_start,
            point_radius: self.point_radius,
            _padding: [0.0; 2],
        };
        let pick_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point cloud pick uniforms"),
            contents: bytemuck::cast_slice(&[pick_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create pick bind group (reuses position buffer from render_data)
        if let Some(render_data) = &self.render_data {
            let pick_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("point cloud pick bind group"),
                layout: pick_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: pick_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: render_data.position_buffer.as_entire_binding(),
                    },
                ],
            });
            self.pick_bind_group = Some(pick_bind_group);
        }

        self.pick_uniform_buffer = Some(pick_uniform_buffer);
    }

    /// Returns the pick bind group if initialized.
    #[must_use]
    pub fn pick_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.pick_bind_group.as_ref()
    }

    /// Updates pick uniforms (e.g., when point radius changes).
    pub fn update_pick_uniforms(&self, queue: &wgpu::Queue) {
        if let Some(buffer) = &self.pick_uniform_buffer {
            let pick_uniforms = PickUniforms {
                global_start: self.global_start,
                point_radius: self.point_radius,
                _padding: [0.0; 2],
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[pick_uniforms]));
        }
    }

    /// Sets the point radius.
    pub fn set_point_radius(&mut self, radius: f32) {
        self.point_radius = radius;
    }

    /// Gets the point radius.
    #[must_use]
    pub fn point_radius(&self) -> f32 {
        self.point_radius
    }

    /// Sets the base color.
    pub fn set_base_color(&mut self, color: Vec3) {
        self.base_color = color.extend(1.0);
    }

    /// Gets the base color.
    #[must_use]
    pub fn base_color(&self) -> Vec4 {
        self.base_color
    }

    /// Returns the currently active color quantity, if any.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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

    /// Builds the egui UI for this point cloud.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui, available_materials: &[&str]) {
        let mut color = [self.base_color.x, self.base_color.y, self.base_color.z];
        let mut radius = self.point_radius;

        if polyscope_ui::build_point_cloud_ui(
            ui,
            self.points.len(),
            &mut radius,
            &mut color,
            &mut self.material,
            available_materials,
        ) {
            self.base_color = Vec4::new(color[0], color[1], color[2], self.base_color.w);
            self.point_radius = radius;
        }

        // Show quantities
        if !self.quantities.is_empty() {
            ui.separator();
            ui.label("Quantities:");
            for quantity in &mut self.quantities {
                // Cast to concrete type and call build_egui_ui
                if let Some(sq) = quantity
                    .as_any_mut()
                    .downcast_mut::<PointCloudScalarQuantity>()
                {
                    sq.build_egui_ui(ui);
                } else if let Some(cq) = quantity
                    .as_any_mut()
                    .downcast_mut::<PointCloudColorQuantity>()
                {
                    cq.build_egui_ui(ui);
                } else if let Some(vq) = quantity
                    .as_any_mut()
                    .downcast_mut::<PointCloudVectorQuantity>()
                {
                    vq.build_egui_ui(ui);
                }
            }
        }
    }

    /// Updates GPU buffers based on current state.
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
        let Some(render_data) = &self.render_data else {
            return;
        };

        // Convert glam Mat4 to [[f32; 4]; 4] for GPU
        let model_matrix = self.transform.to_cols_array_2d();

        let mut uniforms = PointUniforms {
            model_matrix,
            point_radius: self.point_radius,
            use_per_point_color: 0,
            _padding: [0.0; 2],
            base_color: self.base_color.to_array(),
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

    fn material(&self) -> &str {
        &self.material
    }

    fn set_material(&mut self, material: &str) {
        self.material = material.to_string();
    }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        // Rendering is handled by polyscope/src/app/render.rs
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        // Pick rendering is handled by polyscope/src/app/render.rs
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is handled by polyscope-ui/src/structure_ui.rs
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // Pick UI is handled by polyscope-ui/src/panels.rs
    }

    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
        self.pick_uniform_buffer = None;
        self.pick_bind_group = None;
        for quantity in &mut self.quantities {
            quantity.clear_gpu_resources();
        }
    }

    fn refresh(&mut self) {
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
