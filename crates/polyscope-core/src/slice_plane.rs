//! Slice plane functionality for cutting through geometry.
//!
//! Slice planes allow visualizing the interior of 3D geometry by
//! discarding fragments on one side of the plane.

use glam::Vec3;

/// A slice plane that can cut through geometry.
///
/// The plane is defined by a point (origin) and a normal direction.
/// Geometry on the negative side of the plane (opposite to normal) is discarded.
#[derive(Debug, Clone)]
pub struct SlicePlane {
    /// Unique name of the slice plane.
    name: String,
    /// A point on the plane (the origin).
    origin: Vec3,
    /// The normal direction of the plane (points toward kept geometry).
    normal: Vec3,
    /// Whether the slice plane is active.
    enabled: bool,
    /// Whether to draw a visual representation of the plane.
    draw_plane: bool,
    /// Whether to draw a widget at the plane origin.
    draw_widget: bool,
    /// Color of the plane visualization.
    color: Vec3,
    /// Transparency of the plane visualization (0.0 = transparent, 1.0 = opaque).
    transparency: f32,
}

impl SlicePlane {
    /// Creates a new slice plane with default settings.
    ///
    /// By default, the plane is at the origin with +Y normal.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            enabled: true,
            draw_plane: true,
            draw_widget: true,
            color: Vec3::new(0.5, 0.5, 0.5),
            transparency: 0.3,
        }
    }

    /// Creates a slice plane with specific pose.
    pub fn with_pose(name: impl Into<String>, origin: Vec3, normal: Vec3) -> Self {
        Self {
            name: name.into(),
            origin,
            normal: normal.normalize(),
            enabled: true,
            draw_plane: true,
            draw_widget: true,
            color: Vec3::new(0.5, 0.5, 0.5),
            transparency: 0.3,
        }
    }

    /// Returns the name of this slice plane.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the origin point of the plane.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// Sets the origin point of the plane.
    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }

    /// Returns the normal direction of the plane.
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Sets the normal direction of the plane.
    pub fn set_normal(&mut self, normal: Vec3) {
        self.normal = normal.normalize();
    }

    /// Sets both origin and normal at once.
    pub fn set_pose(&mut self, origin: Vec3, normal: Vec3) {
        self.origin = origin;
        self.normal = normal.normalize();
    }

    /// Returns whether the slice plane is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether the slice plane is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether to draw the plane visualization.
    pub fn draw_plane(&self) -> bool {
        self.draw_plane
    }

    /// Sets whether to draw the plane visualization.
    pub fn set_draw_plane(&mut self, draw: bool) {
        self.draw_plane = draw;
    }

    /// Returns whether to draw the widget.
    pub fn draw_widget(&self) -> bool {
        self.draw_widget
    }

    /// Sets whether to draw the widget.
    pub fn set_draw_widget(&mut self, draw: bool) {
        self.draw_widget = draw;
    }

    /// Returns the color of the plane visualization.
    pub fn color(&self) -> Vec3 {
        self.color
    }

    /// Sets the color of the plane visualization.
    pub fn set_color(&mut self, color: Vec3) {
        self.color = color;
    }

    /// Returns the transparency of the plane visualization.
    pub fn transparency(&self) -> f32 {
        self.transparency
    }

    /// Sets the transparency of the plane visualization.
    pub fn set_transparency(&mut self, transparency: f32) {
        self.transparency = transparency.clamp(0.0, 1.0);
    }

    /// Returns the signed distance from a point to the plane.
    ///
    /// Positive values are on the normal side (kept), negative on the opposite (discarded).
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        (point - self.origin).dot(self.normal)
    }

    /// Returns whether a point is on the kept side of the plane.
    pub fn is_kept(&self, point: Vec3) -> bool {
        !self.enabled || self.signed_distance(point) >= 0.0
    }

    /// Projects a point onto the plane.
    pub fn project(&self, point: Vec3) -> Vec3 {
        point - self.signed_distance(point) * self.normal
    }
}

impl Default for SlicePlane {
    fn default() -> Self {
        Self::new("default")
    }
}

/// GPU-compatible slice plane uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SlicePlaneUniforms {
    /// Origin point of the plane.
    pub origin: [f32; 3],
    /// Whether the plane is enabled (1.0) or disabled (0.0).
    pub enabled: f32,
    /// Normal direction of the plane.
    pub normal: [f32; 3],
    /// Padding for alignment.
    pub _padding: f32,
}

impl From<&SlicePlane> for SlicePlaneUniforms {
    fn from(plane: &SlicePlane) -> Self {
        Self {
            origin: plane.origin.to_array(),
            enabled: if plane.enabled { 1.0 } else { 0.0 },
            normal: plane.normal.to_array(),
            _padding: 0.0,
        }
    }
}

impl Default for SlicePlaneUniforms {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            enabled: 0.0,
            normal: [0.0, 1.0, 0.0],
            _padding: 0.0,
        }
    }
}

/// Maximum number of slice planes supported.
pub const MAX_SLICE_PLANES: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_plane_creation() {
        let plane = SlicePlane::new("test");
        assert_eq!(plane.name(), "test");
        assert_eq!(plane.origin(), Vec3::ZERO);
        assert_eq!(plane.normal(), Vec3::Y);
        assert!(plane.is_enabled());
    }

    #[test]
    fn test_slice_plane_pose() {
        let mut plane = SlicePlane::new("test");
        plane.set_pose(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(plane.origin(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(plane.normal(), Vec3::X);
    }

    #[test]
    fn test_signed_distance() {
        let plane = SlicePlane::with_pose("test", Vec3::ZERO, Vec3::Y);

        // Point above the plane (positive Y)
        assert!(plane.signed_distance(Vec3::new(0.0, 1.0, 0.0)) > 0.0);

        // Point below the plane (negative Y)
        assert!(plane.signed_distance(Vec3::new(0.0, -1.0, 0.0)) < 0.0);

        // Point on the plane
        assert!((plane.signed_distance(Vec3::new(1.0, 0.0, 1.0))).abs() < 1e-6);
    }

    #[test]
    fn test_is_kept() {
        let plane = SlicePlane::with_pose("test", Vec3::ZERO, Vec3::Y);

        // Above plane - kept
        assert!(plane.is_kept(Vec3::new(0.0, 1.0, 0.0)));

        // Below plane - not kept
        assert!(!plane.is_kept(Vec3::new(0.0, -1.0, 0.0)));

        // Disabled plane - everything is kept
        let mut disabled_plane = plane.clone();
        disabled_plane.set_enabled(false);
        assert!(disabled_plane.is_kept(Vec3::new(0.0, -1.0, 0.0)));
    }

    #[test]
    fn test_project() {
        let plane = SlicePlane::with_pose("test", Vec3::ZERO, Vec3::Y);

        // Project point above plane onto plane
        let projected = plane.project(Vec3::new(1.0, 5.0, 2.0));
        assert!((projected - Vec3::new(1.0, 0.0, 2.0)).length() < 1e-6);
    }

    #[test]
    fn test_uniforms() {
        let plane = SlicePlane::with_pose("test", Vec3::new(1.0, 2.0, 3.0), Vec3::Z);
        let uniforms = SlicePlaneUniforms::from(&plane);

        assert_eq!(uniforms.origin, [1.0, 2.0, 3.0]);
        assert_eq!(uniforms.normal, [0.0, 0.0, 1.0]);
        assert_eq!(uniforms.enabled, 1.0);
    }
}
