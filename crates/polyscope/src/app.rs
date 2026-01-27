//! Application window and event loop management.

use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;
use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use polyscope_core::slice_plane::SlicePlaneUniforms;
use polyscope_core::{GroundPlaneConfig, GroundPlaneMode};
use polyscope_render::{reflection, PickResult, RenderEngine};
use polyscope_structures::{
    CameraView, CurveNetwork, PointCloud, SurfaceMesh, VolumeGrid, VolumeMesh,
};
use polyscope_ui::EguiIntegration;

use crate::Vec3;

/// The polyscope application state.
pub struct App {
    window: Option<Arc<Window>>,
    engine: Option<RenderEngine>,
    egui: Option<EguiIntegration>,
    close_requested: bool,
    background_color: Vec3,
    // Mouse state for camera control
    // These track the PHYSICAL button state, updated on every press/release
    mouse_pos: (f64, f64),
    left_mouse_down: bool,
    right_mouse_down: bool,
    // Modifier keys
    shift_down: bool,
    // Drag tracking - accumulated distance since mouse press
    drag_distance: f64,
    // Selection state
    selection: Option<PickResult>,
    last_click_pos: Option<(f64, f64)>,
    // GPU picking - selected element index (from GPU pick)
    selected_element_index: Option<u32>,
    // Ground plane settings
    ground_plane: GroundPlaneConfig,
    // Screenshot state
    screenshot_pending: Option<String>,
    screenshot_counter: u32,
    // Camera settings UI state
    camera_settings: polyscope_ui::CameraSettings,
    // Scene extents UI state
    scene_extents: polyscope_ui::SceneExtents,
    // Appearance settings UI state
    appearance_settings: polyscope_ui::AppearanceSettings,
    // Slice plane UI state
    slice_plane_settings: Vec<polyscope_ui::SlicePlaneSettings>,
    new_slice_plane_name: String,
    // Group UI state
    group_settings: Vec<polyscope_ui::GroupSettings>,
    new_group_name: String,
    // Gizmo UI state
    gizmo_settings: polyscope_ui::GizmoSettings,
    selection_info: polyscope_ui::SelectionInfo,
    // Slice plane gizmo state
    slice_plane_selection: polyscope_ui::SlicePlaneSelectionInfo,
    // Visual gizmo
    transform_gizmo: polyscope_ui::TransformGizmo,
    // Tone mapping settings
    tone_mapping_settings: polyscope_ui::ToneMappingSettings,
    // Whether the camera has been auto-fitted to the scene
    camera_fitted: bool,
}

impl App {
    /// Creates a new application.
    pub fn new() -> Self {
        Self {
            window: None,
            engine: None,
            egui: None,
            close_requested: false,
            background_color: Vec3::new(0.1, 0.1, 0.1),
            mouse_pos: (0.0, 0.0),
            left_mouse_down: false,
            right_mouse_down: false,
            shift_down: false,
            drag_distance: 0.0,
            selection: None,
            last_click_pos: None,
            selected_element_index: None,
            ground_plane: GroundPlaneConfig::default(),
            screenshot_pending: None,
            screenshot_counter: 0,
            camera_settings: polyscope_ui::CameraSettings::default(),
            scene_extents: polyscope_ui::SceneExtents::default(),
            appearance_settings: polyscope_ui::AppearanceSettings::default(),
            slice_plane_settings: crate::get_slice_plane_settings(),
            new_slice_plane_name: String::new(),
            group_settings: crate::get_group_settings(),
            new_group_name: String::new(),
            gizmo_settings: crate::get_gizmo_settings(),
            selection_info: polyscope_ui::SelectionInfo::default(),
            slice_plane_selection: polyscope_ui::SlicePlaneSelectionInfo::default(),
            transform_gizmo: polyscope_ui::TransformGizmo::new(),
            tone_mapping_settings: polyscope_ui::ToneMappingSettings::default(),
            camera_fitted: false,
        }
    }

    /// Requests a screenshot with an auto-generated filename.
    pub fn request_auto_screenshot(&mut self) {
        let filename = format!("screenshot_{:04}.png", self.screenshot_counter);
        self.screenshot_counter += 1;
        self.screenshot_pending = Some(filename);
    }

    /// Sets the background color.
    #[allow(dead_code)]
    pub fn set_background_color(&mut self, color: Vec3) {
        self.background_color = color;
    }

    /// Performs GPU-based picking to find which structure and element is at the given screen position.
    ///
    /// Uses the GPU pick buffer to determine the exact structure and element at the click position.
    /// Returns (`type_name`, name, `element_index`) or None if clicking on empty space.
    fn gpu_pick_at(&self, x: u32, y: u32) -> Option<(String, String, u32)> {
        let engine = self.engine.as_ref()?;

        // Read pick buffer
        let (struct_id, elem_id) = engine.pick_at(x, y)?;

        // Background check (struct_id 0 means nothing was hit)
        if struct_id == 0 {
            return None;
        }

        // Look up structure info from ID
        let (type_name, name) = engine.lookup_structure_id(struct_id)?;
        Some((type_name.to_string(), name.to_string(), u32::from(elem_id)))
    }

