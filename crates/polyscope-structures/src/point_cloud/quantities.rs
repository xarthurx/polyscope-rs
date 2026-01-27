//! Point cloud quantity implementations.

use glam::Vec3;
use polyscope_core::quantity::{Quantity, QuantityKind, VertexQuantity};
use polyscope_render::{ColorMap, PointCloudRenderData, VectorRenderData, VectorUniforms};

/// A scalar quantity on a point cloud.
pub struct PointCloudScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl PointCloudScalarQuantity {
    /// Creates a new scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            colormap_name: "viridis".to_string(),
            range_min: min,
            range_max: max,
        }
    }

    /// Returns the scalar values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Maps scalar values to colors using the colormap.
    #[must_use]
    pub fn compute_colors(&self, colormap: &ColorMap) -> Vec<Vec3> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        self.values
            .iter()
            .map(|&v| {
                let t = (v - self.range_min) / range;
                colormap.sample(t)
            })
            .collect()
    }

    /// Gets the colormap name.
    #[must_use]
    pub fn colormap_name(&self) -> &str {
        &self.colormap_name
    }

    /// Sets the colormap name.
    pub fn set_colormap(&mut self, name: impl Into<String>) {
        self.colormap_name = name.into();
    }

    /// Gets the range minimum.
    #[must_use]
    pub fn range_min(&self) -> f32 {
        self.range_min
    }

    /// Gets the range maximum.
    #[must_use]
    pub fn range_max(&self) -> f32 {
        self.range_max
    }

    /// Sets the range.
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.range_min = min;
        self.range_max = max;
    }

    /// Builds the egui UI for this scalar quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let colormaps = ["viridis", "blues", "reds", "coolwarm", "rainbow"];
        polyscope_ui::build_scalar_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.colormap_name,
            &mut self.range_min,
            &mut self.range_max,
            &colormaps,
        )
    }
}

impl Quantity for PointCloudScalarQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.structure_name
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Scalar
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl VertexQuantity for PointCloudScalarQuantity {}

/// A vector quantity on a point cloud.
pub struct PointCloudVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
    render_data: Option<VectorRenderData>,
}

impl PointCloudVectorQuantity {
    /// Creates a new vector quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            enabled: false,
            length_scale: 1.0,
            radius: 0.005,
            color: Vec3::new(0.8, 0.2, 0.2), // Red
            render_data: None,
        }
    }

    /// Returns the vectors.
    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    /// Initializes GPU resources for this vector quantity.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        base_positions: &[Vec3],
    ) {
        self.render_data = Some(VectorRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            base_positions,
            &self.vectors,
        ));
    }

    /// Returns the render data if initialized.
    #[must_use]
    pub fn render_data(&self) -> Option<&VectorRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU uniforms with the given model transform.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, model: &glam::Mat4) {
        if let Some(render_data) = &self.render_data {
            let uniforms = VectorUniforms {
                model: model.to_cols_array(),
                length_scale: self.length_scale,
                radius: self.radius,
                _padding: [0.0; 2],
                color: [self.color.x, self.color.y, self.color.z, 1.0],
            };
            render_data.update_uniforms(queue, &uniforms);
        }
    }

    /// Sets the length scale.
    pub fn set_length_scale(&mut self, scale: f32) {
        self.length_scale = scale;
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    /// Sets the color.
    pub fn set_color(&mut self, color: Vec3) {
        self.color = color;
    }

    /// Gets the length scale.
    #[must_use]
    pub fn length_scale(&self) -> f32 {
        self.length_scale
    }

    /// Gets the radius.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Gets the color.
    #[must_use]
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Builds the egui UI for this vector quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut color = [self.color.x, self.color.y, self.color.z];
        let changed = polyscope_ui::build_vector_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.length_scale,
            &mut self.radius,
            &mut color,
        );
        if changed {
            self.color = Vec3::new(color[0], color[1], color[2]);
        }
        changed
    }
}

impl Quantity for PointCloudVectorQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.structure_name
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Vector
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl VertexQuantity for PointCloudVectorQuantity {}

/// A color quantity on a point cloud.
pub struct PointCloudColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl PointCloudColorQuantity {
    /// Creates a new color quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
        }
    }

    /// Returns the colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    /// Applies this color quantity to the point cloud render data.
    pub fn apply_to_render_data(&self, queue: &wgpu::Queue, render_data: &PointCloudRenderData) {
        render_data.update_colors(queue, &self.colors);
    }

    /// Builds the egui UI for this color quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for PointCloudColorQuantity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn structure_name(&self) -> &str {
        &self.structure_name
    }

    fn kind(&self) -> QuantityKind {
        QuantityKind::Color
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // TODO: Implement UI
    }

    fn refresh(&mut self) {
        // TODO: Refresh GPU resources
    }

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl VertexQuantity for PointCloudColorQuantity {}
