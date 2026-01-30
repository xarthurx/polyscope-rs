//! Camera and view management.

use glam::{Mat3, Mat4, Quat, Vec3};

/// Camera navigation/interaction style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationStyle {
    /// Turntable - orbits around target, constrained to up direction.
    #[default]
    Turntable,
    /// Free - unconstrained rotation using camera-local axes.
    Free,
    /// Planar - 2D panning only, no rotation.
    Planar,
    /// Arcball - sphere-mapped rotation (virtual trackball).
    Arcball,
    /// First person - mouse look + WASD movement.
    FirstPerson,
    /// None - all camera controls disabled.
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

    /// Returns the camera's local up direction (from the view matrix).
    #[must_use]
    pub fn camera_up(&self) -> Vec3 {
        let view = self.view_matrix();
        let r = Mat3::from_cols(
            view.x_axis.truncate(),
            view.y_axis.truncate(),
            view.z_axis.truncate(),
        );
        r.transpose() * Vec3::Y
    }

    // ========================================================================
    // Per-mode orbit/rotation methods
    // ========================================================================

    /// Turntable orbit: yaw around world-space up, pitch around camera-space
    /// right, with gimbal-lock protection. Always looks at target.
    ///
    /// Matches C++ Polyscope `processRotate` for `NavigateStyle::Turntable`.
    pub fn orbit_turntable(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).length();
        let look_dir = self.forward();
        let up_vec = self.up_direction.to_vec3();

        // Gimbal-lock protection: prevent flipping past poles
        let dot = look_dir.dot(up_vec);
        let clamped_dy = if dot > 0.99 {
            delta_y.min(0.0) // only allow moving away from top pole
        } else if dot < -0.99 {
            delta_y.max(0.0) // only allow moving away from bottom pole
        } else {
            delta_y
        };

        // Pitch around camera-space right axis
        let right_dir = self.right();
        let pitch_rot = Mat4::from_axis_angle(right_dir, -clamped_dy);

        // Yaw around world-space up axis
        // Negate: we rotate the camera position (not the view matrix), so the
        // orbit direction must be inverted to match the C++ view-matrix convention.
        let yaw_rot = Mat4::from_axis_angle(up_vec, -delta_x);

        // Apply: translate to center, rotate, translate back
        let to_center = Mat4::from_translation(self.target);
        let from_center = Mat4::from_translation(-self.target);
        let transform = to_center * yaw_rot * pitch_rot * from_center;

        let new_pos = transform.transform_point3(self.position);

        // Re-enforce exact distance to prevent numerical drift
        let offset = new_pos - self.target;
        let actual_dist = offset.length();
        if actual_dist > 1e-8 {
            self.position = self.target + offset * (radius / actual_dist);
        } else {
            self.position = new_pos;
        }

        // Recompute up from world up direction, but keep it perpendicular to look
        let new_look = (self.target - self.position).normalize();
        let new_right = new_look.cross(up_vec).normalize();
        self.up = new_right.cross(new_look).normalize();

        // Guard against degenerate up (looking straight along up axis)
        if self.up.length_squared() < 0.5 {
            self.up = up_vec;
        }
    }

    /// Free orbit: unconstrained rotation using camera-local axes.
    /// Both yaw and pitch use the camera's own coordinate frame.
    ///
    /// Matches C++ Polyscope `processRotate` for `NavigateStyle::Free`.
    pub fn orbit_free(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).length();
        let right_dir = self.right();
        let up_dir = self.camera_up();

        // Yaw around camera-space up, then pitch around camera-space right
        // Negate: position-based orbit is opposite to view-matrix rotation.
        let yaw_rot = Mat4::from_axis_angle(up_dir, -delta_x);
        let pitch_rot = Mat4::from_axis_angle(right_dir, -delta_y);

        let to_center = Mat4::from_translation(self.target);
        let from_center = Mat4::from_translation(-self.target);
        let transform = to_center * pitch_rot * yaw_rot * from_center;

        let new_pos = transform.transform_point3(self.position);

        // Re-enforce exact distance
        let offset = new_pos - self.target;
        let actual_dist = offset.length();
        if actual_dist > 1e-8 {
            self.position = self.target + offset * (radius / actual_dist);
        } else {
            self.position = new_pos;
        }

        // Update up vector by rotating it along with the camera
        let rot = pitch_rot * yaw_rot;
        self.up = rot.transform_vector3(self.up).normalize();
    }

    /// Arcball orbit: maps 2D mouse positions to a virtual sphere for rotation.
    /// `start` and `end` are normalized screen coordinates in [-1, 1].
    ///
    /// Matches C++ Polyscope `processRotate` for `NavigateStyle::Arcball`.
    pub fn orbit_arcball(&mut self, start: [f32; 2], end: [f32; 2]) {
        let to_sphere = |v: [f32; 2]| -> Vec3 {
            let x = v[0].clamp(-1.0, 1.0);
            let y = v[1].clamp(-1.0, 1.0);
            let mag = x * x + y * y;
            if mag <= 1.0 {
                Vec3::new(x, y, -(1.0 - mag).sqrt())
            } else {
                Vec3::new(x, y, 0.0).normalize()
            }
        };

        let sphere_start = to_sphere(start);
        let sphere_end = to_sphere(end);

        let rot_axis = -sphere_start.cross(sphere_end);
        if rot_axis.length_squared() < 1e-12 {
            return; // No meaningful rotation
        }
        let rot_angle = sphere_start.dot(sphere_end).clamp(-1.0, 1.0).acos();
        if rot_angle.abs() < 1e-8 {
            return;
        }

        // Build rotation in camera space, then convert to world space
        let view = self.view_matrix();
        let r = Mat3::from_cols(
            view.x_axis.truncate(),
            view.y_axis.truncate(),
            view.z_axis.truncate(),
        );
        let r_inv = r.transpose();

        // Camera-space rotation
        let cam_rot = Mat3::from_axis_angle(rot_axis.normalize(), rot_angle);

        // World-space rotation: R^-1 * cam_rot * R
        let world_rot = r_inv * cam_rot * r;
        let world_rot4 = Mat4::from_mat3(world_rot);

        let to_center = Mat4::from_translation(self.target);
        let from_center = Mat4::from_translation(-self.target);
        let transform = to_center * world_rot4 * from_center;

        let radius = (self.position - self.target).length();
        let new_pos = transform.transform_point3(self.position);

        // Re-enforce distance
        let offset = new_pos - self.target;
        let actual_dist = offset.length();
        if actual_dist > 1e-8 {
            self.position = self.target + offset * (radius / actual_dist);
        } else {
            self.position = new_pos;
        }

        // Rotate up vector
        self.up = (world_rot * self.up).normalize();
    }

    /// First-person mouse look: yaw around world up, pitch around camera right.
    /// Unlike orbit modes, this moves the target (look direction) rather than
    /// orbiting around a fixed target.
    ///
    /// Matches C++ Polyscope `processRotate` for `NavigateStyle::FirstPerson`.
    pub fn mouse_look(&mut self, delta_x: f32, delta_y: f32) {
        let up_vec = self.up_direction.to_vec3();
        let look_dir = self.forward();

        // Gimbal-lock protection for pitch
        let dot = look_dir.dot(up_vec);
        let clamped_dy = if dot > 0.99 {
            delta_y.min(0.0)
        } else if dot < -0.99 {
            delta_y.max(0.0)
        } else {
            delta_y
        };

        // Yaw around world up
        // Negate: positive mouse delta_x (drag right) should turn view right
        let yaw_rot = Quat::from_axis_angle(up_vec, -delta_x);
        // Pitch around camera right
        let right_dir = self.right();
        let pitch_rot = Quat::from_axis_angle(right_dir, -clamped_dy);

        // Apply rotations to look direction
        let new_look = (pitch_rot * yaw_rot * look_dir).normalize();

        // Move the target while keeping position fixed
        let dist = (self.target - self.position).length();
        self.target = self.position + new_look * dist;

        // Update up to stay perpendicular to look direction
        let new_right = new_look.cross(up_vec).normalize();
        self.up = new_right.cross(new_look).normalize();
        if self.up.length_squared() < 0.5 {
            self.up = up_vec;
        }
    }

    /// First-person WASD movement in camera-local coordinates.
    /// `delta` is (right, up, forward) movement in camera space,
    /// pre-scaled by `move_speed` and delta time by the caller.
    pub fn move_first_person(&mut self, delta: Vec3) {
        let fwd = self.forward();
        let right = self.right();
        let cam_up = self.camera_up();

        let world_offset = right * delta.x + cam_up * delta.y + fwd * delta.z;
        self.position += world_offset;
        self.target += world_offset;
    }

    /// Legacy orbit method — delegates to `orbit_turntable`.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.orbit_turntable(delta_x, delta_y);
    }

    /// Pans the camera (translates position and target together).
    /// For Turntable mode, this moves the orbit center.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up_dir = self.camera_up();
        let offset = right * delta_x + up_dir * delta_y;
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
