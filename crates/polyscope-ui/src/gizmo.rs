//! Visual 3D gizmo integration using transform-gizmo-egui.
//!
//! The gizmo shows the handles of the selected mode (translate, rotate or
//! scale) and snaps to the configured increments.

use egui::Ui;
use glam::{DMat4, Mat4, Quat, Vec3};
use polyscope_core::GizmoMode as PolyscopeGizmoMode;
use transform_gizmo_egui::{
    EnumSet, Gizmo, GizmoConfig, GizmoInteraction, GizmoMode, GizmoOrientation, GizmoVisuals,
    config::TransformPivotPoint, math::Transform, mint,
};

use crate::GizmoSettings;

/// Handle, orientation and snap configuration of the gizmo for one interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ModeConfig {
    modes: EnumSet<GizmoMode>,
    orientation: GizmoOrientation,
    snapping: bool,
    /// Rotation increment in radians.
    snap_angle: f32,
    snap_distance: f32,
    snap_scale: f32,
}

impl ModeConfig {
    /// Builds the configuration from the UI settings.
    ///
    /// If `allow_scale` is false (slice planes), the scale mode shows the
    /// translation and rotation handles. Snapping is on only if every shown
    /// handle kind has a positive increment.
    fn new(settings: &GizmoSettings, allow_scale: bool) -> Self {
        let translate = (GizmoMode::all_translate(), settings.snap_translate);
        let rotate = (GizmoMode::all_rotate(), settings.snap_rotate);
        let scale = (GizmoMode::all_scale(), settings.snap_scale);
        let kinds: &[(EnumSet<GizmoMode>, f32)] = match settings.mode {
            PolyscopeGizmoMode::Translate => &[translate],
            PolyscopeGizmoMode::Rotate => &[rotate],
            PolyscopeGizmoMode::Scale if allow_scale => &[scale],
            PolyscopeGizmoMode::Scale => &[translate, rotate],
        };
        Self {
            modes: kinds.iter().fold(EnumSet::empty(), |acc, (m, _)| acc | *m),
            orientation: if settings.local_space {
                GizmoOrientation::Local
            } else {
                GizmoOrientation::Global
            },
            snapping: kinds.iter().all(|(_, snap)| *snap > 0.0),
            snap_angle: settings.snap_rotate.to_radians(),
            snap_distance: settings.snap_translate,
            snap_scale: settings.snap_scale,
        }
    }
}

/// Wrapper around transform-gizmo-egui for polyscope integration.
pub struct TransformGizmo {
    gizmo: Gizmo,
    /// True while the user drags a handle.
    dragging: bool,
}

impl Default for TransformGizmo {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformGizmo {
    /// Creates a new transform gizmo.
    #[must_use]
    pub fn new() -> Self {
        Self {
            gizmo: Gizmo::default(),
            dragging: false,
        }
    }

    /// Returns an egui area to show a gizmo over the 3D view.
    ///
    /// The area must be interactable: egui 0.36 leaves the widgets of a
    /// non-interactable area out of the hit test. The area itself senses
    /// only hover, so it takes no clicks.
    pub fn overlay_area(id: egui::Id) -> egui::Area {
        egui::Area::new(id)
            .interactable(true)
            .sense(egui::Sense::hover())
    }

    /// Draws the gizmo and handles interaction.
    ///
    /// Shows the handles of the mode in `settings` and snaps to its increments.
    /// Returns the updated transform if the gizmo was manipulated.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `view_matrix` - Camera view matrix
    /// * `projection_matrix` - Camera projection matrix
    /// * `model_matrix` - Current transform of the object
    /// * `settings` - Mode, coordinate space and snap increments
    /// * `viewport` - Viewport rect
    pub fn interact(
        &mut self,
        ui: &mut Ui,
        view_matrix: Mat4,
        projection_matrix: Mat4,
        model_matrix: Mat4,
        settings: &GizmoSettings,
        viewport: egui::Rect,
    ) -> Option<Mat4> {
        let mode_config = ModeConfig::new(settings, true);
        self.interact_with(
            ui,
            view_matrix,
            projection_matrix,
            model_matrix,
            mode_config,
            viewport,
        )
    }

