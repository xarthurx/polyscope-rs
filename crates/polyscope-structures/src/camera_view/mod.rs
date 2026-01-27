//! Camera view structure for visualizing camera poses.

mod camera_parameters;

pub use camera_parameters::*;

use glam::{Mat4, Vec3};
use polyscope_core::pick::PickResult;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::{HasQuantities, RenderContext, Structure};
use polyscope_render::CurveNetworkRenderData;

/// A camera view structure for visualizing camera poses.
pub struct CameraView {
    name: String,

    // Camera data
    params: CameraParameters,

    // Common structure fields
    enabled: bool,
    transform: Mat4,
    quantities: Vec<Box<dyn Quantity>>,

    // Visualization parameters
    color: Vec3,
    widget_focal_length: f32,
    widget_focal_length_is_relative: bool,
    widget_thickness: f32,

    // GPU resources (reuses CurveNetwork edge rendering)
    render_data: Option<CurveNetworkRenderData>,
}

impl CameraView {
    /// Creates a new camera view from camera parameters.
    pub fn new(name: impl Into<String>, params: CameraParameters) -> Self {
        Self {
            name: name.into(),
            params,
            enabled: true,
            transform: Mat4::IDENTITY,
            quantities: Vec::new(),
            color: Vec3::new(0.0, 0.0, 0.0),
            widget_focal_length: 0.05,
            widget_focal_length_is_relative: true,
            widget_thickness: 0.02,
            render_data: None,
        }
    }

    /// Creates a camera view with position and look direction.
    pub fn from_look_at(
        name: impl Into<String>,
        position: Vec3,
        look_at: Vec3,
        up: Vec3,
        fov_vertical_degrees: f32,
        aspect_ratio: f32,
    ) -> Self {
        let look_dir = (look_at - position).normalize();
        let params = CameraParameters::from_vectors(
            position,
            look_dir,
            up,
            fov_vertical_degrees,
            aspect_ratio,
        );
        Self::new(name, params)
    }

    /// Gets the camera parameters.
    #[must_use] 
    pub fn params(&self) -> &CameraParameters {
        &self.params
    }

    /// Updates the camera parameters.
    pub fn set_params(&mut self, params: CameraParameters) -> &mut Self {
        self.params = params;
        self.render_data = None; // Invalidate cached geometry
        self
    }

