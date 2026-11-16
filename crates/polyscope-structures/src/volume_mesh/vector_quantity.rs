//! Vector quantities for volume meshes.

use glam::{Mat4, Vec3, Vec4};
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};
use polyscope_render::{VectorRenderData, VectorUniforms};

use super::VolumeMesh;

/// A vector quantity defined at mesh vertices.
pub struct VolumeMeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec4,
    render_data: Option<VectorRenderData>,
}

impl VolumeMeshVertexVectorQuantity {
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
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec4::new(0.1, 0.1, 0.8, 1.0),
            render_data: None,
        }
    }

    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color.extend(1.0);
        self
    }

    /// Initializes GPU resources for this vector quantity.
    ///
    /// `bases` are the arrow start points in object space.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        bases: &[Vec3],
    ) {
        self.render_data = Some(VectorRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            bases,
            &self.vectors,
        ));
    }

    /// Returns the render data if initialized.
    #[must_use]
    pub fn render_data(&self) -> Option<&VectorRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU uniforms with the given model transform.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, model: &Mat4) {
        if let Some(render_data) = &self.render_data {
            render_data.update_uniforms(
                queue,
                &VectorUniforms {
                    model: model.to_cols_array(),
                    length_scale: self.vector_length_scale,
                    radius: self.vector_radius,
                    _padding: [0.0; 2],
                    color: self.vector_color.to_array(),
                },
            );
        }
    }

    /// Builds the egui UI for this quantity: enable checkbox, length, radius and color.
    ///
    /// Returns true if a setting changed.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        build_vector_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.vector_length_scale,
            &mut self.vector_radius,
            &mut self.vector_color,
        )
    }
}

impl Quantity for VolumeMeshVertexVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {
        // The arrow bases may have moved, so rebuild the GPU data on the next frame.
        self.render_data = None;
    }
    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl VertexQuantity for VolumeMeshVertexVectorQuantity {}

/// A vector quantity defined at mesh cells.
pub struct VolumeMeshCellVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec4,
    render_data: Option<VectorRenderData>,
}

impl VolumeMeshCellVectorQuantity {
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
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec4::new(0.1, 0.1, 0.8, 1.0),
            render_data: None,
        }
    }

    #[must_use]
    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color.extend(1.0);
        self
    }

    /// Initializes GPU resources for this vector quantity.
    ///
    /// `bases` are the arrow start points in object space.
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        bases: &[Vec3],
    ) {
        self.render_data = Some(VectorRenderData::new(
            device,
            bind_group_layout,
            camera_buffer,
            bases,
            &self.vectors,
        ));
    }

    /// Returns the render data if initialized.
    #[must_use]
    pub fn render_data(&self) -> Option<&VectorRenderData> {
        self.render_data.as_ref()
    }

    /// Updates GPU uniforms with the given model transform.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, model: &Mat4) {
        if let Some(render_data) = &self.render_data {
            render_data.update_uniforms(
                queue,
                &VectorUniforms {
                    model: model.to_cols_array(),
                    length_scale: self.vector_length_scale,
                    radius: self.vector_radius,
                    _padding: [0.0; 2],
                    color: self.vector_color.to_array(),
                },
            );
        }
    }

    /// Builds the egui UI for this quantity: enable checkbox, length, radius and color.
    ///
    /// Returns true if a setting changed.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        build_vector_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.vector_length_scale,
            &mut self.vector_radius,
            &mut self.vector_color,
        )
    }
}

