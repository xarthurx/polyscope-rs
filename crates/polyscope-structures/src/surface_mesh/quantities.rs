//! Surface mesh quantity implementations.

use glam::{Vec3, Vec4};
use polyscope_core::quantity::{FaceQuantity, Quantity, QuantityKind, VertexQuantity};
use polyscope_render::{ColorMap, VectorRenderData, VectorUniforms};

/// A vertex scalar quantity on a surface mesh.
pub struct MeshVertexScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl MeshVertexScalarQuantity {
    /// Creates a new vertex scalar quantity.
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

    /// Maps scalar values to colors using the colormap.
    #[must_use]
    pub fn compute_colors(&self, colormap: &ColorMap) -> Vec<Vec4> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        self.values
            .iter()
            .map(|&v| {
                let t = (v - self.range_min) / range;
                colormap.sample(t).extend(1.0)
            })
            .collect()
    }

    /// Builds the egui UI for this quantity.
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

impl Quantity for MeshVertexScalarQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl VertexQuantity for MeshVertexScalarQuantity {}

/// A face scalar quantity on a surface mesh.
pub struct MeshFaceScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    colormap_name: String,
    range_min: f32,
    range_max: f32,
}

impl MeshFaceScalarQuantity {
    /// Creates a new face scalar quantity.
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

    /// Computes vertex colors by expanding face values to all vertices of each face.
    /// For each vertex, uses the color of the last face it belongs to.
    #[must_use]
    pub fn compute_vertex_colors(
        &self,
        faces: &[Vec<u32>],
        num_vertices: usize,
        colormap: &ColorMap,
    ) -> Vec<Vec4> {
        let range = self.range_max - self.range_min;
        let range = if range.abs() < 1e-10 { 1.0 } else { range };

        let mut colors = vec![Vec4::splat(0.5); num_vertices];

        for (face_idx, face) in faces.iter().enumerate() {
            let t = (self.values[face_idx] - self.range_min) / range;
            let color = colormap.sample(t).extend(1.0);
            for &vi in face {
                colors[vi as usize] = color;
            }
        }

        colors
    }

    /// Builds the egui UI for this quantity.
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

impl Quantity for MeshFaceScalarQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn data_size(&self) -> usize {
        self.values.len()
    }
}

impl FaceQuantity for MeshFaceScalarQuantity {}

/// A vertex color quantity on a surface mesh.
pub struct MeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec4>,
    enabled: bool,
    has_transparency: bool,
}

impl MeshVertexColorQuantity {
    /// Creates a new vertex color quantity (RGB, alpha defaults to 1.0).
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors: colors.into_iter().map(|c| c.extend(1.0)).collect(),
            enabled: false,
            has_transparency: false,
        }
    }

    /// Creates a new vertex color quantity with explicit RGBA alpha values.
    pub fn new_with_alpha(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> Self {
        let has_transparency = colors.iter().any(|c| c.w < 0.999);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
            has_transparency,
        }
    }

    /// Returns the colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec4] {
        &self.colors
    }

    /// Returns true if any color has alpha < 1.0.
    #[must_use]
    pub fn has_transparency(&self) -> bool {
        self.has_transparency
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for MeshVertexColorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl VertexQuantity for MeshVertexColorQuantity {}

/// A face color quantity on a surface mesh.
pub struct MeshFaceColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec4>,
    enabled: bool,
    has_transparency: bool,
}

impl MeshFaceColorQuantity {
    /// Creates a new face color quantity (RGB, alpha defaults to 1.0).
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors: colors.into_iter().map(|c| c.extend(1.0)).collect(),
            enabled: false,
            has_transparency: false,
        }
    }

    /// Creates a new face color quantity with explicit RGBA alpha values.
    pub fn new_with_alpha(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> Self {
        let has_transparency = colors.iter().any(|c| c.w < 0.999);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
            has_transparency,
        }
    }

    /// Returns the colors.
    #[must_use]
    pub fn colors(&self) -> &[Vec4] {
        &self.colors
    }

    /// Returns true if any color has alpha < 1.0.
    #[must_use]
    pub fn has_transparency(&self) -> bool {
        self.has_transparency
    }

    /// Computes vertex colors by expanding face colors to all vertices of each face.
    /// For each vertex, uses the color of the last face it belongs to.
    #[must_use]
    pub fn compute_vertex_colors(&self, faces: &[Vec<u32>], num_vertices: usize) -> Vec<Vec4> {
        let mut colors = vec![Vec4::splat(0.5); num_vertices];

        for (face_idx, face) in faces.iter().enumerate() {
            let color = self.colors[face_idx];
            for &vi in face {
                colors[vi as usize] = color;
            }
        }

        colors
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_color_quantity_ui(ui, &self.name, &mut self.enabled, self.colors.len())
    }
}

