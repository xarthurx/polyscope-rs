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
    mouse_pos: (f64, f64),
    mouse_down: bool,
    right_mouse_down: bool,
    // Selection state
    selection: Option<PickResult>,
    last_click_pos: Option<(f64, f64)>,
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
            mouse_down: false,
            right_mouse_down: false,
            selection: None,
            last_click_pos: None,
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

    /// Renders a single frame.
    fn render(&mut self) {
        let (Some(engine), Some(egui), Some(window)) =
            (&mut self.engine, &mut self.egui, &self.window)
        else {
            return;
        };

        let Some(surface) = &engine.surface else {
            return;
        };

        // Update camera uniforms
        engine.update_camera_uniforms();

        // Initialize GPU resources for any uninitialized point clouds and vector quantities
        crate::with_context_mut(|ctx| {
            for structure in ctx.registry.iter_mut() {
                if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                        // Initialize point cloud render data
                        if pc.render_data().is_none() {
                            pc.init_gpu_resources(
                                &engine.device,
                                engine.point_bind_group_layout(),
                                engine.camera_buffer(),
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
        };
        let mut gp_height = self.ground_plane.height;
        let mut gp_height_is_relative = self.ground_plane.height_is_relative;

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
            // Get camera matrices from engine
            let view_matrix = engine.camera.view_matrix();
            let projection_matrix = engine.camera.projection_matrix();

            // Get current transform as matrix
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.selection_info.translation),
                glam::Vec3::from(self.selection_info.rotation_degrees),
                glam::Vec3::from(self.selection_info.scale),
            );

            // Create a central panel for the gizmo overlay
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(&egui.context, |ui| {
                    let viewport = ui.clip_rect();

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.mode,
                        self.gizmo_settings.space,
                        viewport,
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
                    }
                });
        }

        // Update background color if changed
        self.background_color = Vec3::new(bg_color[0], bg_color[1], bg_color[2]);

        // Update ground plane settings from UI
        self.ground_plane.mode = match gp_mode {
            0 => GroundPlaneMode::None,
            _ => GroundPlaneMode::Tile,
        };
        self.ground_plane.height = gp_height;
        self.ground_plane.height_is_relative = gp_height_is_relative;

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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            // Draw curve network edges and camera views
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
                                    // 2 vertices per edge (LineList topology)
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
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
        }

        // Render ground plane (after scene, before UI)
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
        engine.render_ground_plane(
            &mut encoder,
            &view,
            self.ground_plane.mode == GroundPlaneMode::Tile,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
        );

        // Render egui on top
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
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            self.ground_plane.mode == GroundPlaneMode::Tile,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
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
        let engine = RenderEngine::new_windowed(window.clone())
            .block_on()
            .expect("failed to create render engine");

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
        // Let egui handle events first
        if let (Some(egui), Some(window)) = (&mut self.egui, &self.window) {
            if egui.handle_event(window, &event) {
                return; // egui consumed the event
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                if let Some(engine) = &mut self.engine {
                    engine.resize(size.width, size.height);
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

                if let Some(engine) = &mut self.engine {
                    if self.mouse_down {
                        // Orbit camera
                        engine
                            .camera
                            .orbit(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                    } else if self.right_mouse_down {
                        // Pan camera
                        let scale = engine.camera.position.distance(engine.camera.target) * 0.002;
                        engine
                            .camera
                            .pan(-delta_x as f32 * scale, delta_y as f32 * scale);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.mouse_down = true;
                        self.last_click_pos = Some(self.mouse_pos);
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        // Check if this was a click (not a drag)
                        if let Some(click_pos) = self.last_click_pos {
                            let dx = self.mouse_pos.0 - click_pos.0;
                            let dy = self.mouse_pos.1 - click_pos.1;
                            let drag_distance = (dx * dx + dy * dy).sqrt();

                            // If we didn't drag much, it's a click - do picking
                            if drag_distance < 5.0 {
                                // Placeholder selection - actual GPU picking can be added later
                                self.selection = Some(PickResult {
                                    hit: true,
                                    structure_type: "PointCloud".to_string(),
                                    structure_name: "test".to_string(),
                                    element_index: 42,
                                    element_type: polyscope_render::PickElementType::Point,
                                    screen_pos: glam::Vec2::new(
                                        self.mouse_pos.0 as f32,
                                        self.mouse_pos.1 as f32,
                                    ),
                                    depth: 0.5,
                                });
                            }
                        }
                        self.mouse_down = false;
                        self.last_click_pos = None;
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.right_mouse_down = true;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        // Clear selection on right click
                        self.selection = None;
                        self.right_mouse_down = false;
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
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
