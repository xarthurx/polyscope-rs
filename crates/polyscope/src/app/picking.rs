use std::time::{Duration, Instant};

use super::{App, CurveNetwork, PickResult, PointCloud, SurfaceMesh, Vec3, VolumeMesh};

/// Longest time between the two clicks of a double-click. C++ Polyscope uses
/// the Dear `ImGui` default `io.MouseDoubleClickTime` of 0.30 s.
pub(super) const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(300);

/// Longest distance in pixels between the two clicks of a double-click
/// (Dear `ImGui` default `io.MouseDoubleClickMaxDist`).
pub(super) const DOUBLE_CLICK_DIST: f64 = 6.0;

/// Time that a single-click pick waits before it changes the selection.
/// C++ uses the double-click time plus a 50 ms margin.
const PICK_DELAY: Duration = DOUBLE_CLICK_TIME.saturating_add(Duration::from_millis(50));

/// Distance in pixels from the press position at which a drag starts
/// (Dear `ImGui` default `io.MouseDragThreshold`).
const DRAG_START_DIST: f64 = 6.0;

/// Drag length below which a release is still a click. The length adds the
/// moves after the drag start, as fractions of the window size
/// (C++ `dragIgnoreThreshold`).
const DRAG_IGNORE_THRESHOLD: f64 = 0.01;

/// What a mouse-button release in the 3D view does (C++ `processInputEvents`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseAction {
    /// Nothing happens.
    Nothing,
    /// Queue a pick that changes the selection later.
    QueuePick,
    /// Set the view center at the cursor and cancel a queued pick.
    SetViewCenter,
    /// Clear the selection and cancel a queued pick.
    ClearSelection,
    /// Cancel a queued pick only.
    CancelPick,
}

/// Returns the action of a mouse-button release, as C++ Polyscope decides it.
///
/// - Left: a double-click or a Ctrl+Shift click sets the view center. A click
///   without Shift, Ctrl or Alt queues a pick.
/// - Right, without Shift, Ctrl or Alt: a click clears the selection. Each
///   release cancels a queued pick.
///
/// The end of a drag is not a click. On macOS, Cmd acts as Ctrl, as in Dear `ImGui`.
pub(crate) fn release_action(
    button: winit::event::MouseButton,
    is_click: bool,
    is_double_click: bool,
    modifiers: winit::keyboard::ModifiersState,
) -> ReleaseAction {
    use winit::event::MouseButton;
    let ctrl = if cfg!(target_os = "macos") {
        modifiers.super_key()
    } else {
        modifiers.control_key()
    };
    let shift = modifiers.shift_key();
    let any_modifier = shift || ctrl || modifiers.alt_key();
    match button {
        MouseButton::Left if is_click && (is_double_click || (ctrl && shift)) => {
            ReleaseAction::SetViewCenter
        }
        MouseButton::Left if is_click && !any_modifier => ReleaseAction::QueuePick,
        MouseButton::Right if !any_modifier && is_click => ReleaseAction::ClearSelection,
        MouseButton::Right if !any_modifier => ReleaseAction::CancelPick,
        _ => ReleaseAction::Nothing,
    }
}

/// The object that a single click in the 3D view hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClickHit {
    /// A slice plane, by name.
    Plane(String),
    /// An element of a structure.
    Structure {
        type_name: String,
        name: String,
        element_index: u32,
    },
}

/// A single-click pick that waits until the click cannot become a double-click.
#[derive(Debug, Clone)]
pub(crate) struct PendingPick {
    /// The hit, or `None` for the background.
    hit: Option<ClickHit>,
    /// Screen position of the click.
    screen_pos: glam::Vec2,
    /// Time of the click.
    time: Instant,
}

impl App {
    /// Starts the drag tracking of a mouse-button press at the cursor.
    pub(crate) fn begin_mouse_press(&mut self) {
        self.mouse_press_pos = Some(self.mouse_pos);
        self.mouse_dragging = false;
        self.drag_distance = 0.0;
    }

    /// Tracks a cursor move of `delta` pixels while a button is held, as C++
    /// Polyscope does with Dear `ImGui`.
    ///
    /// A drag starts when the cursor is `DRAG_START_DIST` pixels from the press
    /// position. After that, each move adds |dx| / width + |dy| / height.
    pub(crate) fn track_mouse_drag(&mut self, delta: (f64, f64), window_size: (u32, u32)) {
        if !self.mouse_dragging {
            let Some(press) = self.mouse_press_pos else {
                return;
            };
            let from_press = (self.mouse_pos.0 - press.0).hypot(self.mouse_pos.1 - press.1);
            if from_press < DRAG_START_DIST {
                return;
            }
            self.mouse_dragging = true;
        }
        let width = f64::from(window_size.0.max(1));
        let height = f64::from(window_size.1.max(1));
        self.drag_distance += delta.0.abs() / width + delta.1.abs() / height;
    }

