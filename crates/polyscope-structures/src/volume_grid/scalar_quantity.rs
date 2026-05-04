//! Scalar quantities for volume grids.

use glam::{UVec3, Vec3};
use polyscope_core::quantity::{Quantity, QuantityKind};
use polyscope_core::{McmMesh, marching_cubes};
use polyscope_render::{GridcubePickUniforms, GridcubeRenderData, IsosurfaceRenderData};
use wgpu::util::DeviceExt;

/// Visualization mode for volume grid scalar quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeGridVizMode {
    /// Colored cubes at each grid node/cell.
    Gridcube,
    /// Isosurface extracted via marching cubes (node scalars only).
    Isosurface,
}

/// A scalar quantity defined at grid nodes.
pub struct VolumeGridNodeScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    node_dim: UVec3,
    enabled: bool,

    // Visualization parameters
    color_map: String,
    data_min: f32,
    data_max: f32,

    // Visualization mode
    viz_mode: VolumeGridVizMode,

    // Gridcube state
    gridcube_render_data: Option<GridcubeRenderData>,
    gridcube_dirty: bool,

    // Isosurface state
    isosurface_level: f32,
    isosurface_color: Vec3,
    isosurface_render_data: Option<IsosurfaceRenderData>,
    isosurface_mesh_cache: Option<McmMesh>,
    isosurface_dirty: bool,

    // Grid geometry (needed for MC coordinate transform)
    bound_min: Vec3,
    bound_max: Vec3,

    // Flag: user clicked "Register as Surface Mesh"
    register_as_mesh_requested: bool,

    // Pick state
    pick_uniform_buffer: Option<wgpu::Buffer>,
    pick_bind_group: Option<wgpu::BindGroup>,
    global_start: u32,
}

impl VolumeGridNodeScalarQuantity {
    /// Creates a new node scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
        node_dim: UVec3,
        bound_min: Vec3,
        bound_max: Vec3,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        let isosurface_level = (data_min + data_max) * 0.5;
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            node_dim,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
            viz_mode: VolumeGridVizMode::Gridcube,
            gridcube_render_data: None,
            gridcube_dirty: true,
            isosurface_level,
            isosurface_color: Vec3::new(0.047, 0.451, 0.690), // default blue
            isosurface_render_data: None,
            isosurface_mesh_cache: None,
            isosurface_dirty: true,
            bound_min,
            bound_max,
            register_as_mesh_requested: false,
            pick_uniform_buffer: None,
            pick_bind_group: None,
            global_start: 0,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Returns the grid node dimensions.
    #[must_use]
    pub fn node_dim(&self) -> UVec3 {
        self.node_dim
    }

    /// Gets the value at a 3D index.
    #[must_use]
    pub fn get(&self, i: u32, j: u32, k: u32) -> f32 {
        let idx = i as usize
            + j as usize * self.node_dim.x as usize
            + k as usize * self.node_dim.x as usize * self.node_dim.y as usize;
        self.values.get(idx).copied().unwrap_or(0.0)
    }

