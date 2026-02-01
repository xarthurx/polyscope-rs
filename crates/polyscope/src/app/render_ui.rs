//! egui UI integration: panels, gizmos, settings synchronization.

use super::{App, PointCloud, SurfaceMesh, CurveNetwork, CameraView, VolumeGrid, VolumeMesh, Vec3, GroundPlaneMode};

/// Result of building the UI for one frame.
pub(super) struct UiResult {
    pub egui_output: egui::FullOutput,
}

impl App {
    /// Build the egui UI for one frame. Engine and egui must be passed separately
    /// (via Option::take) to avoid borrow checker issues with self.
    pub(super) fn build_ui(
        &mut self,
        engine: &mut polyscope_render::RenderEngine,
        egui: &mut polyscope_ui::EguiIntegration,
        window: &std::sync::Arc<winit::window::Window>,
    ) -> UiResult {
        // Multi-pass egui layout: egui's Grid widget makes itself invisible on its
        // first frame (sizing pass) and calls ctx.request_discard() expecting a second
        // pass. We loop to handle this, preventing a one-frame blink when panels open.
        let max_egui_passes: u32 = 2;
        let mut egui_output = egui::FullOutput::default();

        // Build UI (declare mutable state before the loop so it persists across passes)
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
        let mut reset_view_requested = false;
        let mut ssaa_changed = false;

        for egui_pass in 0..max_egui_passes {
        if egui_pass == 0 {
            egui.begin_frame(window);
        } else {
            egui.begin_rerun_pass();
            // Reset mutable flags for the re-run pass
            camera_changed = false;
            scene_extents_changed = false;
            screenshot_requested = false;
            reset_view_requested = false;
            ssaa_changed = false;
        }

        let panel_width = polyscope_ui::build_left_panel(&egui.context, |ui| {
            let view_action = polyscope_ui::build_controls_section(ui, &mut bg_color);
            match view_action {
                polyscope_ui::ViewAction::Screenshot => {
                    screenshot_requested = true;
                }
                polyscope_ui::ViewAction::ResetView => {
                    reset_view_requested = true;
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
                    ctx.options.ssaa_factor = self.appearance_settings.ssaa_factor;
                });

                // Mark SSAA as changed (will apply outside closure)
                ssaa_changed = true;
            }

            // Tone mapping settings panel
            polyscope_ui::panels::build_tone_mapping_section(ui, &mut self.tone_mapping_settings);

            // Material loading section
            let material_action = polyscope_ui::panels::build_material_section(
                ui,
                &mut self.material_load_state,
            );
            match material_action {
                polyscope_ui::MaterialAction::LoadStatic { name, path } => {
                    match engine.load_static_material(&name, &path) {
                        Ok(()) => {
                            self.material_load_state.status = format!("Loaded static material '{name}'");
                        }
                        Err(e) => {
                            self.material_load_state.status = format!("Error: {e}");
                        }
                    }
                }
                polyscope_ui::MaterialAction::LoadBlendable { name, base_path, extension } => {
                    let filenames = [
                        format!("{base_path}_r{extension}"),
                        format!("{base_path}_g{extension}"),
                        format!("{base_path}_b{extension}"),
                        format!("{base_path}_k{extension}"),
                    ];
                    let refs: [&str; 4] = [&filenames[0], &filenames[1], &filenames[2], &filenames[3]];
                    match engine.load_blendable_material(&name, refs) {
                        Ok(()) => {
                            self.material_load_state.status = format!("Loaded blendable material '{name}'");
                        }
                        Err(e) => {
                            self.material_load_state.status = format!("Error: {e}");
                        }
                    }
                }
                polyscope_ui::MaterialAction::None => {}
            }

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

            // Groups section (only shown if groups were created via API)
            let groups_action = polyscope_ui::panels::build_groups_section(
                ui,
                &mut self.group_settings,
            );
            if groups_action != polyscope_ui::GroupsAction::None {
                crate::handle_group_action(groups_action, &mut self.group_settings);
            }

            // Collect colormap names for VolumeGrid UI
            let colormap_names: Vec<String> = engine.color_maps.names().map(String::from).collect();
            let colormap_name_refs: Vec<&str> = colormap_names.iter().map(String::as_str).collect();

            // Collect material names for structure UI (built-in + custom)
            let available_materials: Vec<&str> = engine.materials.names();

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
                                    pc.build_egui_ui(ui, &available_materials);
                                }
                            }
                            if type_name == "SurfaceMesh" {
                                if let Some(mesh) = s.as_any_mut().downcast_mut::<SurfaceMesh>() {
                                    mesh.build_egui_ui(ui, &available_materials);
                                }
                            }
                            if type_name == "CurveNetwork" {
                                if let Some(cn) = s.as_any_mut().downcast_mut::<CurveNetwork>() {
                                    cn.build_egui_ui(ui, &available_materials);
                                }
                            }
                            if type_name == "CameraView" {
                                if let Some(cv) = s.as_any_mut().downcast_mut::<CameraView>() {
                                    cv.build_egui_ui(ui);
                                }
                            }
                            if type_name == "VolumeGrid" {
                                if let Some(vg) = s.as_any_mut().downcast_mut::<VolumeGrid>() {
                                    vg.build_egui_ui(ui, &colormap_name_refs);
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
        // Update dynamic panel width (with small margin to account for resize handle)
        self.left_panel_width = f64::from(panel_width) + 5.0;

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
        let panel_w = self.left_panel_width as f32;
        let pointer_over_ui = egui.context.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|pos| pos.x <= panel_w)
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
                        let old_rotation_deg =
                            glam::Vec3::from(self.selection_info.rotation_degrees);
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
                        let local_centroid =
                            old_rotation.inverse() * (world_centroid - old_translation) / old_scale;

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
                            let new_trans =
                                new_world_centroid - new_rotation * (local_centroid * new_scale);
                            (new_trans, new_world_centroid)
                        } else {
                            // User rotated/scaled only: keep world_centroid fixed
                            let new_trans =
                                world_centroid - new_rotation * (local_centroid * new_scale);
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
                                        if let Some(pc) =
                                            structure.as_any().downcast_ref::<PointCloud>()
                                        {
                                            pc.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "SurfaceMesh" {
                                        if let Some(mesh) =
                                            structure.as_any().downcast_ref::<SurfaceMesh>()
                                        {
                                            mesh.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "CurveNetwork" {
                                        if let Some(cn) =
                                            structure.as_any().downcast_ref::<CurveNetwork>()
                                        {
                                            cn.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "VolumeMesh" {
                                        if let Some(vm) =
                                            structure.as_any().downcast_ref::<VolumeMesh>()
                                        {
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
            // When auto-compute is OFF, sync manual edits back to context
            if !self.scene_extents.auto_compute {
                polyscope_core::state::with_context_mut(|ctx| {
                    ctx.length_scale = self.scene_extents.length_scale;
                    ctx.bounding_box = (
                        glam::Vec3::from_array(self.scene_extents.bbox_min),
                        glam::Vec3::from_array(self.scene_extents.bbox_max),
                    );
                });
            }
        }

        // Apply SSAA settings if changed
        if ssaa_changed && engine.ssaa_factor() != self.appearance_settings.ssaa_factor {
            engine.set_ssaa_factor(self.appearance_settings.ssaa_factor);
        }

        // Queue screenshot request from UI button (will be processed after render)
        if screenshot_requested {
            let filename = format!("screenshot_{:04}.png", self.screenshot_counter);
            self.screenshot_counter += 1;
            self.screenshot_pending = Some(filename);
        }

        // Reset camera to home view (matching C++ Polyscope's resetCameraToHomeView)
        if reset_view_requested {
            let bbox = crate::with_context(|ctx| ctx.bounding_box);
            let (min, max) = bbox;
            if min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0 {
                engine.camera.look_at_box(min, max);
                engine.camera.fov = std::f32::consts::FRAC_PI_4; // Reset FOV to default 45°
            }
        }

        // End egui pass and check for discard request (multi-pass layout)
        let pass_output = egui.end_pass();
        egui_output.append(pass_output);

        if !egui_output.platform_output.requested_discard() {
            break;
        }
        // Clear discard reasons before the next pass
        egui_output.platform_output.request_discard_reasons.clear();
        } // end multi-pass egui loop

        // Handle platform output (clipboard, cursor, etc.) once after all passes
        egui.handle_platform_output(window, &egui_output.platform_output);

        UiResult {
            egui_output,
        }
    }
}
