//! Visual 3D gizmo integration using transform-gizmo-egui.
//!
//! Shows all gizmo modes (translate, rotate, scale) simultaneously.

use egui::Ui;
use glam::{DMat4, Mat4, Quat, Vec3};
use transform_gizmo_egui::{
    Gizmo, GizmoConfig, GizmoExt, GizmoMode, GizmoOrientation, GizmoVisuals,
    config::TransformPivotPoint, math::Transform, mint,
};

/// Wrapper around transform-gizmo-egui for polyscope integration.
pub struct TransformGizmo {
    gizmo: Gizmo,
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
        }
    }

    /// Draws the gizmo and handles interaction.
    ///
    /// Shows all modes (translate, rotate, scale) simultaneously.
    /// Returns the updated transform if the gizmo was manipulated.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `view_matrix` - Camera view matrix
    /// * `projection_matrix` - Camera projection matrix
    /// * `model_matrix` - Current transform of the object
    /// * `local_space` - If true, use local coordinates; if false, use world coordinates
    /// * `viewport` - Viewport rect
    pub fn interact(
        &mut self,
        ui: &mut Ui,
        view_matrix: Mat4,
        projection_matrix: Mat4,
        model_matrix: Mat4,
        local_space: bool,
        viewport: egui::Rect,
    ) -> Option<Mat4> {
        let orientation = if local_space {
            GizmoOrientation::Local
        } else {
            GizmoOrientation::Global
        };

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
            modes: GizmoMode::all(),
            mode_override: None,
            orientation,
            pivot_point: TransformPivotPoint::MedianPoint,
            snapping: false,
            snap_angle: 0.0,
            snap_distance: 0.0,
            snap_scale: 0.0,
            visuals: GizmoVisuals::default(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };

        // Update gizmo configuration
        self.gizmo.update_config(config);

        // Interact with gizmo
        if let Some((_result, new_transforms)) = self.gizmo.interact(ui, &[transform]) {
            if let Some(new_transform) = new_transforms.first() {
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
        }

        None
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