    /// Gets the color map name.
    #[must_use]
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self.gridcube_dirty = true;
        self
    }

    /// Gets the data range.
    #[must_use]
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    // --- Visualization mode ---

    /// Gets the current visualization mode.
    #[must_use]
    pub fn viz_mode(&self) -> VolumeGridVizMode {
        self.viz_mode
    }

    /// Sets the visualization mode.
    pub fn set_viz_mode(&mut self, mode: VolumeGridVizMode) -> &mut Self {
        self.viz_mode = mode;
        self
    }

    // --- Isosurface ---

    /// Gets the isosurface level.
    #[must_use]
    pub fn isosurface_level(&self) -> f32 {
        self.isosurface_level
    }

    /// Sets the isosurface level (invalidates cache).
    pub fn set_isosurface_level(&mut self, level: f32) -> &mut Self {
        self.isosurface_level = level;
        self.isosurface_dirty = true;
        // Keep old mesh cache and render data — replaced atomically in init phase
        self
    }

    /// Gets the isosurface color.
    #[must_use]
    pub fn isosurface_color(&self) -> Vec3 {
        self.isosurface_color
    }

    /// Sets the isosurface color.
    pub fn set_isosurface_color(&mut self, color: Vec3) -> &mut Self {
        self.isosurface_color = color;
        self
    }

    /// Returns whether the isosurface needs re-extraction.
    #[must_use]
    pub fn isosurface_dirty(&self) -> bool {
        self.isosurface_dirty
    }

    /// Returns whether the gridcube needs GPU re-init.
    #[must_use]
    pub fn gridcube_dirty(&self) -> bool {
        self.gridcube_dirty
    }

    /// Extracts the isosurface mesh using marching cubes.
    ///
    /// `VolumeGrid` stores values as `idx = i + j*nx + k*nx*ny` (z-slowest, x-fastest),
    /// but `marching_cubes` indexes as `(i_mc*ny + j_mc)*nz + k_mc` (x-slowest, z-fastest).
    /// We pass dims in `(nz, ny, nx)` order so MC's inner loop steps over our X axis,
    /// then map output vertices/normals back to world space:
    ///   `world_x = k_mc, world_y = j_mc, world_z = i_mc`
    /// (i.e. swap the X and Z components of every position and normal).
    ///
    /// That swap has determinant -1, which flips triangle handedness. We restore
    /// CCW winding by swapping two indices in every triangle — required for the
    /// `front_facing` test in `simple_mesh.wgsl` and for `register_isosurface_as_mesh`
    /// which recomputes per-face normals from winding.
    pub fn extract_isosurface(&mut self) -> &McmMesh {
        if self.isosurface_mesh_cache.is_none() || self.isosurface_dirty {
            let nx = self.node_dim.x;
            let ny = self.node_dim.y;
            let nz = self.node_dim.z;

            let mut mesh = marching_cubes(&self.values, self.isosurface_level, nz, ny, nx);

            let cell_dim = Vec3::new(
                (nx - 1).max(1) as f32,
                (ny - 1).max(1) as f32,
                (nz - 1).max(1) as f32,
            );
            let spacing = (self.bound_max - self.bound_min) / cell_dim;

            for v in &mut mesh.vertices {
                *v = Vec3::new(
                    v.z * spacing.x + self.bound_min.x,
                    v.y * spacing.y + self.bound_min.y,
                    v.x * spacing.z + self.bound_min.z,
                );
            }

            for n in &mut mesh.normals {
                *n = Vec3::new(n.z / spacing.x, n.y / spacing.y, n.x / spacing.z);
                let len = n.length();
                if len > 0.0 {
                    *n /= len;
                }
            }

            for tri in mesh.indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }

            self.isosurface_mesh_cache = Some(mesh);
            self.isosurface_dirty = false;
        }
        self.isosurface_mesh_cache.as_ref().unwrap()
    }

    /// Returns the cached isosurface mesh, if available.
    #[must_use]
    pub fn isosurface_mesh(&self) -> Option<&McmMesh> {
        self.isosurface_mesh_cache.as_ref()
    }

    // --- GPU resources ---

    /// Returns the gridcube render data.
    #[must_use]
    pub fn gridcube_render_data(&self) -> Option<&GridcubeRenderData> {
        self.gridcube_render_data.as_ref()
    }

    /// Returns a mutable reference to the gridcube render data.
    pub fn gridcube_render_data_mut(&mut self) -> Option<&mut GridcubeRenderData> {
        self.gridcube_render_data.as_mut()
    }

    /// Sets the gridcube render data.
    pub fn set_gridcube_render_data(&mut self, data: GridcubeRenderData) {
        self.gridcube_render_data = Some(data);
        self.gridcube_dirty = false;
    }

    /// Returns the isosurface render data.
    #[must_use]
    pub fn isosurface_render_data(&self) -> Option<&IsosurfaceRenderData> {
        self.isosurface_render_data.as_ref()
    }

    /// Returns a mutable reference to the isosurface render data.
    pub fn isosurface_render_data_mut(&mut self) -> Option<&mut IsosurfaceRenderData> {
        self.isosurface_render_data.as_mut()
    }

    /// Sets the isosurface render data.
    pub fn set_isosurface_render_data(&mut self, data: IsosurfaceRenderData) {
        self.isosurface_render_data = Some(data);
        self.isosurface_dirty = false;
    }

    /// Clears the isosurface render data (e.g. when isovalue yields empty mesh).
    pub fn clear_isosurface_render_data(&mut self) {
        self.isosurface_render_data = None;
        self.isosurface_dirty = false;
    }

    // --- Pick resources ---

    /// Initializes pick resources for this quantity.
    ///
    /// Requires that `gridcube_render_data` is already initialized (needs the position buffer).
    pub fn init_pick_resources(
        &mut self,
        device: &wgpu::Device,
        pick_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        global_start: u32,
    ) {
        self.global_start = global_start;

        let Some(gridcube_rd) = &self.gridcube_render_data else {
            return;
        };

        let uniforms = GridcubePickUniforms {
            global_start,
            cube_size_factor: 1.0,
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube node pick uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gridcube node pick bind group"),
            layout: pick_bind_group_layout,
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
                    resource: gridcube_rd.position_buffer.as_entire_binding(),
                },
            ],
        });

        self.pick_uniform_buffer = Some(uniform_buffer);
        self.pick_bind_group = Some(bind_group);
    }

    /// Returns the pick bind group, if initialized.
    #[must_use]
    pub fn pick_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.pick_bind_group.as_ref()
    }

    /// Updates the pick uniform buffer with current model transform and cube size factor.
    pub fn update_pick_uniforms(
        &self,
        queue: &wgpu::Queue,
        model: [[f32; 4]; 4],
        cube_size_factor: f32,
    ) {
        if let Some(buffer) = &self.pick_uniform_buffer {
            let uniforms = GridcubePickUniforms {
                model,
                global_start: self.global_start,
                cube_size_factor,
                ..Default::default()
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Returns the number of pick elements (= number of gridcube instances).
    #[must_use]
    pub fn num_pick_elements(&self) -> u32 {
        self.gridcube_render_data
            .as_ref()
            .map_or(0, |rd| rd.num_instances)
    }

    /// Returns the total vertices for the pick draw call.
    #[must_use]
    pub fn pick_total_vertices(&self) -> u32 {
        self.gridcube_render_data
            .as_ref()
            .map_or(0, GridcubeRenderData::total_vertices)
    }

    /// Returns whether the user has requested registering the isosurface as a mesh.
    #[must_use]
    pub fn register_as_mesh_requested(&self) -> bool {
        self.register_as_mesh_requested
    }

    /// Clears the register-as-mesh request flag.
    pub fn clear_register_as_mesh_request(&mut self) {
        self.register_as_mesh_requested = false;
    }

    /// Builds egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui, colormap_names: &[&str]) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label(format!("[{:.3}, {:.3}]", self.data_min, self.data_max));
        });

        if self.enabled {
            let indent_id = egui::Id::new(&self.name).with("node_scalar_indent");
            ui.indent(indent_id, |ui| {
                // Viz mode toggle
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    if ui
                        .selectable_label(self.viz_mode == VolumeGridVizMode::Gridcube, "Gridcube")
                        .clicked()
                    {
                        self.viz_mode = VolumeGridVizMode::Gridcube;
                    }
                    if ui
                        .selectable_label(
                            self.viz_mode == VolumeGridVizMode::Isosurface,
                            "Isosurface",
                        )
                        .clicked()
                    {
                        self.viz_mode = VolumeGridVizMode::Isosurface;
                    }
                });

                match self.viz_mode {
                    VolumeGridVizMode::Gridcube => {
                        self.build_gridcube_ui(ui, colormap_names);
                    }
                    VolumeGridVizMode::Isosurface => {
                        self.build_isosurface_ui(ui);
                    }
                }
            });
        }
    }

    fn build_gridcube_ui(&mut self, ui: &mut egui::Ui, colormap_names: &[&str]) {
        // Colormap selector
        if !colormap_names.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Colormap:");
                egui::ComboBox::from_id_salt(format!("{}_colormap", self.name))
                    .selected_text(&self.color_map)
                    .show_ui(ui, |ui| {
                        for &name in colormap_names {
                            if ui.selectable_label(self.color_map == name, name).clicked() {
                                self.color_map = name.to_string();
                                self.gridcube_dirty = true;
                            }
                        }
                    });
            });
        }

        // Data range
        ui.horizontal(|ui| {
            ui.label("Range:");
            let mut min = self.data_min;
            let mut max = self.data_max;
            let speed = (max - min).abs() * 0.01;
            let speed = if speed > 0.0 { speed } else { 0.01 };
            ui.add(egui::DragValue::new(&mut min).speed(speed));
            ui.label("–");
            ui.add(egui::DragValue::new(&mut max).speed(speed));
            if (min - self.data_min).abs() > f32::EPSILON
                || (max - self.data_max).abs() > f32::EPSILON
            {
                self.data_min = min;
                self.data_max = max;
            }
        });
    }

    fn build_isosurface_ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new(format!("{}_iso_grid", self.name))
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Color:");
                let mut color = [
                    self.isosurface_color.x,
                    self.isosurface_color.y,
                    self.isosurface_color.z,
                ];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    self.isosurface_color = Vec3::new(color[0], color[1], color[2]);
                }
                ui.end_row();

                ui.label("Level:");
                let mut level = self.isosurface_level;
                let (range_min, range_max) = (self.data_min, self.data_max);
                if ui
                    .add(egui::Slider::new(&mut level, range_min..=range_max))
                    .changed()
                {
                    self.isosurface_level = level;
                    self.isosurface_dirty = true;
                }
                ui.end_row();
            });

        // Triangle count
        if let Some(mesh) = &self.isosurface_mesh_cache {
            ui.label(format!("{} tris", mesh.indices.len() / 3));
        }

        // Buttons: equal-width columns, same row
        let has_cache = self.isosurface_mesh_cache.is_some();
        if has_cache {
            ui.columns(2, |cols| {
                let w = cols[0].available_width();
                let h = cols[0].spacing().interact_size.y;
                if cols[0]
                    .add_sized([w, h], egui::Button::new("Refresh"))
                    .clicked()
                {
                    self.isosurface_dirty = true;
                    self.isosurface_mesh_cache = None;
                    self.isosurface_render_data = None;
                }
                if cols[1]
                    .add_sized([w, h], egui::Button::new("Register Mesh"))
                    .clicked()
                {
                    self.register_as_mesh_requested = true;
                }
            });
        } else if ui.button("Refresh").clicked() {
            self.isosurface_dirty = true;
            self.isosurface_mesh_cache = None;
            self.isosurface_render_data = None;
        }
    }
}

