//! Camera and view management.

use glam::{Mat4, Vec3};

/// Camera navigation/interaction style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationStyle {
    /// Turntable - orbits around target, constrained to up direction.
    #[default]
    Turntable,
    /// Free - unconstrained rotation.
    Free,
    /// Planar - 2D panning only.
    Planar,
    /// First person - WASD-style movement.
    FirstPerson,
    /// None - camera controls disabled.
    None,
}

/// Camera projection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    /// Perspective projection.
    #[default]
    Perspective,
    /// Orthographic projection.
    Orthographic,
}

/// Axis direction for up/front vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisDirection {
    /// Positive X axis.
    PosX,
    /// Negative X axis.
    NegX,
    /// Positive Y axis (default up).
    #[default]
    PosY,
    /// Negative Y axis.
    NegY,
    /// Positive Z axis.
    PosZ,
    /// Negative Z axis (default front).
    NegZ,
}

impl AxisDirection {
    /// Returns the unit vector for this direction.
    #[must_use]
    pub fn to_vec3(self) -> Vec3 {
        match self {
            AxisDirection::PosX => Vec3::X,
            AxisDirection::NegX => Vec3::NEG_X,
            AxisDirection::PosY => Vec3::Y,
            AxisDirection::NegY => Vec3::NEG_Y,
            AxisDirection::PosZ => Vec3::Z,
            AxisDirection::NegZ => Vec3::NEG_Z,
        }
    }

    /// Returns display name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            AxisDirection::PosX => "+X",
            AxisDirection::NegX => "-X",
            AxisDirection::PosY => "+Y",
            AxisDirection::NegY => "-Y",
            AxisDirection::PosZ => "+Z",
            AxisDirection::NegZ => "-Z",
        }
    }

    /// Returns the corresponding front direction for this up direction.
    /// Follows right-hand coordinate system conventions:
    /// - +Y up → -Z front (standard graphics convention)
    /// - -Y up → +Z front
    /// - +Z up → +X front (CAD/engineering convention)
    /// - -Z up → -X front
    /// - +X up → +Y front
    /// - -X up → -Y front
    #[must_use]
    pub fn default_front_direction(self) -> AxisDirection {
        match self {
            AxisDirection::PosY => AxisDirection::NegZ,
            AxisDirection::NegY => AxisDirection::PosZ,
            AxisDirection::PosZ => AxisDirection::PosX,
            AxisDirection::NegZ => AxisDirection::NegX,
            AxisDirection::PosX => AxisDirection::PosY,
            AxisDirection::NegX => AxisDirection::NegY,
        }
    }

    /// Converts from a u32 index (used in UI) to `AxisDirection`.
    /// Order: 0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z
    #[must_use]
    #[allow(clippy::match_same_arms)] // 2 and _ both map to PosY (default) intentionally
    pub fn from_index(index: u32) -> Self {
        match index {
            0 => AxisDirection::PosX,
            1 => AxisDirection::NegX,
            2 => AxisDirection::PosY,
            3 => AxisDirection::NegY,
            4 => AxisDirection::PosZ,
            5 => AxisDirection::NegZ,
            _ => AxisDirection::PosY, // Default
        }
    }

    /// Converts to a u32 index (used in UI).
    #[must_use]
    pub fn to_index(self) -> u32 {
        match self {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        }
    }
}

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
    /// Navigation style.
    pub navigation_style: NavigationStyle,
    /// Projection mode.
    pub projection_mode: ProjectionMode,
    /// Up direction.
    pub up_direction: AxisDirection,
    /// Front direction.
    pub front_direction: AxisDirection,
    /// Movement speed multiplier.
    pub move_speed: f32,
    /// Orthographic scale (used when `projection_mode` is Orthographic).
    pub ortho_scale: f32,
}