    /// Gets the widget color.
    #[must_use] 
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the widget color.
    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        // Note: render_data will be refreshed on next frame
        self
    }

    /// Gets the widget focal length (distance from camera origin to frame).
    #[must_use] 
    pub fn widget_focal_length(&self) -> f32 {
        self.widget_focal_length
    }

    /// Sets the widget focal length.
    pub fn set_widget_focal_length(&mut self, length: f32, is_relative: bool) -> &mut Self {
        self.widget_focal_length = length;
        self.widget_focal_length_is_relative = is_relative;
        self.render_data = None; // Invalidate cached geometry
        self
    }

    /// Gets the widget thickness (line/sphere radius relative to focal length).
    #[must_use] 
    pub fn widget_thickness(&self) -> f32 {
        self.widget_thickness
    }

    /// Sets the widget thickness.
    pub fn set_widget_thickness(&mut self, thickness: f32) -> &mut Self {
        self.widget_thickness = thickness;
        // Note: render_data will need to be refreshed to apply new thickness
        self
    }

    /// Computes the actual focal length based on length scale.
    fn compute_focal_length(&self, length_scale: f32) -> f32 {
        if self.widget_focal_length_is_relative {
            self.widget_focal_length * length_scale
        } else {
            self.widget_focal_length
        }
    }

    /// Computes the line radius based on focal length.
    fn compute_radius(&self, length_scale: f32) -> f32 {
        let focal = self.compute_focal_length(length_scale);
        focal * self.widget_thickness
    }

    /// Generates the camera frustum wireframe geometry.
    fn generate_wireframe(&self, length_scale: f32) -> (Vec<Vec3>, Vec<[u32; 2]>) {
        let focal = self.compute_focal_length(length_scale);

        let root = self.params.position();
        let (look_dir, up_dir, right_dir) = self.params.camera_frame();

        // Frame center is at focal distance from camera
        let frame_center = root + look_dir * focal;

        // Compute frame half-dimensions based on FoV and aspect ratio
        let half_height = focal * (self.params.fov_vertical_degrees().to_radians() / 2.0).tan();
        let half_width = self.params.aspect_ratio() * half_height;

        let frame_up = up_dir * half_height;
        let frame_right = right_dir * half_width;

        // Frame corners
        let upper_left = frame_center + frame_up - frame_right;
        let upper_right = frame_center + frame_up + frame_right;
        let lower_left = frame_center - frame_up - frame_right;
        let lower_right = frame_center - frame_up + frame_right;

        // Orientation triangle (above frame)
        let tri_left = frame_center + frame_up * 1.2 - frame_right * 0.7;
        let tri_right = frame_center + frame_up * 1.2 + frame_right * 0.7;
        let tri_top = frame_center + frame_up * 2.0;

        // Nodes: 0=root, 1-4=corners, 5-7=triangle
        let nodes = vec![
            root,        // 0
            upper_left,  // 1
            upper_right, // 2
            lower_left,  // 3
            lower_right, // 4
            tri_left,    // 5
            tri_right,   // 6
            tri_top,     // 7
        ];

        // Edges
        let edges = vec![
            // From root to corners
            [0, 1],
            [0, 2],
            [0, 3],
            [0, 4],
            // Frame rectangle
            [1, 2],
            [2, 4],
            [4, 3],
            [3, 1],
            // Orientation triangle
            [5, 6],
            [6, 7],
            [7, 5],
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
        length_scale: f32,
    ) {
        let (nodes, edges) = self.generate_wireframe(length_scale);

        // Build edge indices
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

        // Update uniforms with our custom settings
        let uniforms = polyscope_render::CurveNetworkUniforms {
            color: [self.color.x, self.color.y, self.color.z, 1.0],
            radius: self.compute_radius(length_scale),
            radius_is_relative: 0, // Absolute radius since we already computed it
            render_mode: 0,        // Lines
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

    /// Builds the egui UI for this camera view.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        // Color picker
        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut color = [self.color.x, self.color.y, self.color.z];
            if ui.color_edit_button_rgb(&mut color).changed() {
                self.set_color(Vec3::new(color[0], color[1], color[2]));
            }
        });

        // Widget thickness
        ui.horizontal(|ui| {
            ui.label("Thickness:");
            let mut thickness = self.widget_thickness;
            if ui
                .add(
                    egui::DragValue::new(&mut thickness)
                        .speed(0.001)
                        .range(0.001..=0.5),
                )
                .changed()
            {
                self.set_widget_thickness(thickness);
            }
        });

        // Camera info
        ui.separator();
        ui.label(format!(
            "Position: ({:.2}, {:.2}, {:.2})",
            self.params.position().x,
            self.params.position().y,
            self.params.position().z
        ));
        ui.label(format!("FoV: {:.1}°", self.params.fov_vertical_degrees()));
        ui.label(format!("Aspect: {:.2}", self.params.aspect_ratio()));
    }
}

impl Structure for CameraView {
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
        "CameraView"
    }

    fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        // Bounding box is just the camera position
        // The frustum extends based on length scale which we don't have here
        Some((self.params.position(), self.params.position()))
    }

    fn length_scale(&self) -> f32 {
        // No obvious length scale for a camera
        0.0
    }

    fn transform(&self) -> Mat4 {
        self.transform
    }

    fn set_transform(&mut self, transform: Mat4) {
        self.transform = transform;
        // Note: transform is applied in world space, render_data will need refresh
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn draw(&self, _ctx: &mut dyn RenderContext) {
        // Drawing is handled externally using render_data()
    }

    fn draw_pick(&self, _ctx: &mut dyn RenderContext) {
        // Picking not yet implemented
    }

    fn build_ui(&mut self, _ui: &dyn std::any::Any) {
        // UI is built via build_egui_ui
    }

    fn build_pick_ui(&self, _ui: &dyn std::any::Any, _pick: &PickResult) {
        // Pick UI not implemented
    }

    fn refresh(&mut self) {
        // Invalidate render data so it will be regenerated
        self.render_data = None;
        for quantity in &mut self.quantities {
            quantity.refresh();
        }
    }
}

impl HasQuantities for CameraView {
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
