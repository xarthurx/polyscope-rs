use std::time::{Duration, Instant};

use super::{
    App, CurveNetwork, PickResult, PointCloud, Structure, SurfaceMesh, Vec3, VolumeGrid, VolumeMesh,
};
use polyscope_render::PickElementType;

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

/// Returns true if a cursor at `cursor_x` is over a UI panel.
///
/// The left panel covers `0..=left_width`. The selection panel covers the last
/// `right_width` of the window; it is hidden when `right_width` is 0. All
/// values are in logical pixels.
pub(crate) fn ui_panel_contains(
    cursor_x: f64,
    window_width: f64,
    left_width: f64,
    right_width: f64,
) -> bool {
    cursor_x <= left_width || (right_width > 0.0 && cursor_x >= window_width - right_width)
}

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
        element_type: PickElementType,
    },
}

/// An element in the pick buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PickBufferHit {
    /// Structure type name, for example `PointCloud`.
    pub type_name: String,
    /// Structure name.
    pub name: String,
    /// The index of the quantity that owns the pick range, for the gridcubes
    /// of a volume grid quantity.
    pub quantity_index: Option<usize>,
    /// The index of the element in its structure or quantity.
    pub element_index: u32,
}

/// The camera and the image size of the frame that the pick buffer shows.
///
/// Input events between frames can move the live camera. A click uses this
/// copy for its ray and for its pick depth, so they match the pick buffer.
#[derive(Debug, Clone)]
pub(crate) struct PickView {
    pub camera: polyscope_render::Camera,
    pub width: u32,
    pub height: u32,
}

impl PickView {
    /// Returns the current camera and image size of an engine.
    pub(crate) fn of(engine: &polyscope_render::RenderEngine) -> Self {
        Self {
            camera: engine.camera.clone(),
            width: engine.width,
            height: engine.height,
        }
    }
}