    /// Returns whether a release now is a click, not the end of a drag.
    pub(crate) fn release_is_click(&self) -> bool {
        self.drag_distance < DRAG_IGNORE_THRESHOLD
    }

    /// Clears the selection and cancels a queued pick.
    pub(crate) fn clear_selection(&mut self) {
        self.cancel_pending_pick();
        self.apply_click_hit(None, glam::Vec2::ZERO);
    }

    /// Queues the pick of a single click (C++ commit 022deea).
    ///
    /// The pick changes the selection only when a double-click is no longer
    /// possible, so the first click of a double-click does not change it.
    pub(crate) fn queue_click_pick(&mut self, hit: Option<ClickHit>, screen_pos: glam::Vec2) {
        self.pending_pick = Some(PendingPick {
            hit,
            screen_pos,
            time: Instant::now(),
        });
    }

    /// Cancels the queued pick, for example on a double-click.
    pub(crate) fn cancel_pending_pick(&mut self) {
        self.pending_pick = None;
    }

    /// Applies the queued pick when the pick delay has passed at `now`.
    ///
    /// As in C++, the pick applies at once when a selection exists already.
    pub(crate) fn update_pending_pick(&mut self, now: Instant) {
        let Some(pending) = &self.pending_pick else {
            return;
        };
        if self.selection.is_none() && now.saturating_duration_since(pending.time) < PICK_DELAY {
            return;
        }
        if let Some(pending) = self.pending_pick.take() {
            self.apply_click_hit(pending.hit, pending.screen_pos);
        }
    }