impl Quantity for VolumeGridNodeScalarQuantity {
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

    fn data_size(&self) -> usize {
        self.values.len()
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is built via build_egui_ui
    }

    fn refresh(&mut self) {
        self.gridcube_render_data = None;
        self.gridcube_dirty = true;
        self.isosurface_render_data = None;
        self.isosurface_mesh_cache = None;
        self.isosurface_dirty = true;
        self.pick_uniform_buffer = None;
        self.pick_bind_group = None;
    }

    fn clear_gpu_resources(&mut self) {
        self.gridcube_render_data = None;
        self.isosurface_render_data = None;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A scalar quantity defined at grid cells.
pub struct VolumeGridCellScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    cell_dim: UVec3,
    enabled: bool,

    // Visualization parameters
    color_map: String,
    data_min: f32,
    data_max: f32,

    // Gridcube state (cell scalars only support gridcube, not isosurface)
    gridcube_render_data: Option<GridcubeRenderData>,
    gridcube_dirty: bool,

    // Grid geometry (reserved for future gridcube world-space mapping)
    #[allow(dead_code)]
    bound_min: Vec3,
    #[allow(dead_code)]
    bound_max: Vec3,

    // Pick state
    pick_uniform_buffer: Option<wgpu::Buffer>,
    pick_bind_group: Option<wgpu::BindGroup>,
    global_start: u32,
}

impl VolumeGridCellScalarQuantity {
    /// Creates a new cell scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
        cell_dim: UVec3,
        bound_min: Vec3,
        bound_max: Vec3,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            cell_dim,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
            gridcube_render_data: None,
            gridcube_dirty: true,
            bound_min,
            bound_max,
            pick_uniform_buffer: None,
            pick_bind_group: None,
            global_start: 0,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Returns the grid cell dimensions.
    #[must_use]
    pub fn cell_dim(&self) -> UVec3 {
        self.cell_dim
    }