/// Returns whether a structure still has the element of a pick hit.
///
/// The structure can change between the frame of the pick buffer and the
/// delayed selection, for example when a point cloud gets fewer points.
fn element_exists(structure: &dyn Structure, element_type: PickElementType, index: u32) -> bool {
    let any = structure.as_any();
    let count = if let Some(pc) = any.downcast_ref::<PointCloud>() {
        pc.points().len()
    } else if let Some(mesh) = any.downcast_ref::<SurfaceMesh>() {
        mesh.num_faces()
    } else if let Some(cn) = any.downcast_ref::<CurveNetwork>() {
        cn.num_edges()
    } else if let Some(vm) = any.downcast_ref::<VolumeMesh>() {
        vm.num_cells()
    } else if let Some(vg) = any.downcast_ref::<VolumeGrid>() {
        let nodes = vg.node_dim();
        let dim = if element_type == PickElementType::Cell {
            nodes.saturating_sub(glam::UVec3::ONE)
        } else {
            nodes
        };
        dim.x as usize * dim.y as usize * dim.z as usize
    } else {
        // A camera view is selected as a whole: its wireframe always has the
        // same edges.
        return true;
    };
    (index as usize) < count
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
    /// Returns true if the cursor is over the left panel or the selection panel.
    ///
    /// The panel widths are in logical pixels. The cursor position and the
    /// window width are in physical pixels.
    pub(crate) fn cursor_over_ui_panel(&self) -> bool {
        let scale = self
            .egui
            .as_ref()
            .map_or(1.0, |egui| f64::from(egui.context.pixels_per_point()));
        let cursor_x = self.mouse_pos.0 / scale;
        let window_width = self
            .engine
            .as_ref()
            .map_or(f64::INFINITY, |engine| f64::from(engine.width) / scale);
        ui_panel_contains(
            cursor_x,
            window_width,
            self.left_panel_width,
            self.right_panel_width,
        )
    }

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
                // A pick reports the structure, also for a volume grid quantity.
                // The element must still exist.
                ClickHit::Structure {
                    type_name,
                    name,
                    element_index,
                    element_type,
                } => ctx
                    .registry
                    .get(type_name, name)
                    .is_some_and(|s| element_exists(s, *element_type, *element_index)),
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
                element_type,
            }) => {
                log::debug!(
                    "[CLICK DEBUG] Hit structure '{type_name}::{name}' {element_type:?} {element_index}"
                );
                self.selected_element_index = Some(element_index);
                self.deselect_slice_plane_selection();

                let structure_name = name;

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

    /// Returns the element in the pick buffer at a pixel, or None for the
    /// background.
    pub(super) fn gpu_pick_at(&self, x: u32, y: u32) -> Option<PickBufferHit> {
        let engine = self.engine.as_ref()?;

        // Read pick buffer — returns flat 24-bit global index
        let global_index = engine.pick_at(x, y)?;

        // Background check (index 0 means nothing was hit)
        if global_index == 0 {
            return None;
        }

        let owner = engine.lookup_pick_owner(global_index)?;
        Some(PickBufferHit {
            type_name: owner.type_name.to_owned(),
            name: owner.structure_name.to_owned(),
            quantity_index: owner.quantity_index,
            element_index: owner.local_index,
        })
    }

    /// Returns the view of the frame that the pick buffer shows, or the live
    /// view before the first pick pass.
    fn click_view(&self) -> Option<PickView> {
        self.pick_view
            .clone()
            .or_else(|| self.engine.as_ref().map(PickView::of))
    }

    /// Returns what a single click at a screen position hits: the element in
    /// the pick buffer under the cursor, or a slice plane in front of it.
    ///
    /// The pick buffer gives the structure and the element for every
    /// structure type. The pick buffer, the pick depth and the slice plane
    /// test all use the center of the pixel under the cursor, as the
    /// rasterizer does, and the camera of the frame of the pick buffer. Only a
    /// slice plane under the cursor needs the distance of the element, so a
    /// click without one reads no depth.
    pub(crate) fn click_hit_at(&self, screen: glam::Vec2) -> Option<ClickHit> {
        let view = self.click_view()?;
        if screen.x < 0.0 || screen.y < 0.0 {
            return None;
        }
        let pixel = screen.floor() + glam::Vec2::splat(0.5);
        let element = self.pick_buffer_hit_at(pixel.x as u32, pixel.y as u32);
        let ray = self.screen_ray(pixel, view.width, view.height, &view.camera);
        let plane = ray.and_then(|(origin, dir)| self.pick_slice_plane_at_ray(origin, dir));
        log::debug!("[CLICK DEBUG] element: {element:?}, plane: {plane:?}");
        match (plane, element, ray) {
            (None, element, _) => element,
            // The element wins only if it is nearer than the slice plane.
            (Some((plane_name, plane_t)), Some(element), Some((origin, dir))) => {
                let nearer = self
                    .pick_depth_distance(&view, pixel, origin, dir)
                    .is_some_and(|t| t < plane_t);
                Some(if nearer {
                    element
                } else {
                    ClickHit::Plane(plane_name)
                })
            }
            (Some((plane_name, _)), _, _) => Some(ClickHit::Plane(plane_name)),
        }
    }

    /// Returns the element in the pick buffer at a pixel.
    ///
    /// Returns None for the background, and for a structure that is hidden or
    /// removed after the frame of the pick buffer.
    fn pick_buffer_hit_at(&self, x: u32, y: u32) -> Option<ClickHit> {
        let hit = self.gpu_pick_at(x, y)?;
        let visible = crate::with_context(|ctx| {
            ctx.registry
                .get(&hit.type_name, &hit.name)
                .is_some_and(|structure| ctx.is_structure_visible(structure))
        });
        if !visible {
            return None;
        }
        let element_type = self.pick_element_type(&hit)?;
        Some(ClickHit::Structure {
            type_name: hit.type_name,
            name: hit.name,
            element_index: hit.element_index,
            element_type,
        })
    }

    /// Returns the element type of an element in the pick buffer, or None for
    /// a gridcube range that is not in the last pick pass.
    ///
    /// A gridcube gets the type that the last pick pass recorded: a cell of a
    /// cell quantity, or a node of a node quantity, which the selection shows
    /// as a vertex.
    fn pick_element_type(&self, hit: &PickBufferHit) -> Option<PickElementType> {
        Some(match hit.type_name.as_str() {
            "PointCloud" => PickElementType::Point,
            "SurfaceMesh" => PickElementType::Face,
            "VolumeMesh" => PickElementType::Cell,
            "CurveNetwork" | "CameraView" => PickElementType::Edge,
            "VolumeGrid" => {
                let key = (hit.name.clone(), hit.quantity_index?);
                *self.grid_pick_element_types.get(&key)?
            }
            _ => PickElementType::None,
        })
    }

    /// Returns the distance along the click ray of the surface in the pick
    /// depth buffer at a pixel center. The scene draws with the same depth, so
    /// the distance decides what the window shows in front.
    fn pick_depth_distance(
        &self,
        view: &PickView,
        pixel: glam::Vec2,
        ray_origin: Vec3,
        ray_dir: Vec3,
    ) -> Option<f32> {
        let engine = self.engine.as_ref()?;
        let depth = engine.pick_depth_at(pixel.x as u32, pixel.y as u32)?;
        let position = view
            .camera
            .screen_depth_to_world(pixel, depth, view.width, view.height)?;
        Some((position - ray_origin).dot(ray_dir))
    }

    /// Returns the world position of the structure at a screen position.
    ///
    /// Like C++ `pickAtScreenCoords`, this reads the pick buffer for the hit and the
    /// pick depth buffer for the position. Thus it works for every structure type
    /// in the pick pass. Returns None for the background. The position comes
    /// from the camera of the frame of the pick buffer.
    pub(super) fn pick_world_position_at(&self, screen: glam::Vec2) -> Option<Vec3> {
        let engine = self.engine.as_ref()?;
        let view = self.click_view()?;
        if screen.x < 0.0 || screen.y < 0.0 {
            return None;
        }
        let (x, y) = (screen.x as u32, screen.y as u32);
        self.gpu_pick_at(x, y)?;
        let depth = engine.pick_depth_at(x, y)?;
        view.camera
            .screen_depth_to_world(screen, depth, view.width, view.height)
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