impl Quantity for VolumeMeshCellVectorQuantity {
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
    fn data_size(&self) -> usize {
        self.vectors.len()
    }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {
        // The arrow bases may have moved, so rebuild the GPU data on the next frame.
        self.render_data = None;
    }
    fn clear_gpu_resources(&mut self) {
        self.render_data = None;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl CellQuantity for VolumeMeshCellVectorQuantity {}

/// Builds the vector quantity UI and writes the edited color back.
fn build_vector_ui(
    ui: &mut egui::Ui,
    name: &str,
    enabled: &mut bool,
    length_scale: &mut f32,
    radius: &mut f32,
    color: &mut Vec4,
) -> bool {
    let mut rgb = [color.x, color.y, color.z];
    let changed =
        polyscope_ui::build_vector_quantity_ui(ui, name, enabled, length_scale, radius, &mut rgb);
    if changed {
        *color = Vec4::new(rgb[0], rgb[1], rgb[2], color.w);
    }
    changed
}

impl VolumeMesh {
    /// Returns the currently active vertex vector quantity, if any.
    #[must_use]
    pub fn active_vertex_vector_quantity(&self) -> Option<&VolumeMeshVertexVectorQuantity> {
        self.quantities
            .iter()
            .filter(|q| q.is_enabled())
            .find_map(|q| q.as_any().downcast_ref::<VolumeMeshVertexVectorQuantity>())
    }

    /// Returns the currently active vertex vector quantity (mutable), if any.
    pub fn active_vertex_vector_quantity_mut(
        &mut self,
    ) -> Option<&mut VolumeMeshVertexVectorQuantity> {
        self.quantities
            .iter_mut()
            .filter(|q| q.is_enabled())
            .find_map(|q| {
                q.as_any_mut()
                    .downcast_mut::<VolumeMeshVertexVectorQuantity>()
            })
    }

    /// Returns the currently active cell vector quantity, if any.
    #[must_use]
    pub fn active_cell_vector_quantity(&self) -> Option<&VolumeMeshCellVectorQuantity> {
        self.quantities
            .iter()
            .filter(|q| q.is_enabled())
            .find_map(|q| q.as_any().downcast_ref::<VolumeMeshCellVectorQuantity>())
    }

    /// Returns the currently active cell vector quantity (mutable), if any.
    pub fn active_cell_vector_quantity_mut(&mut self) -> Option<&mut VolumeMeshCellVectorQuantity> {
        self.quantities
            .iter_mut()
            .filter(|q| q.is_enabled())
            .find_map(|q| {
                q.as_any_mut()
                    .downcast_mut::<VolumeMeshCellVectorQuantity>()
            })
    }

    /// Returns the center of each cell in object space.
    ///
    /// A cell center is the mean of the valid (non-sentinel) vertices of the cell.
    #[must_use]
    pub fn cell_centers(&self) -> Vec<Vec3> {
        self.cells
            .iter()
            .map(|cell| self.cell_centroid(cell))
            .collect()
    }

    /// Initializes GPU resources for the enabled vertex and cell vector quantities.
    ///
    /// Vertex vectors start at the vertices. Cell vectors start at the cell centers.
    pub fn init_vector_quantity_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        if self
            .active_vertex_vector_quantity()
            .is_some_and(|vq| vq.render_data().is_none())
        {
            let vertices = self.vertices.clone();
            if let Some(vq) = self.active_vertex_vector_quantity_mut() {
                vq.init_gpu_resources(device, bind_group_layout, camera_buffer, &vertices);
            }
        }

        if self
            .active_cell_vector_quantity()
            .is_some_and(|vq| vq.render_data().is_none())
        {
            let centers = self.cell_centers();
            if let Some(vq) = self.active_cell_vector_quantity_mut() {
                vq.init_gpu_resources(device, bind_group_layout, camera_buffer, &centers);
            }
        }
    }

    /// Updates the model uniforms of the enabled vertex and cell vector quantities.
    pub fn update_vector_quantity_uniforms(&self, queue: &wgpu::Queue) {
        if let Some(vq) = self.active_vertex_vector_quantity() {
            vq.update_uniforms(queue, &self.transform);
        }
        if let Some(vq) = self.active_cell_vector_quantity() {
            vq.update_uniforms(queue, &self.transform);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_centers_ignore_sentinel_slots() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
            Vec3::new(0.0, 0.0, 4.0),
        ];
        let mesh = VolumeMesh::new_tet_mesh("tet", vertices, vec![[0, 1, 2, 3]]);
        let centers = mesh.cell_centers();
        assert_eq!(centers.len(), 1);
        assert!((centers[0] - Vec3::splat(1.0)).length() < 1e-6);
    }

    #[test]
    fn test_active_vector_quantities() {
        use polyscope_core::structure::HasQuantities;

        let vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let mut mesh = VolumeMesh::new_tet_mesh("tet", vertices, vec![[0, 1, 2, 3]]);
        mesh.add_vertex_vector_quantity("vv", vec![Vec3::X; 4]);
        mesh.add_cell_vector_quantity("cv", vec![Vec3::Y]);
        assert!(mesh.active_vertex_vector_quantity().is_none());
        assert!(mesh.active_cell_vector_quantity().is_none());

        mesh.get_quantity_mut("vv").unwrap().set_enabled(true);
        mesh.get_quantity_mut("cv").unwrap().set_enabled(true);
        assert_eq!(mesh.active_vertex_vector_quantity().unwrap().name(), "vv");
        assert_eq!(mesh.active_cell_vector_quantity().unwrap().name(), "cv");
    }
}