impl Camera {
    /// Creates a new camera with default settings.
    #[must_use]
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio,
            near: 0.01,
            far: 1000.0,
            navigation_style: NavigationStyle::Turntable,
            projection_mode: ProjectionMode::Perspective,
            up_direction: AxisDirection::PosY,
            front_direction: AxisDirection::NegZ,
            move_speed: 1.0,
            ortho_scale: 1.0,
        }
    }

    /// Sets the aspect ratio.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Returns the view matrix.
    #[must_use]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Returns the projection matrix.
    #[must_use]
    pub fn projection_matrix(&self) -> Mat4 {
        match self.projection_mode {
            ProjectionMode::Perspective => {
                Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
            }
            ProjectionMode::Orthographic => {
                let half_height = self.ortho_scale;
                let half_width = half_height * self.aspect_ratio;
                // For orthographic, we need a much larger depth range to avoid clipping.
                // The camera may be far from the scene, but we want to see everything
                // around the target point. Use a symmetric range centered on the
                // camera-to-target distance.
                let dist = (self.position - self.target).length();
                // Near plane should be negative relative to target to see objects
                // between camera and target. We use a large range to avoid clipping.
                let ortho_depth = (dist + self.far).max(self.ortho_scale * 100.0);
                Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    -ortho_depth, // Negative near to see behind focus point
                    ortho_depth,
                )
            }
        }
    }

    /// Returns the combined view-projection matrix.
    #[must_use]
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Returns the camera's forward direction.
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Returns the camera's right direction.
    #[must_use]
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

    /// Zooms the camera (moves toward/away from target for perspective,
    /// adjusts `ortho_scale` for orthographic).
    pub fn zoom(&mut self, delta: f32) {
        match self.projection_mode {
            ProjectionMode::Perspective => {
                let direction = self.forward();
                let distance = (self.position - self.target).length();
                let new_distance = (distance - delta).max(0.1);
                self.position = self.target - direction * new_distance;
            }
            ProjectionMode::Orthographic => {
                // For orthographic, adjust the scale (smaller = zoom in, larger = zoom out)
                // delta > 0 means zoom in (scroll up), so decrease scale
                // Use a proportional factor based on current scale for consistent feel
                let zoom_factor = 1.0 - delta * 0.4;
                self.ortho_scale = (self.ortho_scale * zoom_factor).clamp(0.01, 1000.0);
            }
        }
    }

    /// Resets the camera to look at the given bounding box.
    pub fn look_at_box(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = (max - min).length();
        let extents = max - min;

        self.target = center;
        self.position = center + Vec3::new(0.0, 0.0, size * 1.5);
        self.near = size * 0.001;
        self.far = size * 100.0;

        // Set ortho_scale to fit the model in view
        // Use the larger of height or width/aspect_ratio to ensure model fits
        let half_height = extents.y.max(extents.x / self.aspect_ratio) * 0.6;
        self.ortho_scale = half_height.max(0.1);
    }

    /// Sets the navigation style.
    pub fn set_navigation_style(&mut self, style: NavigationStyle) {
        self.navigation_style = style;
    }

    /// Sets the projection mode.
    pub fn set_projection_mode(&mut self, mode: ProjectionMode) {
        self.projection_mode = mode;
    }

    /// Sets the up direction and updates both the up vector and front direction.
    /// The front direction is automatically derived using right-hand coordinate conventions.
    pub fn set_up_direction(&mut self, direction: AxisDirection) {
        self.up_direction = direction;
        self.up = direction.to_vec3();
        self.front_direction = direction.default_front_direction();
    }

    /// Sets the movement speed.
    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed.max(0.01);
    }

    /// Sets the orthographic scale.
    pub fn set_ortho_scale(&mut self, scale: f32) {
        self.ortho_scale = scale.max(0.01);
    }

    /// Sets the field of view in radians.
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov.clamp(0.1, std::f32::consts::PI - 0.1);
    }

    /// Sets the near clipping plane.
    pub fn set_near(&mut self, near: f32) {
        self.near = near.max(0.001);
    }

    /// Sets the far clipping plane.
    pub fn set_far(&mut self, far: f32) {
        self.far = far.max(self.near + 0.1);
    }

    /// Returns FOV in degrees.
    #[must_use]
    pub fn fov_degrees(&self) -> f32 {
        self.fov.to_degrees()
    }

    /// Sets FOV from degrees.
    pub fn set_fov_degrees(&mut self, degrees: f32) {
        self.set_fov(degrees.to_radians());
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(16.0 / 9.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_direction_to_vec3() {
        assert_eq!(AxisDirection::PosX.to_vec3(), Vec3::X);
        assert_eq!(AxisDirection::NegX.to_vec3(), Vec3::NEG_X);
        assert_eq!(AxisDirection::PosY.to_vec3(), Vec3::Y);
        assert_eq!(AxisDirection::NegY.to_vec3(), Vec3::NEG_Y);
        assert_eq!(AxisDirection::PosZ.to_vec3(), Vec3::Z);
        assert_eq!(AxisDirection::NegZ.to_vec3(), Vec3::NEG_Z);
    }

    #[test]
    fn test_camera_defaults() {
        let camera = Camera::default();
        assert_eq!(camera.navigation_style, NavigationStyle::Turntable);
        assert_eq!(camera.projection_mode, ProjectionMode::Perspective);
        assert_eq!(camera.up_direction, AxisDirection::PosY);
        assert_eq!(camera.move_speed, 1.0);
    }

    #[test]
    fn test_projection_mode_perspective() {
        let camera = Camera::new(1.0);
        let proj = camera.projection_matrix();
        // Perspective matrix has non-zero w division
        assert!(proj.w_axis.z != 0.0);
    }

    #[test]
    fn test_projection_mode_orthographic() {
        let mut camera = Camera::new(1.0);
        camera.projection_mode = ProjectionMode::Orthographic;
        camera.ortho_scale = 5.0;
        let proj = camera.projection_matrix();
        // Orthographic matrix has w_axis.w = 1.0, w_axis.z = 0.0
        assert!((proj.w_axis.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_set_fov_clamping() {
        let mut camera = Camera::new(1.0);
        camera.set_fov(0.0); // Too small
        assert!(camera.fov >= 0.1);

        camera.set_fov(std::f32::consts::PI); // Too large
        assert!(camera.fov < std::f32::consts::PI);
    }

    #[test]
    fn test_fov_degrees_conversion() {
        let mut camera = Camera::new(1.0);
        camera.set_fov_degrees(90.0);
        assert!((camera.fov_degrees() - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_set_up_direction() {
        let mut camera = Camera::new(1.0);
        camera.set_up_direction(AxisDirection::PosZ);
        assert_eq!(camera.up, Vec3::Z);
        assert_eq!(camera.up_direction, AxisDirection::PosZ);
    }

    #[test]
    fn test_zoom_perspective() {
        let mut camera = Camera::new(1.0);
        camera.projection_mode = ProjectionMode::Perspective;
        camera.position = Vec3::new(0.0, 0.0, 5.0);
        camera.target = Vec3::ZERO;

        let initial_distance = camera.position.distance(camera.target);
        camera.zoom(1.0); // Zoom in
        let new_distance = camera.position.distance(camera.target);

        assert!(
            new_distance < initial_distance,
            "Perspective zoom in should decrease distance"
        );
    }

    #[test]
    fn test_zoom_orthographic() {
        let mut camera = Camera::new(1.0);
        camera.projection_mode = ProjectionMode::Orthographic;
        camera.ortho_scale = 5.0;

        let initial_scale = camera.ortho_scale;
        camera.zoom(1.0); // Zoom in (positive delta)
        let new_scale = camera.ortho_scale;

        assert!(
            new_scale < initial_scale,
            "Orthographic zoom in should decrease scale"
        );
    }
}
