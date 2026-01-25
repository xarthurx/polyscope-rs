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

use polyscope_core::{GroundPlaneConfig, GroundPlaneMode};
use polyscope_render::{PickResult, RenderEngine};
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
    // Visual gizmo
    transform_gizmo: polyscope_ui::TransformGizmo,
    // Tone mapping settings
    tone_mapping_settings: polyscope_ui::ToneMappingSettings,
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
            transform_gizmo: polyscope_ui::TransformGizmo::new(),
            tone_mapping_settings: polyscope_ui::ToneMappingSettings::default(),
        }
    }

    /// Requests a screenshot to be saved to the specified file.
    pub fn request_screenshot(&mut self, filename: String) {
        self.screenshot_pending = Some(filename);
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
    /// Returns (type_name, name, element_index) or None if clicking on empty space.
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
        Some((type_name.to_string(), name.to_string(), elem_id as u32))
    }

    /// Performs screen-space picking to find which structure (if any) is at the given screen position.
    ///
    /// Projects sample points from each structure to screen space and checks if the click
    /// is within a threshold distance. Returns the (type_name, name) of the clicked structure,
    /// or None if clicking on empty space.
    ///
    /// NOTE: This is the fallback method. GPU picking (gpu_pick_at) is preferred when available.
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
        let pick_threshold = 20.0_f32;
        let mut best_match: Option<(String, String, f32)> = None;

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !structure.is_enabled() {
                    continue;
                }

                let model = structure.transform();
                let mvp = view_proj * model;

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

        // Update camera uniforms
        engine.update_camera_uniforms();

        // Initialize GPU resources for any uninitialized point clouds and vector quantities
        crate::with_context_mut(|ctx| {
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
                        let needs_tube = cn.render_data().map_or(false, |rd| !rd.has_tube_resources());
                        let needs_node = cn.render_data().map_or(false, |rd| !rd.has_node_render_resources());

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
                        if vm.render_data().is_none() {
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
                        mesh.update_gpu_buffers(&engine.queue);
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

        polyscope_ui::build_left_panel(&egui.context, |ui| {
            polyscope_ui::build_controls_section(ui, &mut bg_color);

            // Camera settings panel
            if polyscope_ui::build_camera_settings_section(ui, &mut self.camera_settings) {
                camera_changed = true;
            }

            // Scene extents panel
            if polyscope_ui::build_scene_extents_section(ui, &mut self.scene_extents) {
                scene_extents_changed = true;
            }

            // Appearance settings panel
            polyscope_ui::build_appearance_section(ui, &mut self.appearance_settings);

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

        // Render transform gizmo if visible and something is selected
        if self.gizmo_settings.visible && self.selection_info.has_selection {
            // Get camera matrices from engine - MUST match what's used for 3D rendering
            let view_matrix = engine.camera.view_matrix();
            let projection_matrix = engine.camera.projection_matrix();

            // Get current transform as matrix
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.selection_info.translation),
                glam::Vec3::from(self.selection_info.rotation_degrees),
                glam::Vec3::from(self.selection_info.scale),
            );

            // Use full window viewport to match 3D rendering projection
            // The 3D scene is rendered with full window aspect ratio, so the gizmo
            // must use the same viewport for correct alignment
            let full_window_viewport = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(engine.width as f32, engine.height as f32),
            );

            // Use Area instead of CentralPanel to avoid consuming all mouse events
            // The gizmo handles its own interaction detection
            egui::Area::new(egui::Id::new("gizmo_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .interactable(false) // Don't consume mouse events at the area level
                .show(&egui.context, |ui| {
                    // Set the clip rect to full window
                    ui.set_clip_rect(full_window_viewport);

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.mode,
                        self.gizmo_settings.space,
                        full_window_viewport,
                    ) {
                        // Decompose and update selection info
                        let (translation, rotation, scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);
                        self.selection_info.translation = translation.into();
                        self.selection_info.rotation_degrees = rotation.into();
                        self.selection_info.scale = scale.into();

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
                                            mesh.update_gpu_buffers(&engine.queue);
                                        }
                                    }
                                }
                            }
                        });
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
        let hdr_view = engine.hdr_view().expect("HDR texture should always be available");
        if self.tone_mapping_settings.enabled {
            engine.update_tone_mapping(
                self.tone_mapping_settings.exposure,
                self.tone_mapping_settings.white_level,
                self.tone_mapping_settings.gamma,
            );
        } else {
            // Passthrough values: no exposure adjustment, linear transfer
            engine.update_tone_mapping(0.0, 1.0, 1.0);
        }

        // Store background color for use in render passes
        let bg_r = self.background_color.x as f64;
        let bg_g = self.background_color.y as f64;
        let bg_b = self.background_color.z as f64;

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
                                        let num_workgroups = (render_data.num_edges + 63) / 64;
                                        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        // Main render pass - always render scene to HDR texture
        {
            // All scene content renders to HDR texture for consistent format
            let scene_view = hdr_view;

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg_r,
                            g: bg_g,
                            b: bg_b,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &engine.depth_view,
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

            // Draw surface meshes and volume meshes
            if let Some(pipeline) = &engine.mesh_pipeline {
                render_pass.set_pipeline(pipeline);

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

            // Draw curve network edges (line mode) and camera views
            if let Some(pipeline) = &engine.curve_network_edge_pipeline {
                render_pass.set_pipeline(pipeline);

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

        // Render ground plane to HDR texture (before tone mapping)
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });

        // Ground plane renders to HDR internally (surface_view passed for fallback)
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
        );

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

        // Handle screenshot if pending
        if let Some(filename) = self.screenshot_pending.take() {
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
                            r: self.background_color.x as f64,
                            g: self.background_color.y as f64,
                            b: self.background_color.z as f64,
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
        );

        engine.queue.submit(std::iter::once(encoder.finish()));

        // Capture the screenshot
        match engine.capture_screenshot() {
            Ok(data) => {
                let (width, height) = engine.dimensions();
                match polyscope_render::save_image(&filename, &data, width, height) {
                    Ok(()) => {
                        log::info!("Screenshot saved to {}", filename);
                    }
                    Err(e) => {
                        log::error!("Failed to save screenshot: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to capture screenshot: {}", e);
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

                // Camera control: only if egui isn't using the pointer
                // Controls (matching C++ Polyscope):
                // - Left drag (no Shift): Rotate/orbit
                // - Left drag + Shift OR Right drag: Pan
                if !egui_using_pointer {
                    if let Some(engine) = &mut self.engine {
                        let is_rotate = self.left_mouse_down && !self.shift_down;
                        let is_pan = (self.left_mouse_down && self.shift_down) || self.right_mouse_down;

                        if is_rotate {
                            engine
                                .camera
                                .orbit(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                        } else if is_pan {
                            let scale =
                                engine.camera.position.distance(engine.camera.target) * 0.002;
                            engine
                                .camera
                                .pan(-delta_x as f32 * scale, delta_y as f32 * scale);
                        }
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

                match (button, state) {
                    (MouseButton::Left, ElementState::Released) => {
                        // Skip if egui is actively using pointer (gizmo being dragged)
                        if egui_using_pointer {
                            self.last_click_pos = None;
                            return;
                        }

                        // Check if this was a click (not a drag) in the 3D viewport
                        if !mouse_in_ui_panel && self.drag_distance < DRAG_THRESHOLD {
                            if let Some(engine) = &self.engine {
                                let click_screen = glam::Vec2::new(
                                    self.mouse_pos.0 as f32,
                                    self.mouse_pos.1 as f32,
                                );

                                // Try GPU picking first (pixel-perfect when pick pass is rendered)
                                let gpu_picked = self.gpu_pick_at(
                                    self.mouse_pos.0 as u32,
                                    self.mouse_pos.1 as u32,
                                );

                                // Fall back to screen-space picking if GPU pick misses
                                let picked = gpu_picked.or_else(|| {
                                    self.pick_structure_at_screen_pos(
                                        click_screen,
                                        engine.width,
                                        engine.height,
                                        &engine.camera,
                                    )
                                    .map(|(t, n)| (t, n, 0u32))
                                });

                                if let Some((type_name, name, element_index)) = picked {
                                    // Something was clicked - select it
                                    self.selected_element_index = Some(element_index);

                                    // Determine element type based on structure type
                                    let element_type = match type_name.as_str() {
                                        "PointCloud" => polyscope_render::PickElementType::Point,
                                        "SurfaceMesh" => polyscope_render::PickElementType::Face,
                                        "CurveNetwork" => polyscope_render::PickElementType::Edge,
                                        _ => polyscope_render::PickElementType::None,
                                    };

                                    self.selection = Some(PickResult {
                                        hit: true,
                                        structure_type: type_name.clone(),
                                        structure_name: name.clone(),
                                        element_index: element_index as u64,
                                        element_type,
                                        screen_pos: click_screen,
                                        depth: 0.5,
                                    });
                                    crate::select_structure(&type_name, &name);
                                    self.selection_info = crate::get_selection_info();
                                } else {
                                    // Nothing was clicked - deselect
                                    self.selection = None;
                                    self.selected_element_index = None;
                                    self.selection_info = polyscope_ui::SelectionInfo::default();
                                    crate::deselect_structure();
                                }
                            }
                        }
                        self.last_click_pos = None;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        // Right-click (not drag) in 3D viewport clears selection
                        if !mouse_in_ui_panel && self.drag_distance < DRAG_THRESHOLD {
                            self.selection = None;
                            self.selected_element_index = None;
                            self.selection_info = polyscope_ui::SelectionInfo::default();
                            crate::deselect_structure();
                        }
                    }
                    _ => {}
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
                    let scale = engine.camera.position.distance(engine.camera.target) * 0.1;
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