    /// Changes the selection to the hit of a click. `None` clears the selection.
    fn apply_click_hit(&mut self, hit: Option<ClickHit>, click_screen: glam::Vec2) {
        // The hit object can disappear while the pick waits.
        let hit = hit.filter(|hit| {
            crate::with_context(|ctx| match hit {
                ClickHit::Plane(name) => ctx.has_slice_plane(name),
                ClickHit::Structure {
                    type_name, name, ..
                } => {
                    // A VolumeGrid pick name is "gridname/quantityname".
                    let structure_name = if type_name == "VolumeGrid" {
                        name.split('/').next().unwrap_or(name)
                    } else {
                        name
                    };
                    ctx.registry.get(type_name, structure_name).is_some()
                }
            })
        });

        match hit {
            Some(ClickHit::Plane(plane_name)) => {
                log::debug!("[CLICK DEBUG] Hit plane '{plane_name}'");
                // Select the slice plane and clear structure selection
                self.selection = None;
                self.selected_element_index = None;
                self.selection_info = polyscope_ui::SelectionInfo::default();
                crate::deselect_structure();
                self.select_slice_plane_by_name(&plane_name);
            }
            Some(ClickHit::Structure {
                type_name,
                name,
                element_index,
            }) => {
                log::debug!(
                    "[CLICK DEBUG] Hit structure '{type_name}::{name}' element {element_index}"
                );
                self.selected_element_index = Some(element_index);
                self.deselect_slice_plane_selection();

                let element_type = match type_name.as_str() {
                    "PointCloud" => polyscope_render::PickElementType::Point,
                    "SurfaceMesh" => polyscope_render::PickElementType::Face,
                    "VolumeMesh" => polyscope_render::PickElementType::Cell,
                    "CurveNetwork" => polyscope_render::PickElementType::Edge,
                    "VolumeGrid" => polyscope_render::PickElementType::Vertex,
                    _ => polyscope_render::PickElementType::None,
                };

                // For VolumeGrid, pick name is "gridname/quantityname"
                // Extract the grid name for structure selection
                let structure_name = if type_name == "VolumeGrid" {
                    name.split('/').next().unwrap_or(&name).to_string()
                } else {
                    name.clone()
                };

                self.selection = Some(PickResult {
                    hit: true,
                    structure_type: type_name.clone(),
                    structure_name: structure_name.clone(),
                    element_index: u64::from(element_index),
                    element_type,
                    screen_pos: click_screen,
                    depth: 0.5,
                });
                crate::select_structure(&type_name, &structure_name);
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

    /// Performs GPU-based picking to find which structure and element is at the given screen position.
    ///
    /// Uses the GPU pick buffer to determine the exact structure and element at the click position.
    /// Returns (`type_name`, name, `element_index`) or None if clicking on empty space.
    pub(super) fn gpu_pick_at(&self, x: u32, y: u32) -> Option<(String, String, u32)> {
        let engine = self.engine.as_ref()?;

        // Read pick buffer — returns flat 24-bit global index
        let global_index = engine.pick_at(x, y)?;

        // Background check (index 0 means nothing was hit)
        if global_index == 0 {
            return None;
        }

        // Look up structure info from global index
        let (type_name, name, local_index) = engine.lookup_global_index(global_index)?;
        Some((type_name.to_string(), name.to_string(), local_index))
    }

    /// Returns the world position of the structure at a screen position.
    ///
    /// Like C++ `pickAtScreenCoords`, this reads the pick buffer for the hit and the
    /// pick depth buffer for the position. Thus it works for every structure type
    /// in the pick pass. Returns None for the background.
    pub(super) fn pick_world_position_at(&self, screen: glam::Vec2) -> Option<Vec3> {
        let engine = self.engine.as_ref()?;
        if screen.x < 0.0 || screen.y < 0.0 {
            return None;
        }
        let (x, y) = (screen.x as u32, screen.y as u32);
        self.gpu_pick_at(x, y)?;
        let depth = engine.pick_depth_at(x, y)?;
        engine
            .camera
            .screen_depth_to_world(screen, depth, engine.width, engine.height)
    }

    pub(super) fn screen_ray(
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
    pub(super) fn pick_slice_plane_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
    ) -> Option<(String, f32)> {
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
                let up = if normal.dot(Vec3::Y).abs() < 0.99 {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let y_axis = up.cross(normal).normalize();
                let z_axis = normal.cross(y_axis).normalize();

                let local = hit - plane.origin();
                let y = local.dot(y_axis);
                let z = local.dot(z_axis);
                let size = plane.plane_size();

                if y.abs() <= size && z.abs() <= size {
                    let is_better = best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
                    if is_better {
                        best_hit = Some((plane.name().to_string(), t));
                    }
                }
            }
        });

        best_hit
    }

    pub(super) fn ray_intersect_triangle(
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
        if t > eps { Some(t) } else { None }
    }

    pub(super) fn ray_segment_closest_t(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        a: Vec3,
        b: Vec3,
    ) -> Option<f32> {
        let v = b - a;
        let c = v.dot(v);
        if c < 1e-12 {
            let t = ray_dir.dot(a - ray_origin);
            return (t >= 0.0).then_some(t);
        }

        let w0 = ray_origin - a;
        let a_dot = ray_dir.dot(ray_dir);
        let b_dot = ray_dir.dot(v);
        let d = ray_dir.dot(w0);
        let e = v.dot(w0);
        let denom = a_dot * c - b_dot * b_dot;

        let s;
        let mut t;
        if denom.abs() < 1e-8 {
            s = 0.0;
            t = ray_dir.dot(a - ray_origin);
        } else {
            s = (b_dot * d - a_dot * e) / denom;
            t = (b_dot * e - c * d) / denom;
        }

        if s < 0.0 {
            t = ray_dir.dot(a - ray_origin);
        } else if s > 1.0 {
            t = ray_dir.dot(b - ray_origin);
        }

        (t >= 0.0).then_some(t)
    }

    pub(super) fn pick_structure_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        plane_params: &[(Vec3, Vec3)],
    ) -> Option<(String, String, f32)> {
        let mut best_hit: Option<(String, String, f32)> = None;

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
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
                            if let Some(t) =
                                self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2)
                            {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better =
                                best_hit.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit = Some((
                                    structure.type_name().to_string(),
                                    structure.name().to_string(),
                                    t,
                                ));
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
                            if let Some(t) =
                                self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2)
                            {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better =
                                best_hit.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit = Some((
                                    structure.type_name().to_string(),
                                    structure.name().to_string(),
                                    t,
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        best_hit
    }

    pub(super) fn pick_point_cloud_at_ray(
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

    pub(super) fn pick_curve_network_edge_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        name: &str,
        element_index: u32,
    ) -> Option<f32> {
        crate::with_context(|ctx| {
            let structure = ctx.registry.get("CurveNetwork", name)?;
            let cn = structure.as_any().downcast_ref::<CurveNetwork>()?;
            let edge_idx = element_index as usize;
            if edge_idx >= cn.edge_tail_inds().len() {
                return None;
            }
            let tail_idx = cn.edge_tail_inds()[edge_idx] as usize;
            let tip_idx = cn.edge_tip_inds()[edge_idx] as usize;
            if tail_idx >= cn.nodes().len() || tip_idx >= cn.nodes().len() {
                return None;
            }

            let model = structure.transform();
            let tail = (model * cn.nodes()[tail_idx].extend(1.0)).truncate();
            let tip = (model * cn.nodes()[tip_idx].extend(1.0)).truncate();

            self.ray_segment_closest_t(ray_origin, ray_dir, tail, tip)
        })
    }

    pub(super) fn select_slice_plane_by_name(&mut self, name: &str) {
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

    pub(super) fn deselect_slice_plane_selection(&mut self) {
        for settings in &mut self.slice_plane_settings {
            settings.is_selected = false;
        }
        crate::deselect_slice_plane_gizmo();
    }
}