    /// Gets the value at a 3D index.
    #[must_use]
    pub fn get(&self, i: u32, j: u32, k: u32) -> f32 {
        let idx = i as usize
            + j as usize * self.cell_dim.x as usize
            + k as usize * self.cell_dim.x as usize * self.cell_dim.y as usize;
        self.values.get(idx).copied().unwrap_or(0.0)
    }

    /// Gets the color map name.
    #[must_use]
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self.gridcube_dirty = true;
        self
    }

    /// Gets the data range.
    #[must_use]
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Returns whether the gridcube needs GPU re-init.
    #[must_use]
    pub fn gridcube_dirty(&self) -> bool {
        self.gridcube_dirty
    }

    /// Returns the gridcube render data.
    #[must_use]
    pub fn gridcube_render_data(&self) -> Option<&GridcubeRenderData> {
        self.gridcube_render_data.as_ref()
    }

    /// Returns a mutable reference to the gridcube render data.
    pub fn gridcube_render_data_mut(&mut self) -> Option<&mut GridcubeRenderData> {
        self.gridcube_render_data.as_mut()
    }

    /// Sets the gridcube render data.
    pub fn set_gridcube_render_data(&mut self, data: GridcubeRenderData) {
        self.gridcube_render_data = Some(data);
        self.gridcube_dirty = false;
    }