    /// Performs screen-space picking to find which structure (if any) is at the given screen position.
    ///
    /// Projects sample points from each structure to screen space and checks if the click
    /// is within a threshold distance. Returns the (`type_name`, name) of the clicked structure,
    /// or None if clicking on empty space.
    ///
    /// NOTE: This is the fallback method. GPU picking (`gpu_pick_at`) is preferred when available.
    #[allow(dead_code)]
    fn pick_structure_at_screen_pos(
        &self,
        click_pos: glam::Vec2,
        screen_width: u32,
        screen_height: u32,
        camera: &polyscope_render::Camera,
    ) -> Option<(String, String)> {
        let view_proj = camera.view_projection_matrix();
        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;

        // Threshold distance in pixels for considering a click "on" a structure
        let pick_threshold = 12.0_f32;
        let mut best_match: Option<(String, String, f32)> = None;

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !structure.is_enabled() {
                    continue;
                }

                // Point clouds have GPU picking; skip them in the fallback to avoid sticky picks.
                if structure.type_name() == "PointCloud" {
                    continue;
                }

                let model = structure.transform();
                let mvp = view_proj * model;

                // Gate by screen-space bounding box when available to avoid overly permissive picks.
                if let Some((min, max)) = structure.bounding_box() {
                    let corners = [
                        Vec3::new(min.x, min.y, min.z),
                        Vec3::new(min.x, min.y, max.z),
                        Vec3::new(min.x, max.y, min.z),
                        Vec3::new(min.x, max.y, max.z),
                        Vec3::new(max.x, min.y, min.z),
                        Vec3::new(max.x, min.y, max.z),
                        Vec3::new(max.x, max.y, min.z),
                        Vec3::new(max.x, max.y, max.z),
                    ];

                    let mut min_x = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY;
                    let mut min_y = f32::INFINITY;
                    let mut max_y = f32::NEG_INFINITY;
                    let mut any_valid = false;

                    for corner in corners {
                        let clip = mvp * glam::Vec4::new(corner.x, corner.y, corner.z, 1.0);
                        if clip.w <= 0.0 {
                            continue;
                        }
                        let ndc = clip.truncate() / clip.w;
                        let screen_x = (ndc.x + 1.0) * half_width;
                        let screen_y = (1.0 - ndc.y) * half_height;
                        min_x = min_x.min(screen_x);
                        max_x = max_x.max(screen_x);
                        min_y = min_y.min(screen_y);
                        max_y = max_y.max(screen_y);
                        any_valid = true;
                    }

                    if any_valid {
                        let pad = pick_threshold;
                        if click_pos.x < min_x - pad
                            || click_pos.x > max_x + pad
                            || click_pos.y < min_y - pad
                            || click_pos.y > max_y + pad
                        {
                            continue;
                        }
                    }
                }

                // Get sample points based on structure type
                let sample_points: Vec<Vec3> = if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                        // Sample up to 100 points for efficiency
                        let points = pc.points();
                        let step = (points.len() / 100).max(1);
                        points.iter().step_by(step).copied().collect()
                    } else {
                        continue;
                    }
                } else if structure.type_name() == "SurfaceMesh" {
                    if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                        // Sample vertices
                        let verts = mesh.vertices();
                        let step = (verts.len() / 100).max(1);
                        verts.iter().step_by(step).copied().collect()
                    } else {
                        continue;
                    }
                } else if structure.type_name() == "VolumeMesh" {
                    if let Some(vol) = structure.as_any().downcast_ref::<VolumeMesh>() {
                        // Sample vertices for easier picking
                        let verts = vol.vertices();
                        let step = (verts.len() / 100).max(1);
                        verts.iter().step_by(step).copied().collect()
                    } else {
                        continue;
                    }
                } else {
                    // For other structure types, use bounding box center
                    if let Some((min, max)) = structure.bounding_box() {
                        vec![(min + max) * 0.5]
                    } else {
                        continue;
                    }
                };

                // Project each sample point and check distance to click
                for point in sample_points {
                    let clip = mvp * glam::Vec4::new(point.x, point.y, point.z, 1.0);

                    // Skip points behind camera
                    if clip.w <= 0.0 {
                        continue;
                    }

                    // NDC to screen
                    let ndc = clip.truncate() / clip.w;
                    let screen_x = (ndc.x + 1.0) * half_width;
                    let screen_y = (1.0 - ndc.y) * half_height; // Y is flipped

                    let dist = ((screen_x - click_pos.x).powi(2)
                        + (screen_y - click_pos.y).powi(2))
                    .sqrt();

                    if dist < pick_threshold {
                        // Check if this is closer than previous best match
                        let dominated = best_match
                            .as_ref()
                            .is_some_and(|(_, _, best_dist)| dist >= *best_dist);
                        if !dominated {
                            best_match = Some((
                                structure.type_name().to_string(),
                                structure.name().to_string(),
                                dist,
                            ));
                        }
                    }
                }
            }
        });

        best_match.map(|(type_name, name, _)| (type_name, name))
    }

    fn screen_ray(
        &self,
        click_pos: glam::Vec2,
        screen_width: u32,
        screen_height: u32,
        camera: &polyscope_render::Camera,
    ) -> Option<(Vec3, Vec3)> {
        if screen_width == 0 || screen_height == 0 {
            return None;
        }

        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;
        let ndc_x = (click_pos.x / half_width) - 1.0;
        let ndc_y = 1.0 - (click_pos.y / half_height);

        let inv_view_proj = camera.view_projection_matrix().inverse();

        // wgpu-style NDC depth [0, 1]
        let near = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        if near.w.abs() < 1e-6 || far.w.abs() < 1e-6 {
            return None;
        }

        let ray_origin = near.truncate() / near.w;
        let ray_far = far.truncate() / far.w;
        let ray_dir = (ray_far - ray_origin).normalize_or_zero();
        if ray_dir.length_squared() < 1e-12 {
            return None;
        }

        Some((ray_origin, ray_dir))
    }

    /// Tests whether a ray intersects a visible slice plane quad.
    fn pick_slice_plane_at_ray(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<(String, f32)> {
        let mut best_hit: Option<(String, f32)> = None;
        crate::with_context(|ctx| {
            for plane in ctx.slice_planes() {
                if !plane.is_enabled() || !plane.draw_plane() {
                    continue;
                }

                let normal = plane.normal();
                let denom = normal.dot(ray_dir);
                if denom.abs() < 1e-6 {
                    continue;
                }

                let t = (plane.origin() - ray_origin).dot(normal) / denom;
                if t < 0.0 {
                    continue;
                }

                let hit = ray_origin + ray_dir * t;

                // Compute local plane axes (match visualization orientation)
                let up = if normal.dot(Vec3::Y).abs() < 0.99 { Vec3::Y } else { Vec3::Z };
                let y_axis = up.cross(normal).normalize();
                let z_axis = normal.cross(y_axis).normalize();

                let local = hit - plane.origin();
                let y = local.dot(y_axis);
                let z = local.dot(z_axis);
                let size = plane.plane_size();

                if y.abs() <= size && z.abs() <= size {
                    let is_better =
                        best_hit.as_ref().map_or(true, |(_, best_t)| t < *best_t);
                    if is_better {
                        best_hit = Some((plane.name().to_string(), t));
                    }
                }
            }
        });

        best_hit
    }

    fn ray_intersect_triangle(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
    ) -> Option<f32> {
        let eps = 1e-6;
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = ray_dir.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < eps {
            return None;
        }
        let f = 1.0 / a;
        let s = ray_origin - v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * ray_dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);
        if t > eps {
            Some(t)
        } else {
            None
        }
    }

    fn pick_structure_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        plane_params: &[(Vec3, Vec3)],
    ) -> Option<(String, String, f32)> {
        let mut best_hit: Option<(String, String, f32)> = None;

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !structure.is_enabled() {
                    continue;
                }

                match structure.type_name() {
                    "SurfaceMesh" => {
                        let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() else {
                            continue;
                        };
                        let model = structure.transform();
                        let mut world_verts = Vec::with_capacity(mesh.vertices().len());
                        for v in mesh.vertices() {
                            world_verts.push((model * v.extend(1.0)).truncate());
                        }

                        let mut hit_t: Option<f32> = None;
                        for tri in mesh.triangulation() {
                            let v0 = world_verts[tri[0] as usize];
                            let v1 = world_verts[tri[1] as usize];
                            let v2 = world_verts[tri[2] as usize];
                            if let Some(t) = self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2) {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better = best_hit
                                .as_ref()
                                .map_or(true, |(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit =
                                    Some((structure.type_name().to_string(), structure.name().to_string(), t));
                            }
                        }
                    }
                    "VolumeMesh" => {
                        let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() else {
                            continue;
                        };
                        let model = structure.transform();
                        let (positions, faces) = vm.pick_triangles(plane_params);
                        if positions.is_empty() || faces.is_empty() {
                            continue;
                        }
                        let mut world_positions = Vec::with_capacity(positions.len());
                        for v in positions {
                            world_positions.push((model * v.extend(1.0)).truncate());
                        }

                        let mut hit_t: Option<f32> = None;
                        for tri in faces {
                            let v0 = world_positions[tri[0] as usize];
                            let v1 = world_positions[tri[1] as usize];
                            let v2 = world_positions[tri[2] as usize];
                            if let Some(t) = self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2) {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better = best_hit
                                .as_ref()
                                .map_or(true, |(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit =
                                    Some((structure.type_name().to_string(), structure.name().to_string(), t));
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        best_hit
    }

    fn pick_point_cloud_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        name: &str,
        element_index: u32,
    ) -> Option<f32> {
        crate::with_context(|ctx| {
            let structure = ctx.registry.get("PointCloud", name)?;
            let pc = structure.as_any().downcast_ref::<PointCloud>()?;
            let points = pc.points();
            let idx = element_index as usize;
            if idx >= points.len() {
                return None;
            }
            let model = structure.transform();
            let world_point = (model * points[idx].extend(1.0)).truncate();
            let t = (world_point - ray_origin).dot(ray_dir);
            if t < 0.0 {
                return None;
            }
            let closest = ray_origin + ray_dir * t;
            let dist = (world_point - closest).length();
            let radius_world = model
                .transform_vector3(Vec3::new(pc.point_radius(), 0.0, 0.0))
                .length();
            if dist <= radius_world.max(1e-4) * 1.5 {
                Some(t)
            } else {
                None
            }
        })
    }

    fn select_slice_plane_by_name(&mut self, name: &str) {
        let mut selected_settings: Option<polyscope_ui::SlicePlaneSettings> = None;
        for settings in &mut self.slice_plane_settings {
            if settings.name == name {
                settings.is_selected = true;
                settings.draw_widget = true;
                selected_settings = Some(settings.clone());
            } else {
                settings.is_selected = false;
            }
        }
        crate::select_slice_plane_for_gizmo(name);
        if let Some(settings) = selected_settings {
            crate::apply_slice_plane_settings(&settings);
        }
    }

    fn deselect_slice_plane_selection(&mut self) {
        for settings in &mut self.slice_plane_settings {
            settings.is_selected = false;
        }
        crate::deselect_slice_plane_gizmo();
    }

    /// Renders a single frame.
    fn render(&mut self) {
        let (Some(engine), Some(egui), Some(window)) =
            (&mut self.engine, &mut self.egui, &self.window)
        else {
            return;
        };

        // Check surface exists (but don't hold borrow yet - needed for structure ID assignment)
        if engine.surface.is_none() {
            return;
        }

        // Auto-fit camera to scene on first render with structures
        if !self.camera_fitted {
            let (has_structures, bbox) = crate::with_context(|ctx| {
                let has_structures = !ctx.registry.is_empty();
                (has_structures, ctx.bounding_box)
            });

            if has_structures {
                let (min, max) = bbox;
                // Only fit if bounding box is valid (not default zeros or infinities)
                if min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0 {
                    engine.camera.look_at_box(min, max);
                    self.camera_fitted = true;
                }
            }
        }

        // Update camera uniforms
        engine.update_camera_uniforms();

        // Update slice plane uniforms
        crate::with_context(|ctx| {
            engine.update_slice_plane_uniforms(
                ctx.slice_planes().map(SlicePlaneUniforms::from),
            );
        });

        // Initialize GPU resources for any uninitialized point clouds and vector quantities
        crate::with_context_mut(|ctx| {
            // Collect slice plane data before the loop to avoid borrow conflicts
            let slice_planes: Vec<_> = ctx.slice_planes().cloned().collect();

            for structure in ctx.registry.iter_mut() {
                if structure.type_name() == "PointCloud" {
                    let structure_name = structure.name().to_string();
                    if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                        // Initialize point cloud render data
                        if pc.render_data().is_none() {
                            pc.init_gpu_resources(
                                &engine.device,
                                engine.point_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }

                        // Initialize pick resources (after render data)
                        if pc.pick_bind_group().is_none() && pc.render_data().is_some() {
                            let structure_id = engine.assign_structure_id("PointCloud", &structure_name);
                            pc.init_pick_resources(
                                &engine.device,
                                engine.pick_bind_group_layout(),
                                engine.camera_buffer(),
                                structure_id,
                            );
                        }

                        // Initialize vector quantity render data if enabled
                        let points = pc.points().to_vec();
                        if let Some(vq) = pc.active_vector_quantity_mut() {
                            if vq.render_data().is_none() {
                                vq.init_gpu_resources(
                                    &engine.device,
                                    engine.vector_bind_group_layout(),
                                    engine.camera_buffer(),
                                    &points,
                                );
                            }
                        }
                    }
                }

                if structure.type_name() == "SurfaceMesh" {
                    if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
                        if mesh.render_data().is_none() {
                            mesh.init_gpu_resources(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }
                        // Initialize shadow resources if render data exists but shadow doesn't
                        if mesh.render_data().is_some() && !mesh.has_shadow_resources() {
                            if let (Some(shadow_layout), Some(shadow_pass)) =
                                (engine.shadow_bind_group_layout(), engine.shadow_map_pass())
                            {
                                mesh.init_shadow_resources(
                                    &engine.device,
                                    shadow_layout,
                                    shadow_pass.light_buffer(),
                                );
                            }
                        }
                    }
                }

                if structure.type_name() == "CurveNetwork" {
                    if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
                        if cn.render_data().is_none() {
                            cn.init_gpu_resources(
                                &engine.device,
                                engine.curve_network_edge_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }
                        // Check what needs initialization
                        let needs_tube = cn.render_data().is_some_and(|rd| !rd.has_tube_resources());
                        let needs_node = cn.render_data().is_some_and(|rd| !rd.has_node_render_resources());

                        // Initialize tube resources if not already done
                        if needs_tube {
                            cn.init_tube_resources(
                                &engine.device,
                                engine.curve_network_tube_compute_bind_group_layout(),
                                engine.curve_network_tube_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }
                        // Initialize node render resources for sphere joints
                        if needs_node {
                            cn.init_node_render_resources(
                                &engine.device,
                                engine.point_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }
                    }
                }

                if structure.type_name() == "CameraView" {
                    if let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>() {
                        if cv.render_data().is_none() {
                            cv.init_render_data(
                                &engine.device,
                                engine.curve_network_edge_bind_group_layout(),
                                engine.camera_buffer(),
                                &engine.queue,
                                ctx.length_scale,
                            );
                        }
                    }
                }

                if structure.type_name() == "VolumeGrid" {
                    if let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>() {
                        if vg.render_data().is_none() {
                            vg.init_render_data(
                                &engine.device,
                                engine.curve_network_edge_bind_group_layout(),
                                engine.camera_buffer(),
                                &engine.queue,
                            );
                        }
                    }
                }

                if structure.type_name() == "VolumeMesh" {
                    if let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>() {
                        let mut enabled_planes: Vec<(String, Vec3, Vec3)> = slice_planes
                            .iter()
                            .filter(|p| p.is_enabled())
                            .map(|p| (p.name().to_string(), p.origin(), p.normal()))
                            .collect();
                        enabled_planes.sort_by(|a, b| a.0.cmp(&b.0));

                        let plane_params: Vec<(Vec3, Vec3)> = enabled_planes
                            .iter()
                            .map(|(_, origin, normal)| (*origin, *normal))
                            .collect();

                        if !plane_params.is_empty() {
                            // Use cell culling: regenerate geometry with only visible cells
                            // (cells whose centroid is on the positive side of all enabled planes)
                            vm.update_render_data_with_culling(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                                &plane_params,
                            );
                        } else if vm.is_culled() {
                            // Was culled but no slice plane is active now - reset to show all cells
                            vm.reset_render_data(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        } else if vm.render_data().is_none() {
                            // No slice plane active, initialize normally
                            vm.init_render_data(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }
                    }
                }
            }
        });

        // Update GPU buffers for point clouds and vector quantities
        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                        pc.update_gpu_buffers(&engine.queue, &engine.color_maps);
                        // Update pick uniforms (point radius may have changed)
                        pc.update_pick_uniforms(&engine.queue);

                        // Update vector quantity uniforms
                        if let Some(vq) = pc.active_vector_quantity() {
                            vq.update_uniforms(&engine.queue);
                        }
                    }
                }

                if structure.type_name() == "SurfaceMesh" {
                    if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                        mesh.update_gpu_buffers(&engine.queue, &engine.color_maps);
                    }
                }

                if structure.type_name() == "CurveNetwork" {
                    if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                        cn.update_gpu_buffers(&engine.queue, &engine.color_maps);
                    }
                }

                if structure.type_name() == "VolumeMesh" {
                    if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                        vm.update_gpu_buffers(&engine.queue);
                    }
                }
            }
        });

        // Render pick pass (GPU picking)
        {
            let mut encoder = engine
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("pick pass encoder"),
                });

            if let Some(mut pick_pass) = engine.begin_pick_pass(&mut encoder) {
                // Draw point clouds to pick buffer
                pick_pass.set_pipeline(engine.point_pick_pipeline());

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let (Some(pick_bind_group), Some(render_data)) =
                                    (pc.pick_bind_group(), pc.render_data())
                                {
                                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                                    // 6 vertices per quad, num_points instances
                                    pick_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
                        }
                    }
                });

                // Note: SurfaceMesh and CurveNetwork pick rendering would go here
                // once their pick pipelines are created
            }

            engine.queue.submit(std::iter::once(encoder.finish()));
        }

        // Begin egui frame
        egui.begin_frame(window);

        // Build UI
        let mut bg_color = [
            self.background_color.x,
            self.background_color.y,
            self.background_color.z,
        ];

        // Extract ground plane settings for UI
        let mut gp_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::Tile => 1u32,
            GroundPlaneMode::ShadowOnly => 2u32,
            GroundPlaneMode::TileReflection => 3u32,
        };
        let mut gp_height = self.ground_plane.height;
        let mut gp_height_is_relative = self.ground_plane.height_is_relative;
        let mut gp_shadow_blur_iters = self.ground_plane.shadow_blur_iters;
        let mut gp_shadow_darkness = self.ground_plane.shadow_darkness;
        let mut gp_reflection_intensity = self.ground_plane.reflection_intensity;

        // Sync camera settings from engine
        self.camera_settings = crate::camera_to_settings(&engine.camera);

        // Sync scene extents from context
        self.scene_extents = crate::get_scene_extents();

        let mut camera_changed = false;
        let mut scene_extents_changed = false;
        let mut screenshot_requested = false;

        polyscope_ui::build_left_panel(&egui.context, |ui| {
            let view_action = polyscope_ui::build_controls_section(ui, &mut bg_color);
            match view_action {
                polyscope_ui::ViewAction::Screenshot => {
                    screenshot_requested = true;
                }
                polyscope_ui::ViewAction::ResetView => {
                    // TODO: Reset camera view
                }
                polyscope_ui::ViewAction::None => {}
            }

            // Camera settings panel
            if polyscope_ui::build_camera_settings_section(ui, &mut self.camera_settings) {
                camera_changed = true;
            }

            // Scene extents panel
            if polyscope_ui::build_scene_extents_section(ui, &mut self.scene_extents) {
                scene_extents_changed = true;
            }

            // Appearance settings panel
            if polyscope_ui::build_appearance_section(ui, &mut self.appearance_settings) {
                // Sync SSAO settings to global options
                polyscope_core::with_context_mut(|ctx| {
                    ctx.options.ssao.enabled = self.appearance_settings.ssao_enabled;
                    ctx.options.ssao.radius = self.appearance_settings.ssao_radius;
                    ctx.options.ssao.intensity = self.appearance_settings.ssao_intensity;
                    ctx.options.ssao.bias = self.appearance_settings.ssao_bias;
                    ctx.options.ssao.sample_count = self.appearance_settings.ssao_sample_count;
                });
            }

            // Tone mapping settings panel
            polyscope_ui::panels::build_tone_mapping_section(ui, &mut self.tone_mapping_settings);

            // Slice Planes section
            let slice_action = polyscope_ui::panels::build_slice_planes_section(
                ui,
                &mut self.slice_plane_settings,
                &mut self.new_slice_plane_name,
            );
            if slice_action != polyscope_ui::SlicePlanesAction::None {
                crate::handle_slice_plane_action(
                    slice_action.clone(),
                    &mut self.slice_plane_settings,
                );
                if matches!(slice_action, polyscope_ui::SlicePlanesAction::Add(_)) {
                    self.new_slice_plane_name.clear();
                }
            }

            // Groups section
            let groups_action = polyscope_ui::panels::build_groups_section(
                ui,
                &mut self.group_settings,
                &mut self.new_group_name,
            );
            if groups_action != polyscope_ui::GroupsAction::None {
                crate::handle_group_action(groups_action.clone(), &mut self.group_settings);
                if matches!(groups_action, polyscope_ui::GroupsAction::Create(_)) {
                    self.new_group_name.clear();
                }
            }

            // Sync selection info from context
            self.selection_info = crate::get_selection_info();

            // Gizmo section
            let gizmo_action = polyscope_ui::panels::build_gizmo_section(
                ui,
                &mut self.gizmo_settings,
                &mut self.selection_info,
            );
            if gizmo_action != polyscope_ui::GizmoAction::None {
                crate::handle_gizmo_action(
                    gizmo_action,
                    &self.gizmo_settings,
                    &self.selection_info,
                );
            }

            polyscope_ui::build_ground_plane_section(
                ui,
                &mut gp_mode,
                &mut gp_height,
                &mut gp_height_is_relative,
                &mut gp_shadow_blur_iters,
                &mut gp_shadow_darkness,
                &mut gp_reflection_intensity,
            );

            // Collect structure info
            let structures: Vec<(String, String, bool)> = crate::with_context(|ctx| {
                ctx.registry
                    .iter()
                    .map(|s| {
                        (
                            s.type_name().to_string(),
                            s.name().to_string(),
                            s.is_enabled(),
                        )
                    })
                    .collect()
            });

            polyscope_ui::build_structure_tree_with_ui(
                ui,
                &structures,
                |type_name, name, enabled| {
                    crate::with_context_mut(|ctx| {
                        if let Some(s) = ctx.registry.get_mut(type_name, name) {
                            s.set_enabled(enabled);
                        }
                    });
                },
                |ui, type_name, name| {
                    // Build structure-specific UI
                    crate::with_context_mut(|ctx| {
                        if let Some(s) = ctx.registry.get_mut(type_name, name) {
                            if type_name == "PointCloud" {
                                if let Some(pc) = s.as_any_mut().downcast_mut::<PointCloud>() {
                                    pc.build_egui_ui(ui);
                                }
                            }
                            if type_name == "SurfaceMesh" {
                                if let Some(mesh) = s.as_any_mut().downcast_mut::<SurfaceMesh>() {
                                    mesh.build_egui_ui(ui);
                                }
                            }
                            if type_name == "CurveNetwork" {
                                if let Some(cn) = s.as_any_mut().downcast_mut::<CurveNetwork>() {
                                    cn.build_egui_ui(ui);
                                }
                            }
                            if type_name == "CameraView" {
                                if let Some(cv) = s.as_any_mut().downcast_mut::<CameraView>() {
                                    cv.build_egui_ui(ui);
                                }
                            }
                            if type_name == "VolumeGrid" {
                                if let Some(vg) = s.as_any_mut().downcast_mut::<VolumeGrid>() {
                                    vg.build_egui_ui(ui);
                                }
                            }
                            if type_name == "VolumeMesh" {
                                if let Some(vm) = s.as_any_mut().downcast_mut::<VolumeMesh>() {
                                    vm.build_egui_ui(ui);
                                }
                            }
                        }
                    });
                },
            );
        });

        // Show selection panel if we have a selection
        if let Some(ref selection) = self.selection {
            if selection.hit {
                polyscope_ui::build_selection_panel(&egui.context, selection, |ui| {
                    // Structure-specific pick UI (placeholder for now)
                    ui.label("Quantity values would appear here");
                });
            }
        }

        // Common gizmo setup - check if pointer is over UI panel
        const LEFT_PANEL_WIDTH: f32 = 320.0;
        let pointer_over_ui = egui.context.input(|i| {
            i.pointer.hover_pos().is_some_and(|pos| pos.x <= LEFT_PANEL_WIDTH)
        });

        // Get camera matrices from engine - MUST match what's used for 3D rendering
        let view_matrix = engine.camera.view_matrix();
        let projection_matrix = engine.camera.projection_matrix();

        // Common viewport for gizmo rendering
        let full_window_viewport = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(engine.width as f32, engine.height as f32),
        );

        // Render transform gizmo if visible and a structure is selected
        if self.gizmo_settings.visible && self.selection_info.has_selection {
            // Use centroid for gizmo position (so it appears at the center of the geometry)
            // but keep the rotation and scale from the actual transform
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.selection_info.centroid),
                glam::Vec3::from(self.selection_info.rotation_degrees),
                glam::Vec3::from(self.selection_info.scale),
            );

            // Use Area instead of CentralPanel to avoid consuming all mouse events
            // The gizmo handles its own interaction detection
            egui::Area::new(egui::Id::new("gizmo_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .interactable(false) // Don't consume mouse events at the area level
                .show(&egui.context, |ui| {
                    // Set the clip rect to full window
                    ui.set_clip_rect(full_window_viewport);

                    // Skip gizmo interaction when pointer is over UI panel to prevent flickering
                    if pointer_over_ui {
                        return;
                    }

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.local_space,
                        full_window_viewport,
                    ) {
                        // Decompose the new transform from gizmo
                        let (new_gizmo_pos, new_rotation_deg, new_scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);

                        // Get old values
                        let old_translation = glam::Vec3::from(self.selection_info.translation);
                        let old_rotation_deg = glam::Vec3::from(self.selection_info.rotation_degrees);
                        let old_scale = glam::Vec3::from(self.selection_info.scale);
                        let world_centroid = glam::Vec3::from(self.selection_info.centroid);

                        // Compute local centroid (center of geometry in object space)
                        // world_centroid = translation + rotation * (local_centroid * scale)
                        // local_centroid = inverse(rotation) * (world_centroid - translation) / scale
                        let old_rotation = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            old_rotation_deg.x.to_radians(),
                            old_rotation_deg.y.to_radians(),
                            old_rotation_deg.z.to_radians(),
                        );
                        let local_centroid = old_rotation.inverse() * (world_centroid - old_translation) / old_scale;

                        // Convert new rotation to quaternion
                        let new_rotation = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            new_rotation_deg.x.to_radians(),
                            new_rotation_deg.y.to_radians(),
                            new_rotation_deg.z.to_radians(),
                        );

                        // Compute new translation to keep world_centroid fixed during rotation/scale
                        // For pure rotation/scale: new_translation = world_centroid - new_rotation * (local_centroid * new_scale)
                        // For translation: the gizmo moves, so we use the new gizmo position as the new world_centroid

                        // Check if the gizmo position changed (user translated)
                        let gizmo_moved = (new_gizmo_pos - world_centroid).length() > 0.0001;

                        let (new_translation, new_centroid) = if gizmo_moved {
                            // User translated: new world_centroid = new_gizmo_pos
                            let new_world_centroid = new_gizmo_pos;
                            let new_trans = new_world_centroid - new_rotation * (local_centroid * new_scale);
                            (new_trans, new_world_centroid)
                        } else {
                            // User rotated/scaled only: keep world_centroid fixed
                            let new_trans = world_centroid - new_rotation * (local_centroid * new_scale);
                            (new_trans, world_centroid)
                        };

                        // Update selection info
                        self.selection_info.translation = new_translation.into();
                        self.selection_info.centroid = new_centroid.into();
                        self.selection_info.rotation_degrees = new_rotation_deg.into();
                        self.selection_info.scale = new_scale.into();

                        // Apply to selected structure
                        crate::handle_gizmo_action(
                            polyscope_ui::GizmoAction::TransformChanged,
                            &self.gizmo_settings,
                            &self.selection_info,
                        );

                        // Immediately update GPU buffers so structure renders at new position this frame
                        crate::with_context(|ctx| {
                            if let Some((type_name, name)) = ctx.selected_structure() {
                                if let Some(structure) = ctx.registry.get(type_name, name) {
                                    if type_name == "PointCloud" {
                                        if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                            pc.update_gpu_buffers(&engine.queue, &engine.color_maps);
                                        }
                                    } else if type_name == "SurfaceMesh" {
                                        if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                            mesh.update_gpu_buffers(&engine.queue, &engine.color_maps);
                                        }
                                    } else if type_name == "CurveNetwork" {
                                        if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                            cn.update_gpu_buffers(&engine.queue, &engine.color_maps);
                                        }
                                    } else if type_name == "VolumeMesh" {
                                        if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                                            vm.update_gpu_buffers(&engine.queue);
                                        }
                                    }
                                }
                            }
                        });
                    }
                });
        }

        // Render slice plane gizmo if a slice plane is selected
        // Check if any slice plane is selected via UI
        self.slice_plane_selection = crate::get_slice_plane_selection_info();

        // Also sync selection from UI settings
        for settings in &self.slice_plane_settings {
            if settings.is_selected && settings.enabled && settings.draw_widget {
                if !self.slice_plane_selection.has_selection
                    || self.slice_plane_selection.name != settings.name
                {
                    crate::select_slice_plane_for_gizmo(&settings.name);
                    self.slice_plane_selection = crate::get_slice_plane_selection_info();
                }
            } else if !settings.is_selected
                && self.slice_plane_selection.has_selection
                && self.slice_plane_selection.name == settings.name
            {
                crate::deselect_slice_plane_gizmo();
                self.slice_plane_selection = crate::get_slice_plane_selection_info();
            }
        }

        if self.gizmo_settings.visible && self.slice_plane_selection.has_selection {
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.slice_plane_selection.origin),
                glam::Vec3::from(self.slice_plane_selection.rotation_degrees),
                glam::Vec3::ONE, // No scale for slice planes
            );

            egui::Area::new(egui::Id::new("slice_plane_gizmo_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .interactable(false)
                .show(&egui.context, |ui| {
                    ui.set_clip_rect(full_window_viewport);

                    if pointer_over_ui {
                        return;
                    }

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.local_space,
                        full_window_viewport,
                    ) {
                        let (new_origin, rotation, _scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);

                        self.slice_plane_selection.origin = new_origin.into();
                        self.slice_plane_selection.rotation_degrees = rotation.into();

                        // Apply to selected slice plane
                        crate::apply_slice_plane_gizmo_transform(
                            self.slice_plane_selection.origin,
                            self.slice_plane_selection.rotation_degrees,
                        );

                        // Update UI settings to reflect new position
                        for settings in &mut self.slice_plane_settings {
                            if settings.name == self.slice_plane_selection.name {
                                settings.origin = self.slice_plane_selection.origin;
                                // Normal is derived from transform, update it
                                let rotation = glam::Quat::from_euler(
                                    glam::EulerRot::XYZ,
                                    self.slice_plane_selection.rotation_degrees[0].to_radians(),
                                    self.slice_plane_selection.rotation_degrees[1].to_radians(),
                                    self.slice_plane_selection.rotation_degrees[2].to_radians(),
                                );
                                let normal = rotation * glam::Vec3::X;
                                settings.normal = normal.to_array();
                                break;
                            }
                        }
                    }
                });
        }

        // Update background color if changed
        self.background_color = Vec3::new(bg_color[0], bg_color[1], bg_color[2]);

        // Update ground plane settings from UI
        self.ground_plane.mode = match gp_mode {
            0 => GroundPlaneMode::None,
            1 => GroundPlaneMode::Tile,
            2 => GroundPlaneMode::ShadowOnly,
            _ => GroundPlaneMode::TileReflection,
        };
        self.ground_plane.height = gp_height;
        self.ground_plane.height_is_relative = gp_height_is_relative;
        self.ground_plane.shadow_blur_iters = gp_shadow_blur_iters;
        self.ground_plane.shadow_darkness = gp_shadow_darkness;
        self.ground_plane.reflection_intensity = gp_reflection_intensity;

        // Apply camera settings if changed
        if camera_changed {
            crate::apply_camera_settings(&mut engine.camera, &self.camera_settings);
        }

        // Apply scene extents settings if changed
        if scene_extents_changed {
            crate::set_auto_compute_extents(self.scene_extents.auto_compute);
        }

        // Queue screenshot request from UI button (will be processed after render)
        if screenshot_requested {
            let filename = format!("screenshot_{:04}.png", self.screenshot_counter);
            self.screenshot_counter += 1;
            self.screenshot_pending = Some(filename);
        }

        // End egui frame
        let egui_output = egui.end_frame(window);

        // Now borrow surface for rendering
        let surface = engine.surface.as_ref().expect("surface checked above");
        let output = match surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                engine.resize(engine.width, engine.height);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of memory");
                self.close_requested = true;
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Surface timeout");
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
                log::warn!("Surface error: other");
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        // HDR texture is always available for scene rendering
        // Update tone mapping uniforms - use passthrough values if disabled
        // Get SSAO settings from global options
        let ssao_enabled =
            polyscope_core::with_context(|ctx| ctx.options.ssao.enabled);
        if self.tone_mapping_settings.enabled {
            engine.update_tone_mapping(
                self.tone_mapping_settings.exposure,
                self.tone_mapping_settings.white_level,
                self.tone_mapping_settings.gamma,
                ssao_enabled,
            );
        } else {
            // Passthrough values: no exposure adjustment, linear transfer
            engine.update_tone_mapping(0.0, 1.0, 1.0, ssao_enabled);
        }

        // Store background color for use in render passes
        let bg_r = f64::from(self.background_color.x);
        let bg_g = f64::from(self.background_color.y);
        let bg_b = f64::from(self.background_color.z);

        // Store ground plane settings for later use
        let gp_enabled = self.ground_plane.mode != GroundPlaneMode::None;
        let gp_height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        // Shadow mode: 0=none (disabled), 1=shadow_only, 2=tile_with_shadow
        let gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile => 2u32,
            GroundPlaneMode::TileReflection => 2u32, // TileReflection also uses tile mode with shadows
        };

        // Compute pass for curve network tubes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Curve Network Tube Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(engine.curve_network_tube_compute_pipeline());

            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !structure.is_enabled() {
                        continue;
                    }
                    if structure.type_name() == "CurveNetwork" {
                        if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                            if cn.render_mode() == 1 {
                                // Tube mode
                                if let Some(render_data) = cn.render_data() {
                                    if let Some(compute_bg) = &render_data.compute_bind_group {
                                        compute_pass.set_bind_group(0, compute_bg, &[]);
                                        let num_workgroups = render_data.num_edges.div_ceil(64);
                                        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        // Shadow pass - render scene objects from light's perspective to shadow map
        if let (Some(shadow_pipeline), Some(shadow_map_pass)) =
            (engine.shadow_pipeline(), engine.shadow_map_pass())
        {
            // Compute light matrix from scene bounds
            let (scene_center, scene_radius) = crate::with_context(|ctx| {
                (ctx.center(), ctx.length_scale * 5.0)
            });
            let light_dir = glam::Vec3::new(0.5, -1.0, 0.3).normalize();
            let light_matrix = polyscope_render::ShadowMapPass::compute_light_matrix(
                scene_center,
                scene_radius,
                light_dir,
            );

            // Update light uniforms
            shadow_map_pass.update_light(&engine.queue, light_matrix, light_dir);

            // Begin shadow pass
            {
                let mut shadow_pass = shadow_map_pass.begin_shadow_pass(&mut encoder);
                shadow_pass.set_pipeline(shadow_pipeline);

                // Render shadow-casting structures (SurfaceMesh only for now)
                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(shadow_bg) = mesh.shadow_bind_group() {
                                    shadow_pass.set_bind_group(0, shadow_bg, &[]);
                                    if let Some(rd) = mesh.render_data() {
                                        shadow_pass.draw(0..rd.num_vertices(), 0..1);
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        // Render slice plane visualizations FIRST (before scene geometry)
        // This allows scene geometry to properly occlude the slice planes
        let (slice_planes, length_scale_for_planes) = crate::with_context(|ctx| {
            (ctx.slice_planes().cloned().collect::<Vec<_>>(), ctx.length_scale)
        });
        engine.render_slice_planes_with_clear(
            &mut encoder,
            &slice_planes,
            length_scale_for_planes,
            [bg_r as f32, bg_g as f32, bg_b as f32],
        );

        // Main render pass - always render scene to HDR texture
        // Get fresh reference to hdr_view after slice plane rendering
        let hdr_view = engine.hdr_view().expect("HDR texture should always be available");
        {
            // All scene content renders to HDR texture for consistent format
            let scene_view = hdr_view;

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load slice plane content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &engine.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load slice plane depth
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw point clouds
            if let Some(pipeline) = &engine.point_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(render_data) = pc.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    // 6 vertices per quad, num_points instances
                                    render_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
                        }
                    }
                });
            }

            // Draw vector quantities
            if let Some(pipeline) = &engine.vector_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(vq) = pc.active_vector_quantity() {
                                    if let Some(render_data) = vq.render_data() {
                                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                        // 8 segments * 6 vertices = 48 vertices per arrow
                                        render_pass.draw(0..48, 0..render_data.num_vectors);
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Note: Surface meshes and volume meshes are rendered in a separate pass
            // with MRT (multiple render targets) for SSAO normal output

            // Draw curve network edges (line mode) and camera views
            if let Some(pipeline) = &engine.curve_network_edge_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                // Only render in line mode (0)
                                if cn.render_mode() == 0 {
                                    if let Some(render_data) = cn.render_data() {
                                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                        // 2 vertices per edge (LineList topology)
                                        render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                    }
                                }
                            }
                        }
                        if structure.type_name() == "CameraView" {
                            if let Some(cv) = structure.as_any().downcast_ref::<CameraView>() {
                                if let Some(render_data) = cv.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    // 2 vertices per edge (LineList topology)
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "VolumeGrid" {
                            if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                                if let Some(render_data) = vg.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    // 2 vertices per edge (LineList topology)
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                    }
                });
            }

            // Draw curve network tubes (tube mode)
            if let Some(pipeline) = &engine.curve_network_tube_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                // Only render in tube mode (1)
                                if cn.render_mode() == 1 {
                                    if let Some(render_data) = cn.render_data() {
                                        if let (Some(tube_bg), Some(gen_vb)) = (
                                            &render_data.tube_render_bind_group,
                                            &render_data.generated_vertex_buffer,
                                        ) {
                                            render_pass.set_bind_group(0, tube_bg, &[]);
                                            render_pass.set_vertex_buffer(0, gen_vb.slice(..));
                                            // 36 vertices per edge (12 triangles for bounding box)
                                            render_pass.draw(0..render_data.num_edges * 36, 0..1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Draw curve network node spheres (tube mode - fills gaps at joints)
            if let Some(pipeline) = &engine.point_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                // Only render node spheres in tube mode (1)
                                if cn.render_mode() == 1 {
                                    if let Some(render_data) = cn.render_data() {
                                        if let Some(node_bg) = &render_data.node_render_bind_group {
                                            render_pass.set_bind_group(0, node_bg, &[]);
                                            // 6 vertices per quad, num_nodes instances
                                            render_pass.draw(0..6, 0..render_data.num_nodes);
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }  // End of main render pass scope

        // Surface mesh render pass with MRT (HDR color + normal G-buffer for SSAO)
        if let Some(pipeline) = &engine.mesh_pipeline {
            let hdr_view = engine.hdr_view().expect("HDR view should be available");
            let normal_view = engine.normal_view().expect("Normal view should be available");

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Mesh Pass"),
                color_attachments: &[
                    // Color output (HDR)
                    Some(wgpu::RenderPassColorAttachment {
                        view: hdr_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Preserve existing content
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    // Normal output (G-buffer for SSAO)
                    // Alpha=0 marks "no geometry" so SSAO skips ground plane/background
                    Some(wgpu::RenderPassColorAttachment {
                        view: normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.5, g: 0.5, b: 1.0, a: 0.0, // a=0 means no valid geometry
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &engine.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve depth from main pass
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

            // Check transparency mode: 2 = WeightedBlended (OIT)
            let use_oit = self.appearance_settings.transparency_mode == 2;

            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !structure.is_enabled() {
                        continue;
                    }
                    if structure.type_name() == "SurfaceMesh" {
                        if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                            // In OIT mode, ALL surface meshes go through OIT pass
                            // This avoids z-fighting on overlapping opaque geometry
                            if use_oit {
                                continue;
                            }
                            if let Some(render_data) = mesh.render_data() {
                                render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                render_pass.set_index_buffer(
                                    render_data.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                            }
                        }
                    }
                    if structure.type_name() == "VolumeMesh" {
                        if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                            // Render exterior faces (includes cell culling when slice plane is active)
                            if let Some(render_data) = vm.render_data() {
                                render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                render_pass.set_index_buffer(
                                    render_data.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                            }
                            // Note: No slice cap geometry needed - we use cell culling
                            // which shows whole cells instead of cross-section caps
                        }
                    }
                }
            });
        }

        // Render ground plane BEFORE OIT so transparent objects composite correctly over it
        // Get scene parameters for ground plane
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });

        // Ground plane and reflection rendering (before OIT so depth is available)
        if self.ground_plane.mode == GroundPlaneMode::TileReflection {
            // Compute ground height
            let ground_height = if self.ground_plane.height_is_relative {
                scene_min_y - length_scale * 0.001
            } else {
                self.ground_plane.height
            };

            // Update reflection uniforms
            let reflection_matrix = reflection::ground_reflection_matrix(ground_height);
            engine.update_reflection(
                reflection_matrix,
                self.ground_plane.reflection_intensity,
                ground_height,
            );

            // 1. Render stencil pass (mark ground plane region)
            engine.render_stencil_pass(
                &mut encoder,
                &view,
                ground_height,
                scene_center,
                length_scale,
            );

            // 2. Render ground plane FIRST (opaque base)
            engine.render_ground_plane(
                &mut encoder,
                &view,
                true, // enabled
                scene_center,
                scene_min_y,
                length_scale,
                gp_height_override,
                self.ground_plane.shadow_darkness,
                gp_shadow_mode,
                0.0, // No transparency - fully opaque ground
            );

            // 3. Render reflected meshes ON TOP of ground
            {
                let hdr_view = engine.hdr_texture_view().unwrap_or(&view);
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Reflected Geometry Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: hdr_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: engine.depth_view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Keep stencil from previous pass
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    ..Default::default()
                });

                // Render each visible surface mesh reflected
                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(mesh_data) = mesh.render_data() {
                                    if let Some(bind_group) =
                                        engine.create_reflected_mesh_bind_group(mesh_data)
                                    {
                                        engine.render_reflected_mesh(
                                            &mut render_pass,
                                            &bind_group,
                                            mesh_data.vertex_count(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }
        } else {
            // Non-reflection ground plane modes
            engine.render_ground_plane(
                &mut encoder,
                &view,
                gp_enabled,
                scene_center,
                scene_min_y,
                length_scale,
                gp_height_override,
                self.ground_plane.shadow_darkness,
                gp_shadow_mode,
                0.0,
            );
        }

        // OIT (Order-Independent Transparency) pass for surface meshes
        // All surface meshes go through OIT to handle overlapping geometry correctly
        if self.appearance_settings.transparency_mode == 2 {
            // Check if there are any surface meshes to render
            let has_surface_meshes = crate::with_context(|ctx| {
                ctx.registry.iter().any(|s| {
                    s.is_enabled() && s.type_name() == "SurfaceMesh"
                })
            });

            if has_surface_meshes {
                // Ensure OIT resources are initialized
                engine.ensure_oit_textures();
                engine.ensure_oit_pass();
                engine.ensure_mesh_oit_pipeline();

                let oit_accum_view = engine.oit_accum_view().unwrap();
                let oit_reveal_view = engine.oit_reveal_view().unwrap();
                let oit_pipeline = engine.mesh_oit_pipeline().unwrap();

                // OIT Accumulation Pass
                {
                    let mut oit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("OIT Accumulation Pass"),
                        color_attachments: &[
                            // Accumulation buffer (clear to black/zero)
                            Some(wgpu::RenderPassColorAttachment {
                                view: oit_accum_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            }),
                            // Reveal buffer (clear to 1.0 = fully transparent)
                            Some(wgpu::RenderPassColorAttachment {
                                view: oit_reveal_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            }),
                        ],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &engine.depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Keep opaque depth
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });

                    oit_pass.set_pipeline(oit_pipeline);
                    oit_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                    // Render only transparent meshes
                    crate::with_context(|ctx| {
                        for structure in ctx.registry.iter() {
                            if !structure.is_enabled() {
                                continue;
                            }
                            if structure.type_name() == "SurfaceMesh" {
                                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                    // Render ALL surface meshes through OIT
                                    // This handles both transparent and opaque meshes,
                                    // avoiding z-fighting on overlapping geometry
                                    if let Some(render_data) = mesh.render_data() {
                                        oit_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                        oit_pass.set_index_buffer(
                                            render_data.index_buffer.slice(..),
                                            wgpu::IndexFormat::Uint32,
                                        );
                                        oit_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                    }
                                }
                            }
                        }
                    });
                }

                // OIT Composite Pass - blend transparent result over opaque scene
                {
                    let hdr_view = engine.hdr_view().expect("HDR view should be available");
                    let oit_composite = engine.oit_composite_pass().unwrap();
                    let oit_bind_group = oit_composite.create_bind_group(
                        &engine.device,
                        oit_accum_view,
                        oit_reveal_view,
                    );

                    let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("OIT Composite Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: hdr_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Keep opaque content
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        ..Default::default()
                    });

                    oit_composite.draw(&mut composite_pass, &oit_bind_group);
                }
            }
        }

        // Render SSAO if enabled
        if ssao_enabled {
            polyscope_core::with_context(|ctx| {
                engine.render_ssao(&mut encoder, &ctx.options.ssao);
            });
        }

        // Apply tone mapping from HDR to surface (always runs, uses passthrough if disabled)
        engine.render_tone_mapping(&mut encoder, &view);

        // Render egui on top (directly to surface, after tone mapping)
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [engine.width, engine.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        egui.render(
            &engine.device,
            &engine.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
            egui_output,
        );

        engine.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Handle screenshot if pending (local request from F12 key)
        if let Some(filename) = self.screenshot_pending.take() {
            self.capture_screenshot(filename);
        }

        // Handle screenshot request from public API (screenshot() / screenshot_to_file())
        if let Some(request) = crate::take_screenshot_request() {
            let filename = request.filename.unwrap_or_else(|| {
                let name = format!("screenshot_{:04}.png", self.screenshot_counter);
                self.screenshot_counter += 1;
                name
            });
            self.capture_screenshot(filename);
        }
    }

    /// Captures a screenshot by re-rendering to a dedicated texture.
    fn capture_screenshot(&mut self, filename: String) {
        let Some(engine) = &mut self.engine else {
            log::error!("Cannot capture screenshot: engine not initialized");
            return;
        };

        // Create screenshot target
        let screenshot_view = engine.create_screenshot_target();

        // Re-render to screenshot texture
        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.background_color.x),
                            g: f64::from(self.background_color.y),
                            b: f64::from(self.background_color.z),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw point clouds
            if let Some(pipeline) = &engine.point_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(render_data) = pc.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
                        }
                    }
                });
            }

            // Draw vector quantities
            if let Some(pipeline) = &engine.vector_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(vq) = pc.active_vector_quantity() {
                                    if let Some(render_data) = vq.render_data() {
                                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                        render_pass.draw(0..48, 0..render_data.num_vectors);
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Draw surface meshes and volume meshes
            if let Some(pipeline) = &engine.mesh_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(render_data) = mesh.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.set_index_buffer(
                                        render_data.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "VolumeMesh" {
                            if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                                if let Some(render_data) = vm.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.set_index_buffer(
                                        render_data.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                }
                            }
                        }
                    }
                });
            }

            // Draw curve networks, camera views, and volume grids
            if let Some(pipeline) = &engine.curve_network_edge_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !structure.is_enabled() {
                            continue;
                        }
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                if let Some(render_data) = cn.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "CameraView" {
                            if let Some(cv) = structure.as_any().downcast_ref::<CameraView>() {
                                if let Some(render_data) = cv.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "VolumeGrid" {
                            if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                                if let Some(render_data) = vg.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                    }
                });
            }
        }

        // Render ground plane for screenshot
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });
        let height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        let screenshot_gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile => 2u32,
            GroundPlaneMode::TileReflection => 2u32,
        };
        let screenshot_reflection_intensity = if self.ground_plane.mode == GroundPlaneMode::TileReflection {
            self.ground_plane.reflection_intensity
        } else {
            0.0
        };
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            self.ground_plane.mode != GroundPlaneMode::None,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
            self.ground_plane.shadow_darkness,
            screenshot_gp_shadow_mode,
            screenshot_reflection_intensity,
        );

        // Apply tone mapping from HDR to final screenshot texture
        engine.apply_screenshot_tone_mapping(&mut encoder);

        engine.queue.submit(std::iter::once(encoder.finish()));

        // Capture the screenshot
        match engine.capture_screenshot() {
            Ok(data) => {
                let (width, height) = engine.dimensions();
                match polyscope_render::save_image(&filename, &data, width, height) {
                    Ok(()) => {
                        log::info!("Screenshot saved to {filename}");
                    }
                    Err(e) => {
                        log::error!("Failed to save screenshot: {e}");
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to capture screenshot: {e}");
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("polyscope-rs")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        // Create render engine
        let mut engine = RenderEngine::new_windowed(window.clone())
            .block_on()
            .expect("failed to create render engine");

        // Initialize GPU picking system
        engine.init_pick_buffers(engine.width, engine.height);

        // Create egui integration
        let egui = EguiIntegration::new(&engine.device, engine.surface_config.format, &window);

        self.window = Some(window);
        self.engine = Some(engine);
        self.egui = Some(egui);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // ALWAYS track physical mouse button state, even if egui consumes the event.
        // This prevents the mouse state from getting "stuck" when egui intercepts events.
        match &event {
            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.left_mouse_down = true;
                        self.last_click_pos = Some(self.mouse_pos);
                        self.drag_distance = 0.0;
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.left_mouse_down = false;
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.right_mouse_down = true;
                        self.drag_distance = 0.0;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.right_mouse_down = false;
                    }
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_down = modifiers.state().shift_key();
            }
            _ => {}
        }

        // Let egui handle events
        let egui_consumed = if let (Some(egui), Some(window)) = (&mut self.egui, &self.window) {
            egui.handle_event(window, &event)
        } else {
            false
        };

        // Check if egui is actively using the pointer (e.g., dragging a widget or gizmo)
        let egui_using_pointer = self
            .egui
            .as_ref()
            .is_some_and(|e| e.context.is_using_pointer());

        // Check if mouse is in the left UI panel area (approximately 305px wide + margin)
        // Only block events here, not in the 3D viewport
        const LEFT_PANEL_WIDTH: f64 = 320.0;
        let mouse_in_ui_panel = self.mouse_pos.0 <= LEFT_PANEL_WIDTH;

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                if let Some(engine) = &mut self.engine {
                    engine.resize(size.width, size.height);
                    // Resize pick buffers to match
                    engine.init_pick_buffers(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let delta_x = position.x - self.mouse_pos.0;
                let delta_y = position.y - self.mouse_pos.1;
                self.mouse_pos = (position.x, position.y);

                // Accumulate drag distance
                if self.left_mouse_down || self.right_mouse_down {
                    self.drag_distance += delta_x.abs() + delta_y.abs();
                }

                // Camera control:
                // - Left drag (no Shift): Rotate/orbit - only if egui isn't using pointer (gizmo)
                // - Left drag + Shift OR Right drag: Pan - right drag always works
                if let Some(engine) = &mut self.engine {
                    // Left drag rotation: blocked when gizmo is active
                    let is_rotate = self.left_mouse_down && !self.shift_down && !egui_using_pointer;
                    // Left+Shift pan: blocked when gizmo is active
                    let is_left_pan = self.left_mouse_down && self.shift_down && !egui_using_pointer;
                    // Right drag pan: always works regardless of selection/gizmo
                    let is_right_pan = self.right_mouse_down;

                    if is_rotate {
                        engine
                            .camera
                            .orbit(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                    } else if is_left_pan || is_right_pan {
                        let scale =
                            engine.camera.position.distance(engine.camera.target) * 0.002;
                        engine
                            .camera
                            .pan(-delta_x as f32 * scale, delta_y as f32 * scale);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // For UI panel clicks, let egui handle it exclusively
                if mouse_in_ui_panel && egui_consumed {
                    return;
                }

                // For 3D viewport clicks, we handle picking ourselves
                // (even if egui "consumed" due to gizmo overlay)

                // Threshold for distinguishing click from drag (in pixels)
                const DRAG_THRESHOLD: f64 = 5.0;

                if let (MouseButton::Left, ElementState::Released) = (button, state) {
                    // DEBUG: Log click event
                    log::debug!(
                        "[CLICK DEBUG] Left mouse released at ({:.1}, {:.1}), drag_distance={:.2}, mouse_in_ui_panel={}, egui_using_pointer={}, egui_consumed={}",
                        self.mouse_pos.0, self.mouse_pos.1, self.drag_distance, mouse_in_ui_panel, egui_using_pointer, egui_consumed
                    );

                    // Skip if egui is actively using pointer AND this was a drag (gizmo being dragged)
                    // But allow clicks through - egui_using_pointer can be true even for simple clicks
                    // when gizmo is visible, so we only skip if it was actually a drag operation
                    if egui_using_pointer && self.drag_distance >= DRAG_THRESHOLD {
                        log::debug!("[CLICK DEBUG] EARLY RETURN: egui was dragging (drag_distance={:.2})", self.drag_distance);
                        self.last_click_pos = None;
                        return;
                    }

                    // Check if this was a click (not a drag) in the 3D viewport
                    if !mouse_in_ui_panel && self.drag_distance < DRAG_THRESHOLD {
                        log::debug!("[CLICK DEBUG] Processing click in 3D viewport");
                        if let Some(engine) = &self.engine {
                            let click_screen = glam::Vec2::new(
                                self.mouse_pos.0 as f32,
                                self.mouse_pos.1 as f32,
                            );

                            let ray = self.screen_ray(
                                click_screen,
                                engine.width,
                                engine.height,
                                &engine.camera,
                            );
                            let Some((ray_origin, ray_dir)) = ray else {
                                log::debug!("[CLICK DEBUG] No ray - deselecting");
                                self.selection = None;
                                self.selected_element_index = None;
                                self.selection_info = polyscope_ui::SelectionInfo::default();
                                crate::deselect_structure();
                                self.deselect_slice_plane_selection();
                                self.last_click_pos = None;
                                return;
                            };

                            let plane_hit = self.pick_slice_plane_at_ray(ray_origin, ray_dir);
                            log::debug!("[CLICK DEBUG] plane_hit: {plane_hit:?}");

                            let plane_params = crate::with_context(|ctx| {
                                let mut enabled_planes: Vec<(String, Vec3, Vec3)> = ctx
                                    .slice_planes()
                                    .filter(|p| p.is_enabled())
                                    .map(|p| (p.name().to_string(), p.origin(), p.normal()))
                                    .collect();
                                enabled_planes.sort_by(|a, b| a.0.cmp(&b.0));
                                enabled_planes
                                    .into_iter()
                                    .map(|(_, origin, normal)| (origin, normal))
                                    .collect::<Vec<_>>()
                            });

                            let structure_hit =
                                self.pick_structure_at_ray(ray_origin, ray_dir, &plane_params);
                            log::debug!("[CLICK DEBUG] structure_hit: {structure_hit:?}");

                            // GPU picking for point clouds, refined with ray distance
                            let gpu_picked = self
                                .gpu_pick_at(self.mouse_pos.0 as u32, self.mouse_pos.1 as u32);
                            log::debug!("[CLICK DEBUG] gpu_picked: {gpu_picked:?}");
                            let point_hit = gpu_picked.and_then(|(type_name, name, idx)| {
                                if type_name == "PointCloud" {
                                    self.pick_point_cloud_at_ray(ray_origin, ray_dir, &name, idx)
                                        .map(|t| (name, idx, t))
                                } else {
                                    None
                                }
                            });
                            log::debug!("[CLICK DEBUG] point_hit: {point_hit:?}");

                            enum ClickHit {
                                Plane(String),
                                Structure { type_name: String, name: String, element_index: u32 },
                            }

                            let mut best_hit: Option<(ClickHit, f32)> = None;

                            if let Some((name, t)) = plane_hit {
                                best_hit = Some((ClickHit::Plane(name), t));
                            }

                            if let Some((type_name, name, t)) = structure_hit {
                                let is_better =
                                    best_hit.as_ref().map_or(true, |(_, best_t)| t < *best_t);
                                if is_better {
                                    best_hit = Some((
                                        ClickHit::Structure {
                                            type_name,
                                            name,
                                            element_index: 0,
                                        },
                                        t,
                                    ));
                                }
                            }

                            if let Some((name, idx, t)) = point_hit {
                                let is_better =
                                    best_hit.as_ref().map_or(true, |(_, best_t)| t < *best_t);
                                if is_better {
                                    best_hit = Some((
                                        ClickHit::Structure {
                                            type_name: "PointCloud".to_string(),
                                            name,
                                            element_index: idx,
                                        },
                                        t,
                                    ));
                                }
                            }

                            match &best_hit {
                                Some((ClickHit::Plane(plane_name), t)) => {
                                    log::debug!("[CLICK DEBUG] Hit plane '{plane_name}' at t={t}");
                                    // Select the slice plane and clear structure selection
                                    self.selection = None;
                                    self.selected_element_index = None;
                                    self.selection_info = polyscope_ui::SelectionInfo::default();
                                    crate::deselect_structure();
                                    self.select_slice_plane_by_name(plane_name);
                                }
                                Some((ClickHit::Structure { type_name, name, element_index }, t)) => {
                                    log::debug!("[CLICK DEBUG] Hit structure '{type_name}::{name}' element {element_index} at t={t}");
                                    self.selected_element_index = Some(*element_index);
                                    self.deselect_slice_plane_selection();

                                    let element_type = match type_name.as_str() {
                                        "PointCloud" => polyscope_render::PickElementType::Point,
                                        "SurfaceMesh" => polyscope_render::PickElementType::Face,
                                        "CurveNetwork" => polyscope_render::PickElementType::Edge,
                                        "VolumeMesh" => polyscope_render::PickElementType::Face,
                                        _ => polyscope_render::PickElementType::None,
                                    };

                                    self.selection = Some(PickResult {
                                        hit: true,
                                        structure_type: type_name.clone(),
                                        structure_name: name.clone(),
                                        element_index: u64::from(*element_index),
                                        element_type,
                                        screen_pos: click_screen,
                                        depth: 0.5,
                                    });
                                    crate::select_structure(type_name, name);
                                    self.selection_info = crate::get_selection_info();
                                }
                                None => {
                                    log::debug!("[CLICK DEBUG] No hit - DESELECTING");
                                    // Nothing was clicked - deselect
                                    self.selection = None;
                                    self.selected_element_index = None;
                                    self.selection_info = polyscope_ui::SelectionInfo::default();
                                    crate::deselect_structure();
                                    self.deselect_slice_plane_selection();
                                }
                            }
                        }
                    } else {
                        log::debug!(
                            "[CLICK DEBUG] SKIPPED: mouse_in_ui_panel={} or drag_distance={:.2} >= {}",
                            mouse_in_ui_panel, self.drag_distance, DRAG_THRESHOLD
                        );
                    }
                    self.last_click_pos = None;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Skip if in UI panel and egui consumed
                if mouse_in_ui_panel && egui_consumed {
                    return;
                }

                if let Some(engine) = &mut self.engine {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                    };
                    // Scale zoom delta based on projection mode
                    let scale = match engine.camera.projection_mode {
                        polyscope_render::ProjectionMode::Perspective => {
                            engine.camera.position.distance(engine.camera.target) * 0.1
                        }
                        polyscope_render::ProjectionMode::Orthographic => {
                            // For orthographic, scale based on current ortho_scale
                            engine.camera.ortho_scale * 0.5
                        }
                    };
                    engine.camera.zoom(scroll * scale);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) => {
                            self.close_requested = true;
                        }
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F12) => {
                            // Take screenshot with auto-generated filename
                            self.request_auto_screenshot();
                            log::info!("Screenshot requested (F12)");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if self.close_requested {
            event_loop.exit();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the polyscope application.
pub fn run_app() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("event loop error");
}