    /// Same as [`Self::interact`], for objects without scale (slice planes).
    ///
    /// The scale mode shows the translation and rotation handles.
    pub fn interact_without_scale(
        &mut self,
        ui: &mut Ui,
        view_matrix: Mat4,
        projection_matrix: Mat4,
        model_matrix: Mat4,
        settings: &GizmoSettings,
        viewport: egui::Rect,
    ) -> Option<Mat4> {
        let mode_config = ModeConfig::new(settings, false);
        self.interact_with(
            ui,
            view_matrix,
            projection_matrix,
            model_matrix,
            mode_config,
            viewport,
        )
    }

    fn interact_with(
        &mut self,
        ui: &mut Ui,
        view_matrix: Mat4,
        projection_matrix: Mat4,
        model_matrix: Mat4,
        mode_config: ModeConfig,
        viewport: egui::Rect,
    ) -> Option<Mat4> {
        // Convert glam Mat4 (f32) to DMat4 (f64) for transform-gizmo
        let view_f64 = mat4_to_dmat4(view_matrix);
        let proj_f64 = mat4_to_dmat4(projection_matrix);

        // Convert to row-major mint matrices as required by transform-gizmo
        let view_mint: mint::RowMatrix4<f64> = dmat4_to_row_mint(view_f64);
        let proj_mint: mint::RowMatrix4<f64> = dmat4_to_row_mint(proj_f64);

        // Create transform from model matrix
        let (scale, rotation, translation) = model_matrix.to_scale_rotation_translation();
        let transform = Transform {
            translation: mint::Vector3 {
                x: f64::from(translation.x),
                y: f64::from(translation.y),
                z: f64::from(translation.z),
            },
            rotation: mint::Quaternion {
                v: mint::Vector3 {
                    x: f64::from(rotation.x),
                    y: f64::from(rotation.y),
                    z: f64::from(rotation.z),
                },
                s: f64::from(rotation.w),
            },
            scale: mint::Vector3 {
                x: f64::from(scale.x),
                y: f64::from(scale.y),
                z: f64::from(scale.z),
            },
        };

        let config = GizmoConfig {
            view_matrix: view_mint,
            projection_matrix: proj_mint,
            viewport,
            modes: mode_config.modes,
            mode_override: None,
            orientation: mode_config.orientation,
            pivot_point: TransformPivotPoint::MedianPoint,
            snapping: mode_config.snapping,
            snap_angle: mode_config.snap_angle,
            snap_distance: mode_config.snap_distance,
            snap_scale: mode_config.snap_scale,
            visuals: GizmoVisuals::default(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };

        // Update gizmo configuration
        self.gizmo.update_config(config);

        // Interact with gizmo
        if let Some(new_transform) = self.update_and_draw(ui, transform, viewport) {
            // Convert back to Mat4
            let translation = Vec3::new(
                new_transform.translation.x as f32,
                new_transform.translation.y as f32,
                new_transform.translation.z as f32,
            );
            let rotation = Quat::from_xyzw(
                new_transform.rotation.v.x as f32,
                new_transform.rotation.v.y as f32,
                new_transform.rotation.v.z as f32,
                new_transform.rotation.s as f32,
            );
            let scale = Vec3::new(
                new_transform.scale.x as f32,
                new_transform.scale.y as f32,
                new_transform.scale.z as f32,
            );

            return Some(Mat4::from_scale_rotation_translation(
                scale,
                rotation,
                translation,
            ));
        }

        None
    }

    /// Updates the gizmo from the pointer, draws it, and returns the new transform
    /// during a drag.
    ///
    /// This replaces `GizmoExt::interact`, which senses drags at the cursor
    /// everywhere. Here the gizmo takes the pointer only over a handle or
    /// during a drag. Thus a drag that starts elsewhere still moves the camera.
    fn update_and_draw(
        &mut self,
        ui: &Ui,
        target: Transform,
        viewport: egui::Rect,
    ) -> Option<Transform> {
        let cursor = ui.input(|input| input.pointer.hover_pos());
        let claims_pointer =
            self.dragging || cursor.is_some_and(|pos| self.gizmo.pick_preview((pos.x, pos.y)));

        // The widget tells egui that the gizmo uses the pointer, so that
        // `egui_is_using_pointer` is true during the drag. If an egui widget
        // is on top of the gizmo, `hovered` is false.
        let hovered = match cursor {
            Some(pos) if claims_pointer => ui
                .interact(
                    egui::Rect::from_center_size(pos, egui::Vec2::splat(1.0)),
                    ui.id().with("gizmo_pointer"),
                    egui::Sense::click_and_drag(),
                )
                .hovered(),
            _ => false,
        };

        let interaction = GizmoInteraction {
            cursor_pos: cursor.map_or((0.0, 0.0), |pos| (pos.x, pos.y)),
            hovered,
            drag_started: ui
                .input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)),
            dragging: ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary)),
        };
        let result = self.gizmo.update(interaction, &[target]);
        self.dragging = result.is_some();

        let draw_data = self.gizmo.draw();
        egui::Painter::new(ui.ctx().clone(), ui.layer_id(), viewport).add(egui::Mesh {
            indices: draw_data.indices,
            vertices: draw_data
                .vertices
                .into_iter()
                .zip(draw_data.colors)
                .map(|(pos, [r, g, b, a])| egui::epaint::Vertex {
                    pos: pos.into(),
                    uv: egui::Pos2::default(),
                    color: egui::Rgba::from_rgba_premultiplied(r, g, b, a).into(),
                })
                .collect(),
            ..Default::default()
        });

        result.and_then(|(_, transforms)| transforms.first().copied())
    }

    /// Decomposes a Mat4 into translation, rotation (Euler degrees), and scale.
    #[must_use]
    pub fn decompose_transform(matrix: Mat4) -> (Vec3, Vec3, Vec3) {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        let euler = rotation.to_euler(glam::EulerRot::XYZ);
        let euler_degrees = Vec3::new(
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        );
        (translation, euler_degrees, scale)
    }

    /// Composes a Mat4 from translation, rotation (Euler degrees), and scale.
    #[must_use]
    pub fn compose_transform(translation: Vec3, euler_degrees: Vec3, scale: Vec3) -> Mat4 {
        let rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            euler_degrees.x.to_radians(),
            euler_degrees.y.to_radians(),
            euler_degrees.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(scale, rotation, translation)
    }
}