    // --- Pick resources ---

    /// Initializes pick resources for this cell scalar quantity.
    pub fn init_pick_resources(
        &mut self,
        device: &wgpu::Device,
        pick_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        global_start: u32,
    ) {
        self.global_start = global_start;

        let Some(gridcube_rd) = &self.gridcube_render_data else {
            return;
        };

        let uniforms = GridcubePickUniforms {
            global_start,
            cube_size_factor: 1.0,
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube cell pick uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gridcube cell pick bind group"),
            layout: pick_bind_group_layout,
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
                    resource: gridcube_rd.position_buffer.as_entire_binding(),
                },
            ],
        });

        self.pick_uniform_buffer = Some(uniform_buffer);
        self.pick_bind_group = Some(bind_group);
    }

    /// Returns the pick bind group, if initialized.
    #[must_use]
    pub fn pick_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.pick_bind_group.as_ref()
    }

    /// Updates the pick uniform buffer with current model transform and cube size factor.
    pub fn update_pick_uniforms(
        &self,
        queue: &wgpu::Queue,
        model: [[f32; 4]; 4],
        cube_size_factor: f32,
    ) {
        if let Some(buffer) = &self.pick_uniform_buffer {
            let uniforms = GridcubePickUniforms {
                model,
                global_start: self.global_start,
                cube_size_factor,
                ..Default::default()
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Returns the number of pick elements (= number of gridcube instances).
    #[must_use]
    pub fn num_pick_elements(&self) -> u32 {
        self.gridcube_render_data
            .as_ref()
            .map_or(0, |rd| rd.num_instances)
    }

    /// Returns the total vertices for the pick draw call.
    #[must_use]
    pub fn pick_total_vertices(&self) -> u32 {
        self.gridcube_render_data
            .as_ref()
            .map_or(0, GridcubeRenderData::total_vertices)
    }

    /// Builds egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui, colormap_names: &[&str]) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label(format!("[{:.3}, {:.3}]", self.data_min, self.data_max));
        });

        if self.enabled {
            let indent_id = egui::Id::new(&self.name).with("cell_scalar_indent");
            ui.indent(indent_id, |ui| {
                // Colormap selector
                if !colormap_names.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Colormap:");
                        egui::ComboBox::from_id_salt(format!("{}_colormap", self.name))
                            .selected_text(&self.color_map)
                            .show_ui(ui, |ui| {
                                for &name in colormap_names {
                                    if ui.selectable_label(self.color_map == name, name).clicked() {
                                        self.color_map = name.to_string();
                                        self.gridcube_dirty = true;
                                    }
                                }
                            });
                    });
                }

                // Data range
                ui.horizontal(|ui| {
                    ui.label("Range:");
                    let mut min = self.data_min;
                    let mut max = self.data_max;
                    let speed = (max - min).abs() * 0.01;
                    let speed = if speed > 0.0 { speed } else { 0.01 };
                    ui.add(egui::DragValue::new(&mut min).speed(speed));
                    ui.label("–");
                    ui.add(egui::DragValue::new(&mut max).speed(speed));
                    if (min - self.data_min).abs() > f32::EPSILON
                        || (max - self.data_max).abs() > f32::EPSILON
                    {
                        self.data_min = min;
                        self.data_max = max;
                    }
                });
            });
        }
    }
}