impl Quantity for MeshFaceColorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn data_size(&self) -> usize {
        self.colors.len()
    }
}

impl FaceQuantity for MeshFaceColorQuantity {}

/// A vertex vector quantity on a surface mesh.
pub struct MeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec4,
    render_data: Option<VectorRenderData>,
}

impl MeshVertexVectorQuantity {
    /// Creates a new vertex vector quantity.
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
            color: Vec4::new(0.8, 0.2, 0.2, 1.0),
            render_data: None,
        }
    }

    /// Returns the vectors.
    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    /// Gets the length scale.
    #[must_use]
    pub fn length_scale(&self) -> f32 {
        self.length_scale
    }

    /// Sets the length scale.
    pub fn set_length_scale(&mut self, scale: f32) {
        self.length_scale = scale;
    }

    /// Gets the radius.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, r: f32) {
        self.radius = r;
    }

    /// Gets the color.
    #[must_use]
    pub fn color(&self) -> Vec4 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) {
        self.color = c.extend(1.0);
    }

    /// Auto-scales length and radius based on the structure's bounding box diagonal
    /// and the average vector magnitude, so arrows are proportionally visible.
    ///
    /// Target: effective arrow length ≈ 2% of bbox diagonal, radius = 1/10 of length.
    pub fn auto_scale(&mut self, structure_length_scale: f32) {
        let avg_length: f32 = if self.vectors.is_empty() {
            1.0
        } else {
            let sum: f32 = self.vectors.iter().map(|v| v.length()).sum();
            sum / self.vectors.len() as f32
        };
        if avg_length > 1e-8 {
            // Effective arrow length ≈ 2% of bbox diagonal
            self.length_scale = 0.02 * structure_length_scale / avg_length;
        }
        // Radius = 1/10 of effective arrow length ≈ 0.2% of bbox diagonal
        self.radius = 0.002 * structure_length_scale;
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
                color: self.color.to_array(),
            };
            render_data.update_uniforms(queue, &uniforms);
        }
    }

    /// Builds the egui UI for this quantity.
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
            self.color = Vec4::new(color[0], color[1], color[2], self.color.w);
        }
        changed
    }
}

impl Quantity for MeshVertexVectorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
    }

    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl VertexQuantity for MeshVertexVectorQuantity {}

/// A face vector quantity on a surface mesh.
pub struct MeshFaceVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec4,
    render_data: Option<VectorRenderData>,
}

impl MeshFaceVectorQuantity {
    /// Creates a new face vector quantity.
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
            color: Vec4::new(0.2, 0.2, 0.8, 1.0),
            render_data: None,
        }
    }

    /// Returns the vectors.
    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    /// Gets the length scale.
    #[must_use]
    pub fn length_scale(&self) -> f32 {
        self.length_scale
    }

    /// Sets the length scale.
    pub fn set_length_scale(&mut self, scale: f32) {
        self.length_scale = scale;
    }

    /// Gets the radius.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets the radius.
    pub fn set_radius(&mut self, r: f32) {
        self.radius = r;
    }

    /// Gets the color.
    #[must_use]
    pub fn color(&self) -> Vec4 {
        self.color
    }

    /// Sets the color.
    pub fn set_color(&mut self, c: Vec3) {
        self.color = c.extend(1.0);
    }

    /// Auto-scales length and radius based on the structure's bounding box diagonal
    /// and the average vector magnitude, so arrows are proportionally visible.
    ///
    /// Target: effective arrow length ≈ 2% of bbox diagonal, radius = 1/10 of length.
    pub fn auto_scale(&mut self, structure_length_scale: f32) {
        let avg_length: f32 = if self.vectors.is_empty() {
            1.0
        } else {
            let sum: f32 = self.vectors.iter().map(|v| v.length()).sum();
            sum / self.vectors.len() as f32
        };
        if avg_length > 1e-8 {
            // Effective arrow length ≈ 2% of bbox diagonal
            self.length_scale = 0.02 * structure_length_scale / avg_length;
        }
        // Radius = 1/10 of effective arrow length ≈ 0.2% of bbox diagonal
        self.radius = 0.002 * structure_length_scale;
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
                color: self.color.to_array(),
            };
            render_data.update_uniforms(queue, &uniforms);
        }
    }

    /// Builds the egui UI for this quantity.
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
            self.color = Vec4::new(color[0], color[1], color[2], self.color.w);
        }
        changed
    }
}

impl Quantity for MeshFaceVectorQuantity {
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

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}

    fn refresh(&mut self) {}

    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
    }

    fn data_size(&self) -> usize {
        self.vectors.len()
    }
}

impl FaceQuantity for MeshFaceVectorQuantity {}