/// Convert glam Mat4 (f32) to `DMat4` (f64).
fn mat4_to_dmat4(m: Mat4) -> DMat4 {
    DMat4::from_cols_array(&[
        f64::from(m.x_axis.x),
        f64::from(m.x_axis.y),
        f64::from(m.x_axis.z),
        f64::from(m.x_axis.w),
        f64::from(m.y_axis.x),
        f64::from(m.y_axis.y),
        f64::from(m.y_axis.z),
        f64::from(m.y_axis.w),
        f64::from(m.z_axis.x),
        f64::from(m.z_axis.y),
        f64::from(m.z_axis.z),
        f64::from(m.z_axis.w),
        f64::from(m.w_axis.x),
        f64::from(m.w_axis.y),
        f64::from(m.w_axis.z),
        f64::from(m.w_axis.w),
    ])
}

/// Convert `DMat4` to row-major mint matrix.
fn dmat4_to_row_mint(m: DMat4) -> mint::RowMatrix4<f64> {
    // glam stores column-major, mint::RowMatrix4 expects row-major
    mint::RowMatrix4 {
        x: mint::Vector4 {
            x: m.x_axis.x,
            y: m.y_axis.x,
            z: m.z_axis.x,
            w: m.w_axis.x,
        },
        y: mint::Vector4 {
            x: m.x_axis.y,
            y: m.y_axis.y,
            z: m.z_axis.y,
            w: m.w_axis.y,
        },
        z: mint::Vector4 {
            x: m.x_axis.z,
            y: m.y_axis.z,
            z: m.z_axis.z,
            w: m.w_axis.z,
        },
        w: mint::Vector4 {
            x: m.x_axis.w,
            y: m.y_axis.w,
            z: m.z_axis.w,
            w: m.w_axis.w,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_creation() {
        let gizmo = TransformGizmo::new();
        // Just verify it can be created
        drop(gizmo);
    }

    #[test]
    fn test_mode_config_follows_mode_and_snap() {
        let mut settings = GizmoSettings::default();
        let config = ModeConfig::new(&settings, true);
        assert_eq!(config.modes, GizmoMode::all_translate());
        assert!(!config.snapping, "zero increments must not snap");

        settings.mode = PolyscopeGizmoMode::Rotate;
        settings.snap_rotate = 15.0;
        let config = ModeConfig::new(&settings, true);
        assert_eq!(config.modes, GizmoMode::all_rotate());
        assert!(config.snapping);
        assert!((config.snap_angle - 15.0_f32.to_radians()).abs() < 1e-6);

        settings.mode = PolyscopeGizmoMode::Scale;
        settings.snap_scale = 0.25;
        settings.local_space = true;
        let config = ModeConfig::new(&settings, true);
        assert_eq!(config.modes, GizmoMode::all_scale());
        assert!(config.snapping);
        assert!((config.snap_scale - 0.25).abs() < 1e-6);
        assert_eq!(config.orientation, GizmoOrientation::Local);
    }

    #[test]
    fn test_mode_config_without_scale() {
        let mut settings = GizmoSettings {
            mode: PolyscopeGizmoMode::Scale,
            snap_translate: 0.5,
            ..GizmoSettings::default()
        };
        let config = ModeConfig::new(&settings, false);
        assert_eq!(
            config.modes,
            GizmoMode::all_translate() | GizmoMode::all_rotate()
        );
        // The rotation increment is zero, so snapping must stay off.
        assert!(!config.snapping);

        settings.snap_rotate = 10.0;
        assert!(ModeConfig::new(&settings, false).snapping);
    }

    /// Runs the gizmo in headless egui frames, as the app shows it.
    struct GizmoHarness {
        ctx: egui::Context,
        gizmo: TransformGizmo,
        /// Transform of the target. Each result replaces it, as in the app.
        target: Mat4,
        frame: u32,
    }

    impl GizmoHarness {
        const SCREEN: egui::Rect =
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2 { x: 800.0, y: 600.0 });
        const CENTER: egui::Pos2 = egui::Pos2 { x: 400.0, y: 300.0 };

        /// Creates the harness and runs two frames, so that the overlay area
        /// is past its sizing pass.
        fn new() -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                gizmo: TransformGizmo::new(),
                target: Mat4::IDENTITY,
                frame: 0,
            };
            harness.run(Vec::new());
            harness.run(Vec::new());
            harness
        }

        /// Runs one frame with the given events and returns the gizmo result.
        fn run(&mut self, events: Vec<egui::Event>) -> Option<Mat4> {
            self.frame += 1;
            let input = egui::RawInput {
                screen_rect: Some(Self::SCREEN),
                time: Some(f64::from(self.frame) / 60.0),
                events,
                ..Default::default()
            };
            // The same matrix functions as `polyscope_render::Camera`.
            let eye = Vec3::new(0.0, 0.0, 5.0);
            let view = glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y);
            let projection = glam::camera::rh::proj::directx::perspective(
                45_f32.to_radians(),
                800.0 / 600.0,
                0.1,
                100.0,
            );
            let settings = GizmoSettings::default();
            let gizmo = &mut self.gizmo;
            let mut result = None;
            self.ctx
                .run_ui(input, |ui| {
                    TransformGizmo::overlay_area(egui::Id::new("test_gizmo_overlay"))
                        .fixed_pos(egui::Pos2::ZERO)
                        .show(ui.ctx(), |ui| {
                            ui.set_clip_rect(Self::SCREEN);
                            result = gizmo.interact(
                                ui,
                                view,
                                projection,
                                self.target,
                                &settings,
                                Self::SCREEN,
                            );
                        });
                })
                .drop_without_applying_deltas();
            if let Some(transform) = result {
                self.target = transform;
            }
            result
        }

        fn pointer_moved(&mut self, pos: egui::Pos2) -> Option<Mat4> {
            self.run(vec![egui::Event::PointerMoved(pos)])
        }

        fn primary_button(&mut self, pos: egui::Pos2, pressed: bool) -> Option<Mat4> {
            self.run(vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }])
        }

        /// Returns a point on the outermost handle to the right of the center.
        fn handle_on_x_axis(&self) -> egui::Pos2 {
            let x = (1..300)
                .rev()
                .map(|dx| Self::CENTER.x + dx as f32)
                .find(|x| self.gizmo.gizmo.pick_preview((*x, Self::CENTER.y)))
                .expect("no gizmo handle to the right of the center");
            egui::pos2(x - 2.0, Self::CENTER.y)
        }
    }

    #[test]
    fn test_drag_on_handle_moves_target() {
        let mut harness = GizmoHarness::new();
        let start = harness.handle_on_x_axis();

        // Hover first: the gizmo claims the pointer only over a handle.
        assert!(harness.pointer_moved(start).is_none());
        harness.primary_button(start, true);
        for step in 1..=5 {
            let pos = start + egui::vec2(10.0 * step as f32, 0.0);
            assert!(
                harness.pointer_moved(pos).is_some(),
                "the drag on the handle must change the transform"
            );
            assert!(
                harness.ctx.egui_is_using_pointer(),
                "egui must report the drag, so that the camera does not orbit"
            );
        }
        let end = start + egui::vec2(50.0, 0.0);
        harness.primary_button(end, false);

        let translation = harness.target.w_axis.truncate();
        assert!(
            translation.x > 0.2,
            "translation {translation} must follow +X"
        );
        assert!(translation.y.abs() < 1e-3 && translation.z.abs() < 1e-3);
        assert!(!harness.ctx.egui_is_using_pointer());
    }

    #[test]
    fn test_drag_off_handle_leaves_pointer_to_camera() {
        let mut harness = GizmoHarness::new();
        let start = egui::pos2(100.0, 100.0);

        assert!(harness.pointer_moved(start).is_none());
        assert!(harness.primary_button(start, true).is_none());
        for step in 1..=5 {
            let pos = start + egui::vec2(10.0 * step as f32, 5.0 * step as f32);
            assert!(harness.pointer_moved(pos).is_none());
            assert!(
                !harness.ctx.egui_is_using_pointer(),
                "a drag away from the handles must stay free for the camera"
            );
        }
    }

    #[test]
    fn test_decompose_compose_roundtrip() {
        let translation = Vec3::new(1.0, 2.0, 3.0);
        let euler_degrees = Vec3::new(45.0, 30.0, 15.0);
        let scale = Vec3::new(1.0, 2.0, 1.5);

        let matrix = TransformGizmo::compose_transform(translation, euler_degrees, scale);
        let (t, r, s) = TransformGizmo::decompose_transform(matrix);

        assert!((t - translation).length() < 0.001);
        assert!((r - euler_degrees).length() < 0.1);
        assert!((s - scale).length() < 0.001);
    }
}
