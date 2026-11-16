use super::picking::{
    ClickHit, DOUBLE_CLICK_DIST, DOUBLE_CLICK_TIME, ReleaseAction, release_action,
};
use super::{
    ActiveEventLoop, App, ApplicationHandler, Arc, ControlFlow, EguiIntegration, ElementState,
    FutureExt, KeyCode, LogicalSize, MouseButton, RenderEngine, Vec3, Window, WindowEvent,
    WindowId,
};

impl App {
    /// Requests the next redraw after a frame that started at `frame_start`.
    ///
    /// With `Options::max_fps` = 0 the next frame starts at once. Otherwise
    /// the event loop waits until the next frame is due. It keeps handling
    /// input events while it waits.
    fn schedule_next_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        frame_start: std::time::Instant,
    ) {
        let max_fps = crate::with_context(|ctx| ctx.options.max_fps);
        if max_fps == 0 {
            self.next_frame_time = None;
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }
        let next = frame_start + std::time::Duration::from_secs_f64(1.0 / f64::from(max_fps));
        self.next_frame_time = Some(next);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(next) = self.next_frame_time else {
            return;
        };
        if std::time::Instant::now() >= next {
            self.next_frame_time = None;
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        }
    }

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
            .unwrap_or_else(|e| panic!("failed to create the render engine: {e}"));

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
            WindowEvent::MouseInput { state, button, .. } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => {
                    self.left_mouse_down = true;
                    self.last_click_pos = Some(self.mouse_pos);
                    self.begin_mouse_press();
                    self.press_in_ui_panel = self.cursor_over_ui_panel();
                }
                (MouseButton::Left, ElementState::Released) => {
                    self.left_mouse_down = false;
                }
                (MouseButton::Right, ElementState::Pressed) => {
                    self.right_mouse_down = true;
                    self.begin_mouse_press();
                    self.press_in_ui_panel = self.cursor_over_ui_panel();
                }
                (MouseButton::Right, ElementState::Released) => {
                    self.right_mouse_down = false;
                }
                _ => {}
            },
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_down = modifiers.state().shift_key();
                self.modifiers = modifiers.state();
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
            .is_some_and(|e| e.context.egui_is_using_pointer());

        // Check if mouse is in a UI panel area.
        // Uses the dynamic panel widths (updated each frame from actual egui panel rects)
        // instead of a hardcoded constant, so it stays accurate when a panel resizes
        // (e.g., when groups are created/expanded or the user drags the panel edge).
        let mouse_in_ui_panel = self.cursor_over_ui_panel();

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
                // Per-frame first-person WASD movement
                let now = std::time::Instant::now();
                if let Some(last) = self.last_frame_time {
                    let dt = now.duration_since(last).as_secs_f32();
                    if let Some(engine) = &mut self.engine
                        && engine.camera.navigation_style
                            == polyscope_render::NavigationStyle::FirstPerson
                        && !self.keys_down.is_empty()
                    {
                        let mut delta = Vec3::ZERO;
                        if self.keys_down.contains(&KeyCode::KeyA) {
                            delta.x -= 1.0; // strafe left
                        }
                        if self.keys_down.contains(&KeyCode::KeyD) {
                            delta.x += 1.0; // strafe right
                        }
                        if self.keys_down.contains(&KeyCode::KeyQ) {
                            delta.y += 1.0; // rise
                        }
                        if self.keys_down.contains(&KeyCode::KeyE) {
                            delta.y -= 1.0; // descend
                        }
                        if self.keys_down.contains(&KeyCode::KeyW) {
                            delta.z += 1.0; // forward
                        }
                        if self.keys_down.contains(&KeyCode::KeyS) {
                            delta.z -= 1.0; // backward
                        }
                        if delta.length_squared() > 0.0 {
                            engine.camera.cancel_flight();
                            // The step is relative to the view, so it suits any scene scale
                            let step = engine.camera.first_person_step(dt);
                            engine.camera.move_first_person(delta.normalize() * step);
                        }
                    }
                }
                self.last_frame_time = Some(now);

                // Apply a queued click pick once it cannot become a double-click
                self.update_pending_pick(now);

                self.render();
                self.schedule_next_frame(event_loop, now);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let delta_x = position.x - self.mouse_pos.0;
                let delta_y = position.y - self.mouse_pos.1;
                self.mouse_pos = (position.x, position.y);

                // Accumulate drag distance (C++ Polyscope drag rules)
                if self.left_mouse_down || self.right_mouse_down {
                    let window_size = self
                        .window
                        .as_ref()
                        .map_or((1, 1), |w| (w.inner_size().width, w.inner_size().height));
                    self.track_mouse_drag((delta_x, delta_y), window_size);
                }

                // Camera control:
                // - Left drag (no Shift): Rotate/orbit - only if egui isn't using pointer (gizmo)
                // - Left drag + Shift OR Right drag: Pan - right drag always works
                if let Some(engine) = &mut self.engine {
                    use polyscope_render::NavigationStyle;

                    let nav = engine.camera.navigation_style;

                    // A drag that starts over the left UI panel does not move the
                    // camera. This also applies to widgets that do not sense drags.
                    let camera_drag = !self.press_in_ui_panel;
                    // Left drag rotation: blocked when gizmo is active
                    let is_rotate = self.left_mouse_down
                        && !self.shift_down
                        && !egui_using_pointer
                        && camera_drag;
                    // Left+Shift pan: blocked when gizmo is active
                    let is_left_pan = self.left_mouse_down
                        && self.shift_down
                        && !egui_using_pointer
                        && camera_drag;
                    // Right drag pan: works regardless of selection/gizmo
                    let is_right_pan = self.right_mouse_down && camera_drag;

                    if is_rotate || is_left_pan || is_right_pan {
                        engine.camera.cancel_flight();
                    }

                    if is_rotate {
                        match nav {
                            NavigationStyle::Turntable => {
                                engine
                                    .camera
                                    .orbit_turntable(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                            }
                            NavigationStyle::Free => {
                                engine
                                    .camera
                                    .orbit_free(delta_x as f32 * 0.01, delta_y as f32 * 0.01);
                            }
                            NavigationStyle::Arcball => {
                                // Arcball needs normalized screen coordinates [-1, 1]
                                if let Some(window) = &self.window {
                                    let size = window.inner_size();
                                    let w = size.width as f32;
                                    let h = size.height as f32;
                                    let prev_x = (self.mouse_pos.0 - delta_x) as f32;
                                    let prev_y = (self.mouse_pos.1 - delta_y) as f32;
                                    let cur_x = self.mouse_pos.0 as f32;
                                    let cur_y = self.mouse_pos.1 as f32;

                                    let start =
                                        [(prev_x / w) * 2.0 - 1.0, -((prev_y / h) * 2.0 - 1.0)];
                                    let end = [(cur_x / w) * 2.0 - 1.0, -((cur_y / h) * 2.0 - 1.0)];
                                    engine.camera.orbit_arcball(start, end);
                                }
                            }
                            NavigationStyle::FirstPerson => {
                                engine
                                    .camera
                                    .mouse_look(delta_x as f32 * 0.005, delta_y as f32 * 0.005);
                            }
                            NavigationStyle::Planar | NavigationStyle::None => {
                                // No rotation in these modes
                            }
                        }
                    } else if is_left_pan || is_right_pan {
                        match nav {
                            NavigationStyle::None | NavigationStyle::FirstPerson => {}
                            _ => {
                                let scale =
                                    engine.camera.position.distance(engine.camera.target) * 0.002;
                                engine
                                    .camera
                                    .pan(-delta_x as f32 * scale, delta_y as f32 * scale);
                            }
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

                // A release is a click only if it does not end a drag (C++ drag rules)
                let is_click = self.release_is_click();

                // A right-button release clears the selection or cancels a queued pick
                if let (MouseButton::Right, ElementState::Released) = (button, state)
                    && !mouse_in_ui_panel
                {
                    match release_action(MouseButton::Right, is_click, false, self.modifiers) {
                        ReleaseAction::ClearSelection => self.clear_selection(),
                        ReleaseAction::CancelPick => self.cancel_pending_pick(),
                        _ => {}
                    }
                }

                if let (MouseButton::Left, ElementState::Released) = (button, state) {
                    // DEBUG: Log click event
                    log::debug!(
                        "[CLICK DEBUG] Left mouse released at ({:.1}, {:.1}), drag_distance={:.2}, mouse_in_ui_panel={}, egui_using_pointer={}, egui_consumed={}",
                        self.mouse_pos.0,
                        self.mouse_pos.1,
                        self.drag_distance,
                        mouse_in_ui_panel,
                        egui_using_pointer,
                        egui_consumed
                    );

                    // Skip if egui is actively using pointer AND this was a drag (gizmo being dragged)
                    // But allow clicks through - egui_using_pointer can be true even for simple clicks
                    // when gizmo is visible, so we only skip if it was actually a drag operation
                    if egui_using_pointer && !is_click {
                        log::debug!(
                            "[CLICK DEBUG] EARLY RETURN: egui was dragging (drag_distance={:.2})",
                            self.drag_distance
                        );
                        self.last_click_pos = None;
                        return;
                    }

                    // Double-click detection
                    let is_double_click = if let (Some(prev_time), Some(prev_pos)) =
                        (self.last_left_click_time, self.last_left_click_screen_pos)
                    {
                        let elapsed = prev_time.elapsed();
                        let dist = ((self.mouse_pos.0 - prev_pos.0).powi(2)
                            + (self.mouse_pos.1 - prev_pos.1).powi(2))
                        .sqrt();
                        elapsed < DOUBLE_CLICK_TIME && dist < DOUBLE_CLICK_DIST
                    } else {
                        false
                    };

                    // Record this click for next double-click check
                    if is_click {
                        self.last_left_click_time = Some(std::time::Instant::now());
                        self.last_left_click_screen_pos = Some(self.mouse_pos);
                    } else {
                        // Drags reset double-click tracking
                        self.last_left_click_time = None;
                        self.last_left_click_screen_pos = None;
                    }

                    let action = if mouse_in_ui_panel {
                        ReleaseAction::Nothing
                    } else {
                        release_action(MouseButton::Left, is_click, is_double_click, self.modifiers)
                    };

                    // Double-click or Ctrl+Shift click: set view center to clicked 3D point
                    if action == ReleaseAction::SetViewCenter {
                        // Cancel the pick of the first click
                        self.cancel_pending_pick();

                        // The pick buffers give the hit point for every structure type.
                        let click_screen =
                            glam::Vec2::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
                        if let Some(hit_point) = self.pick_world_position_at(click_screen)
                            && let Some(engine) = &mut self.engine
                        {
                            engine.camera.set_view_center(hit_point);
                            log::info!(
                                "Double-click: set view center to ({:.3}, {:.3}, {:.3})",
                                hit_point.x,
                                hit_point.y,
                                hit_point.z
                            );
                        }

                        // Reset double-click state so triple-click doesn't re-trigger
                        self.last_left_click_time = None;
                        self.last_left_click_screen_pos = None;
                        self.last_click_pos = None;
                        return;
                    }

                    // A click without modifier keys in the 3D viewport picks
                    if action == ReleaseAction::QueuePick {
                        log::debug!("[CLICK DEBUG] Processing click in 3D viewport");
                        if let Some(engine) = &self.engine {
                            let click_screen =
                                glam::Vec2::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);

                            let ray = self.screen_ray(
                                click_screen,
                                engine.width,
                                engine.height,
                                &engine.camera,
                            );
                            let Some((ray_origin, ray_dir)) = ray else {
                                log::debug!("[CLICK DEBUG] No ray - deselecting");
                                self.queue_click_pick(None, click_screen);
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
                            let gpu_picked =
                                self.gpu_pick_at(self.mouse_pos.0 as u32, self.mouse_pos.1 as u32);
                            log::debug!("[CLICK DEBUG] gpu_picked: {gpu_picked:?}");
                            let mut point_hit: Option<(String, u32, f32)> = None;
                            let mut curve_hit: Option<(String, u32, f32)> = None;
                            let mut mesh_hit: Option<(String, u32, f32)> = None;
                            // Filter GPU picks by group visibility
                            let gpu_picked = gpu_picked.filter(|(type_name, name, _)| {
                                crate::with_context(|ctx| {
                                    ctx.is_structure_visible_in_groups(type_name, name)
                                })
                            });
                            if let Some((type_name, name, idx)) = gpu_picked {
                                if type_name == "PointCloud" {
                                    point_hit = self
                                        .pick_point_cloud_at_ray(ray_origin, ray_dir, &name, idx)
                                        .map(|t| (name, idx, t));
                                } else if type_name == "CurveNetwork" {
                                    curve_hit = self
                                        .pick_curve_network_edge_at_ray(
                                            ray_origin, ray_dir, &name, idx,
                                        )
                                        .map(|t| (name, idx, t));
                                } else if type_name == "SurfaceMesh" {
                                    // GPU pick gives face index — validate with ray hit
                                    if let Some((_, _, t)) = &structure_hit {
                                        mesh_hit = Some((name, idx, *t));
                                    } else {
                                        // GPU picked a mesh but ray didn't hit — use a default depth
                                        mesh_hit = Some((name, idx, 1.0));
                                    }
                                }
                            }
                            log::debug!("[CLICK DEBUG] point_hit: {point_hit:?}");
                            log::debug!("[CLICK DEBUG] curve_hit: {curve_hit:?}");

                            let mut best_hit: Option<(ClickHit, f32)> = None;

                            if let Some((name, t)) = plane_hit {
                                best_hit = Some((ClickHit::Plane(name), t));
                            }

                            if let Some((type_name, name, t)) = structure_hit {
                                let is_better =
                                    best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
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
                                    best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
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

                            if let Some((name, idx, t)) = curve_hit {
                                let is_better =
                                    best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
                                if is_better {
                                    best_hit = Some((
                                        ClickHit::Structure {
                                            type_name: "CurveNetwork".to_string(),
                                            name,
                                            element_index: idx,
                                        },
                                        t,
                                    ));
                                }
                            }

                            if let Some((name, idx, t)) = mesh_hit {
                                let is_better =
                                    best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
                                if is_better {
                                    best_hit = Some((
                                        ClickHit::Structure {
                                            type_name: "SurfaceMesh".to_string(),
                                            name,
                                            element_index: idx,
                                        },
                                        t,
                                    ));
                                }
                            }

                            log::debug!("[CLICK DEBUG] best_hit: {best_hit:?}");
                            // The first click of a double-click must not change the
                            // selection, so the pick waits (C++ commit 022deea).
                            self.queue_click_pick(best_hit.map(|(hit, _)| hit), click_screen);
                        }
                    } else {
                        log::debug!(
                            "[CLICK DEBUG] SKIPPED: mouse_in_ui_panel={} drag_distance={:.4} modifiers={:?}",
                            mouse_in_ui_panel,
                            self.drag_distance,
                            self.modifiers
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
                    use polyscope_render::NavigationStyle;
                    let nav = engine.camera.navigation_style;

                    // Zoom disabled for None and FirstPerson modes
                    if nav != NavigationStyle::None && nav != NavigationStyle::FirstPerson {
                        engine.camera.cancel_flight();
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                        };
                        // The zoom step is relative to the view (C++ processZoom).
                        engine.camera.zoom_scroll(scroll);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Track WASD/QE keys for first-person movement
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys_down.insert(code);
                            // Handle special keys
                            match code {
                                KeyCode::Escape => {
                                    self.close_requested = true;
                                }
                                KeyCode::F12 => {
                                    self.request_auto_screenshot();
                                    log::info!("Screenshot requested (F12)");
                                }
                                _ => {}
                            }
                        }
                        ElementState::Released => {
                            self.keys_down.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::DroppedFile(path) => {
                log::info!("File dropped: {}", path.display());
                crate::with_context_mut(|ctx| {
                    if let Some(callback) = &mut ctx.file_drop_callback {
                        callback(&[path]);
                    }
                });
            }
            _ => {}
        }

        if self.close_requested {
            event_loop.exit();
        }
    }
}
