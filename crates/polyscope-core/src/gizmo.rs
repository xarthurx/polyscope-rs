//! Transformation gizmo system for interactive manipulation.
//!
//! Provides visual gizmos for translating, rotating, and scaling structures.

use glam::{Mat4, Quat, Vec3};

/// The type of transformation gizmo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    /// Translation gizmo (arrows along axes).
    #[default]
    Translate,
    /// Rotation gizmo (circles around axes).
    Rotate,
    /// Scale gizmo (boxes along axes).
    Scale,
}

/// The coordinate space for gizmo operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoSpace {
    /// World coordinate system.
    #[default]
    World,
    /// Local coordinate system (relative to object).
    Local,
}

/// Configuration for a transformation gizmo.
#[derive(Debug, Clone)]
pub struct GizmoConfig {
    /// The current gizmo mode.
    pub mode: GizmoMode,
    /// The coordinate space.
    pub space: GizmoSpace,
    /// Whether the gizmo is visible.
    pub visible: bool,
    /// Size of the gizmo (in screen-relative units).
    pub size: f32,
    /// Snap value for translation (0.0 = disabled).
    pub snap_translate: f32,
    /// Snap value for rotation in degrees (0.0 = disabled).
    pub snap_rotate: f32,
    /// Snap value for scale (0.0 = disabled).
    pub snap_scale: f32,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Translate,
            space: GizmoSpace::World,
            visible: true,
            size: 100.0,
            snap_translate: 0.0,
            snap_rotate: 0.0,
            snap_scale: 0.0,
        }
    }
}

impl GizmoConfig {
    /// Creates a new gizmo configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the gizmo mode.
    #[must_use]
    pub fn with_mode(mut self, mode: GizmoMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the coordinate space.
    #[must_use]
    pub fn with_space(mut self, space: GizmoSpace) -> Self {
        self.space = space;
        self
    }

    /// Sets the gizmo size.
    #[must_use]
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the translation snap value.
    #[must_use]
    pub fn with_snap_translate(mut self, snap: f32) -> Self {
        self.snap_translate = snap;
        self
    }

    /// Sets the rotation snap value in degrees.
    #[must_use]
    pub fn with_snap_rotate(mut self, snap: f32) -> Self {
        self.snap_rotate = snap;
        self
    }

    /// Sets the scale snap value.
    #[must_use]
    pub fn with_snap_scale(mut self, snap: f32) -> Self {
        self.snap_scale = snap;
        self
    }
}

/// A transformation represented as separate components.
///
/// This is useful for UI display and incremental manipulation.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Translation component.
    pub translation: Vec3,
    /// Rotation component as a quaternion.
    pub rotation: Quat,
    /// Scale component.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Creates a new identity transform.
    #[must_use]
    pub fn identity() -> Self {
        Self::default()
    }