impl Quantity for VolumeGridCellScalarQuantity {
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

    fn data_size(&self) -> usize {
        self.values.len()
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is built via build_egui_ui
    }

    fn refresh(&mut self) {
        self.gridcube_render_data = None;
        self.gridcube_dirty = true;
        self.pick_uniform_buffer = None;
        self.pick_bind_group = None;
    }

    fn clear_gpu_resources(&mut self) {
        self.gridcube_render_data = None;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a non-uniform 3x4x5 grid with field value = `world_x` and verify the
    /// isosurface lies on the plane `world_x` = level. Regression test for an
    /// indexing-order mismatch between `VolumeGrid` storage (z-slowest) and
    /// `marching_cubes` (x-slowest) — equivalent to upstream C++ commit e91a709.
    /// Also verifies that triangle winding stays consistent after the
    /// handedness-flipping swizzle.
    #[test]
    fn isosurface_x_aligned_non_uniform_grid() {
        let nx: u32 = 3;
        let ny: u32 = 4;
        let nz: u32 = 5;
        let bound_min = Vec3::new(0.0, 0.0, 0.0);
        let bound_max = Vec3::new(2.0, 3.0, 4.0); // spacing = (1, 1, 1)

        let mut values = Vec::with_capacity((nx * ny * nz) as usize);
        for _k in 0..nz {
            for _j in 0..ny {
                for i in 0..nx {
                    values.push(i as f32);
                }
            }
        }

        let mut q = VolumeGridNodeScalarQuantity::new(
            "test",
            "grid",
            values,
            UVec3::new(nx, ny, nz),
            bound_min,
            bound_max,
        );
        q.set_isosurface_level(1.5);

        let mesh = q.extract_isosurface();
        assert!(!mesh.vertices.is_empty(), "isosurface should not be empty");
        for v in &mesh.vertices {
            assert!(
                (v.x - 1.5).abs() < 1e-4,
                "vertex {v:?} should lie on plane world_x = 1.5"
            );
            assert!(
                v.y >= bound_min.y - 1e-4 && v.y <= bound_max.y + 1e-4,
                "vertex {v:?} world_y outside [0, 3]"
            );
            assert!(
                v.z >= bound_min.z - 1e-4 && v.z <= bound_max.z + 1e-4,
                "vertex {v:?} world_z outside [0, 4]"
            );
        }

        // Field gradient is +X everywhere, so outward normals point in +X.
        // Stored normals must agree, AND face normals computed from triangle
        // winding must agree — otherwise `front_facing` and registered-mesh
        // normals will be inverted.
        for n in &mesh.normals {
            assert!(
                n.x > 0.5,
                "stored normal {n:?} should point in +X direction"
            );
        }
        for tri in mesh.indices.chunks_exact(3) {
            let v0 = mesh.vertices[tri[0] as usize];
            let v1 = mesh.vertices[tri[1] as usize];
            let v2 = mesh.vertices[tri[2] as usize];
            let geom_normal = (v1 - v0).cross(v2 - v0);
            assert!(
                geom_normal.x > 0.0,
                "triangle winding gives normal {geom_normal:?}, expected +X"
            );
        }
    }

    /// Anisotropic-spacing variant: 3x4x5 grid with `bound_max = (20, 3, 4)` so
    /// `spacing = (10, 1, 1)`. Field is `world_x / 10` (i.e. integer i), level
    /// is 1.5 — vertices should land on plane `world_x = 15`. Catches a bug
    /// where someone swizzles indices but uses the wrong `spacing` axis.
    #[test]
    fn isosurface_anisotropic_spacing_x_axis() {
        let nx: u32 = 3;
        let ny: u32 = 4;
        let nz: u32 = 5;
        let bound_min = Vec3::new(0.0, 0.0, 0.0);
        let bound_max = Vec3::new(20.0, 3.0, 4.0);

        let mut values = Vec::with_capacity((nx * ny * nz) as usize);
        for _k in 0..nz {
            for _j in 0..ny {
                for i in 0..nx {
                    values.push(i as f32);
                }
            }
        }

        let mut q = VolumeGridNodeScalarQuantity::new(
            "test",
            "grid",
            values,
            UVec3::new(nx, ny, nz),
            bound_min,
            bound_max,
        );
        q.set_isosurface_level(1.5);

        let mesh = q.extract_isosurface();
        assert!(!mesh.vertices.is_empty());
        for v in &mesh.vertices {
            assert!(
                (v.x - 15.0).abs() < 1e-3,
                "vertex {v:?} should lie on plane world_x = 15.0"
            );
        }
    }

    /// Same setup but field value = `world_z`, so the isosurface should lie on a
    /// plane `world_z` = level. Catches axis-confusion regressions on Z specifically.
    #[test]
    fn isosurface_z_aligned_non_uniform_grid() {
        let nx: u32 = 3;
        let ny: u32 = 4;
        let nz: u32 = 5;
        let bound_min = Vec3::new(0.0, 0.0, 0.0);
        let bound_max = Vec3::new(2.0, 3.0, 4.0);

        let mut values = Vec::with_capacity((nx * ny * nz) as usize);
        for k in 0..nz {
            for _j in 0..ny {
                for _i in 0..nx {
                    values.push(k as f32);
                }
            }
        }

        let mut q = VolumeGridNodeScalarQuantity::new(
            "test",
            "grid",
            values,
            UVec3::new(nx, ny, nz),
            bound_min,
            bound_max,
        );
        q.set_isosurface_level(2.5);

        let mesh = q.extract_isosurface();
        assert!(!mesh.vertices.is_empty(), "isosurface should not be empty");
        for v in &mesh.vertices {
            assert!(
                (v.z - 2.5).abs() < 1e-4,
                "vertex {v:?} should lie on plane world_z = 2.5"
            );
        }
    }
}
