//! Camera and view management.

use glam::{Mat4, Vec3};

/// A 3D camera for viewing the scene.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space.
    pub position: Vec3,
    /// Point the camera is looking at.
    pub target: Vec3,
    /// Up vector.
    pub up: Vec3,
    /// Field of view in radians.
    pub fov: f32,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f32,
    /// Near clipping plane.
    pub near: f32,
    /// Far clipping plane.
    pub far: f32,
}

impl Camera {
    /// Creates a new camera with default settings.
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio,
            near: 0.01,
            far: 1000.0,
        }
    }

    /// Sets the aspect ratio.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Returns the view matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Returns the projection matrix.
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    /// Returns the combined view-projection matrix.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Returns the camera's forward direction.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Returns the camera's right direction.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    /// Orbits the camera around the target.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).length();
        let mut theta = (self.position.x - self.target.x).atan2(self.position.z - self.target.z);
        let mut phi = ((self.position.y - self.target.y) / radius).acos();

        theta -= delta_x;
        phi = (phi - delta_y).clamp(0.01, std::f32::consts::PI - 0.01);

        self.position = self.target
            + Vec3::new(
                radius * phi.sin() * theta.sin(),
                radius * phi.cos(),
                radius * phi.sin() * theta.cos(),
            );
    }

    /// Pans the camera.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up;
        let offset = right * delta_x + up * delta_y;
        self.position += offset;
        self.target += offset;
    }

    /// Zooms the camera (moves toward/away from target).
    pub fn zoom(&mut self, delta: f32) {
        let direction = self.forward();
        let distance = (self.position - self.target).length();
        let new_distance = (distance - delta).max(0.1);
        self.position = self.target - direction * new_distance;
    }

    /// Resets the camera to look at the given bounding box.
    pub fn look_at_box(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = (max - min).length();

        self.target = center;
        self.position = center + Vec3::new(0.0, 0.0, size * 1.5);
        self.near = size * 0.001;
        self.far = size * 100.0;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(16.0 / 9.0)
    }
}