    /// Creates a transform from a translation.
    #[must_use]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    /// Creates a transform from a rotation.
    #[must_use]
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Default::default()
        }
    }

    /// Creates a transform from a scale.
    #[must_use]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Default::default()
        }
    }

    /// Creates a transform from a Mat4.
    ///
    /// This decomposition may not be exact for matrices with shear.
    #[must_use]
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Converts this transform to a Mat4.
    #[must_use]
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Returns the rotation as Euler angles (in radians).
    #[must_use]
    pub fn euler_angles(&self) -> Vec3 {
        let (x, y, z) = self.rotation.to_euler(glam::EulerRot::XYZ);
        Vec3::new(x, y, z)
    }

    /// Sets the rotation from Euler angles (in radians).
    pub fn set_euler_angles(&mut self, angles: Vec3) {
        self.rotation = Quat::from_euler(glam::EulerRot::XYZ, angles.x, angles.y, angles.z);
    }

    /// Returns the rotation as Euler angles (in degrees).
    #[must_use]
    pub fn euler_angles_degrees(&self) -> Vec3 {
        self.euler_angles() * (180.0 / std::f32::consts::PI)
    }

    /// Sets the rotation from Euler angles (in degrees).
    pub fn set_euler_angles_degrees(&mut self, degrees: Vec3) {
        self.set_euler_angles(degrees * (std::f32::consts::PI / 180.0));
    }

    /// Translates the transform.
    pub fn translate(&mut self, delta: Vec3) {
        self.translation += delta;
    }

    /// Rotates the transform.
    pub fn rotate(&mut self, delta: Quat) {
        self.rotation = delta * self.rotation;
    }

    /// Scales the transform.
    pub fn scale_by(&mut self, factor: Vec3) {
        self.scale *= factor;
    }

    /// Applies snap to translation.
    pub fn snap_translation(&mut self, snap: f32) {
        if snap > 0.0 {
            self.translation = (self.translation / snap).round() * snap;
        }
    }

    /// Applies snap to rotation (in degrees).
    pub fn snap_rotation(&mut self, snap_degrees: f32) {
        if snap_degrees > 0.0 {
            let mut euler = self.euler_angles_degrees();
            euler = (euler / snap_degrees).round() * snap_degrees;
            self.set_euler_angles_degrees(euler);
        }
    }

    /// Applies snap to scale.
    pub fn snap_scale(&mut self, snap: f32) {
        if snap > 0.0 {
            self.scale = (self.scale / snap).round() * snap;
        }
    }
}

/// Axis for single-axis gizmo operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    /// X axis (red).
    X,
    /// Y axis (green).
    Y,
    /// Z axis (blue).
    Z,
    /// XY plane.
    XY,
    /// XZ plane.
    XZ,
    /// YZ plane.
    YZ,
    /// All axes (free movement).
    All,
    /// No axis selected.
    None,
}

impl GizmoAxis {
    /// Returns the direction vector for this axis.
    #[must_use]
    pub fn direction(&self) -> Option<Vec3> {
        match self {
            GizmoAxis::X => Some(Vec3::X),
            GizmoAxis::Y => Some(Vec3::Y),
            GizmoAxis::Z => Some(Vec3::Z),
            _ => None,
        }
    }

    /// Returns the color for this axis.
    #[must_use]
    pub fn color(&self) -> Vec3 {
        match self {
            GizmoAxis::X | GizmoAxis::YZ => Vec3::new(1.0, 0.2, 0.2), // Red
            GizmoAxis::Y | GizmoAxis::XZ => Vec3::new(0.2, 1.0, 0.2), // Green
            GizmoAxis::Z | GizmoAxis::XY => Vec3::new(0.2, 0.2, 1.0), // Blue
            GizmoAxis::All => Vec3::new(1.0, 1.0, 1.0),               // White
            GizmoAxis::None => Vec3::new(0.5, 0.5, 0.5),              // Gray
        }
    }
}

/// GPU-compatible gizmo uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmoUniforms {
    /// Model matrix for the gizmo.
    pub model: [[f32; 4]; 4],
    /// Color of the axis being drawn.
    pub color: [f32; 3],
    /// Whether the axis is highlighted.
    pub highlighted: f32,
}

impl Default for GizmoUniforms {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: [1.0, 1.0, 1.0],
            highlighted: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_matrix_roundtrip() {
        let t = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let matrix = t.to_matrix();
        let back = Transform::from_matrix(matrix);
        assert!((back.translation - t.translation).length() < 1e-6);
    }

    #[test]
    fn test_transform_euler_angles() {
        let mut t = Transform::identity();
        t.set_euler_angles_degrees(Vec3::new(0.0, 90.0, 0.0));
        let angles = t.euler_angles_degrees();
        assert!((angles.y - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_snap_translation() {
        let mut t = Transform::from_translation(Vec3::new(1.2, 2.7, 3.1));
        t.snap_translation(0.5);
        assert_eq!(t.translation, Vec3::new(1.0, 2.5, 3.0));
    }
}
