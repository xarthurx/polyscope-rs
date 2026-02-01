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
        if min > max {
            (0.0, 1.0)
        } else {
            (min, max)
        }
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
    /// MC output vertices are in grid index space: vertex (i,j,k) has coords
    /// that need swizzle(z,y,x) * `grid_spacing` + `bound_min` to transform to world space.
    pub fn extract_isosurface(&mut self) -> &McmMesh {
        if self.isosurface_mesh_cache.is_none() || self.isosurface_dirty {
            let nx = self.node_dim.x;
            let ny = self.node_dim.y;
            let nz = self.node_dim.z;

            let mut mesh = marching_cubes(&self.values, self.isosurface_level, nx, ny, nz);

            // Transform from MC index space to world space
            // MC uses indexing (i * ny + j) * nz + k, output coords are in (i,j,k) space
            // Need to map: x_world = x_mc * spacing_z + bound_min.z (swizzle z,y,x)
            let cell_dim = Vec3::new(
                (nx - 1).max(1) as f32,
                (ny - 1).max(1) as f32,
                (nz - 1).max(1) as f32,
            );
            let spacing = (self.bound_max - self.bound_min) / cell_dim;

            for v in &mut mesh.vertices {
                // MC output: v.x is in i-dimension, v.y in j-dimension, v.z in k-dimension
                // Grid layout: i maps to x, j maps to y, k maps to z (no swizzle needed
                // since our MC uses same indexing as the grid)
                *v = Vec3::new(
                    v.x * spacing.x + self.bound_min.x,
                    v.y * spacing.y + self.bound_min.y,
                    v.z * spacing.z + self.bound_min.z,
                );
            }

            // Transform normals (only need to scale, then renormalize)
            for n in &mut mesh.normals {
                // Scale normals by inverse spacing to account for non-uniform grid
                *n = Vec3::new(
                    n.x / spacing.x,
                    n.y / spacing.y,
                    n.z / spacing.z,
                );
                let len = n.length();
                if len > 0.0 {
                    *n /= len;
                }
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
    pub fn build_egui_ui(
        &mut self,
        ui: &mut egui::Ui,
        colormap_names: &[&str],
    ) {
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
                        .selectable_label(
                            self.viz_mode == VolumeGridVizMode::Gridcube,
                            "Gridcube",
                        )
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
                            if ui
                                .selectable_label(self.color_map == name, name)
                                .clicked()
                            {
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
        egui::Grid::new(format!("{}_iso_grid", self.name)).num_columns(2).show(ui, |ui| {
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
                if cols[0].add_sized([w, h], egui::Button::new("Refresh")).clicked() {
                    self.isosurface_dirty = true;
                    self.isosurface_mesh_cache = None;
                    self.isosurface_render_data = None;
                }
                if cols[1].add_sized([w, h], egui::Button::new("Register Mesh")).clicked() {
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A scalar quantity defined at grid cells.
#[allow(dead_code)]
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

    // Grid geometry
    bound_min: Vec3,
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
        if min > max {
            (0.0, 1.0)
        } else {
            (min, max)
        }
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
    pub fn build_egui_ui(
        &mut self,
        ui: &mut egui::Ui,
        colormap_names: &[&str],
    ) {
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
                                    if ui
                                        .selectable_label(self.color_map == name, name)
                                        .clicked()
                                    {
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
