//! Camera and view management.

use glam::{Mat3, Mat4, Quat, Vec3};
use std::time::Instant;

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

impl From<u32> for NavigationStyle {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Turntable,
            1 => Self::Free,
            2 => Self::Planar,
            3 => Self::Arcball,
            4 => Self::FirstPerson,
            _ => Self::None,
        }
    }
}

impl From<NavigationStyle> for u32 {
    fn from(v: NavigationStyle) -> Self {
        match v {
            NavigationStyle::Turntable => 0,
            NavigationStyle::Free => 1,
            NavigationStyle::Planar => 2,
            NavigationStyle::Arcball => 3,
            NavigationStyle::FirstPerson => 4,
            NavigationStyle::None => 5,
        }
    }
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

impl From<u32> for ProjectionMode {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Perspective,
            _ => Self::Orthographic,
        }
    }
}

impl From<ProjectionMode> for u32 {
    fn from(v: ProjectionMode) -> Self {
        match v {
            ProjectionMode::Perspective => 0,
            ProjectionMode::Orthographic => 1,
        }
    }
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

impl From<u32> for AxisDirection {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::PosX,
            1 => Self::NegX,
            2 => Self::PosY,
            3 => Self::NegY,
            4 => Self::PosZ,
            _ => Self::NegZ,
        }
    }
}

impl From<AxisDirection> for u32 {
    fn from(v: AxisDirection) -> Self {
        match v {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        }
    }
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

/// State for an in-progress camera flight animation.
///
/// Stores start and end view matrices decomposed into rotation quaternion +
/// translation, matching C++ Polyscope's `startFlightTo()` / `updateFlight()`.
/// The C++ code uses `glm::dualquat` but only for the rotation part (dual=0),
/// so this is effectively quaternion lerp for rotation + linear lerp for
/// translation, both with smoothstep easing.
#[derive(Debug, Clone)]
pub struct CameraFlight {
    start_time: Instant,
    duration_secs: f32,
    /// Start view matrix rotation as quaternion.
    initial_rot: Quat,
    /// End view matrix rotation as quaternion.
    target_rot: Quat,
    /// Start view matrix translation component.
    initial_t: Vec3,
    /// End view matrix translation component.
    target_t: Vec3,
    /// Start FOV in radians.
    initial_fov: f32,
    /// End FOV in radians.
    target_fov: f32,
    /// Camera-to-target distance (preserved throughout flight).
    target_dist: f32,
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
    /// Active camera flight animation (if any).
    pub flight: Option<CameraFlight>,
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
            flight: None,
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
    /// Matches C++ Polyscope `processRotate` for `NavigateStyle::Turntable`:
    /// operates on the view matrix directly, then reconstructs position via
    /// `lookAt`. This avoids degenerate cross products at the poles that
    /// occur when transforming the camera position in world space.
    pub fn orbit_turntable(&mut self, delta_x: f32, delta_y: f32) {
        let up_vec = self.up_direction.to_vec3();

        // Get the camera frame from the current view matrix (always valid)
        let view_mat = self.view_matrix();
        let r = glam::Mat3::from_cols(
            view_mat.x_axis.truncate(),
            view_mat.y_axis.truncate(),
            view_mat.z_axis.truncate(),
        );
        let rt = r.transpose();
        let frame_look = rt * Vec3::new(0.0, 0.0, -1.0);
        let frame_right = rt * Vec3::new(1.0, 0.0, 0.0);

        // Gimbal-lock protection: prevent flipping past poles
        // With positive clamped_dy rotating downward (toward +up pole),
        // clamp to prevent crossing:
        let dot = frame_look.dot(up_vec);
        let clamped_dy = if dot > 0.99 {
            delta_y.max(0.0) // near top pole: only allow pitching downward (away)
        } else if dot < -0.99 {
            delta_y.min(0.0) // near bottom pole: only allow pitching upward (away)
        } else {
            delta_y
        };

        // Build the new view matrix by applying rotations (matching C++ exactly):
        // 1. Translate to center
        let mut vm = view_mat;
        vm *= Mat4::from_translation(self.target);

        // 2. Pitch around camera-space right axis (from the view matrix frame)
        // C++ uses: glm::rotate(identity, -delPhi, frameRightDir)
        // glam's look_at_rh produces an inverted-Z view compared to glm::lookAt,
        // so pitch direction must also be flipped.
        vm *= Mat4::from_axis_angle(frame_right, clamped_dy);

        // 3. Yaw around world-space up axis
        // C++ uses: glm::rotate(identity, delTheta, getUpVec()) — no negation
        vm *= Mat4::from_axis_angle(up_vec, delta_x);

        // 4. Undo centering
        vm *= Mat4::from_translation(-self.target);

        // Extract camera world position from the new view matrix
        let new_view = vm;
        let inv_view = new_view.inverse();
        let new_pos = Vec3::new(inv_view.w_axis.x, inv_view.w_axis.y, inv_view.w_axis.z);

        // Enforce exact distance to prevent numerical drift
        let radius = (self.position - self.target).length();
        let offset = new_pos - self.target;
        let actual_dist = offset.length();
        if actual_dist > 1e-8 {
            self.position = self.target + offset * (radius / actual_dist);
        } else {
            self.position = new_pos;
        }

        // Reconstruct view with lookAt (matches C++ line 204: lookAt(pos, center, upVec))
        // This ensures the up vector and view matrix are always consistent.
        let final_view = Mat4::look_at_rh(self.position, self.target, up_vec);
        if final_view.is_finite() {
            // Extract up from the reconstructed view matrix
            let fr = glam::Mat3::from_cols(
                final_view.x_axis.truncate(),
                final_view.y_axis.truncate(),
                final_view.z_axis.truncate(),
            );
            self.up = fr.transpose() * Vec3::Y;
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

        // Compute camera distance using FOV so the bounding sphere is fully visible.
        // self.fov is in radians already.
        let half_fov_v = self.fov * 0.5;
        let half_fov_h = (half_fov_v.tan() * self.aspect_ratio).atan();
        let half_fov = half_fov_v.min(half_fov_h); // use the tighter angle
        let radius = size * 0.5;
        // Add a small margin (1.1x) so objects don't touch the viewport edge
        let distance = (radius / half_fov.tan()) * 1.1;
        self.position = center + Vec3::new(0.0, 0.0, distance);

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

    // ========================================================================
    // Camera flight animation
    // ========================================================================

    /// Decomposes a 4x4 view matrix into a rotation quaternion and translation vector.
    fn decompose_view_matrix(view: &Mat4) -> (Quat, Vec3) {
        let rot_mat = Mat3::from_cols(
            view.x_axis.truncate(),
            view.y_axis.truncate(),
            view.z_axis.truncate(),
        );
        let rot = Quat::from_mat3(&rot_mat);
        let t = Vec3::new(view.w_axis.x, view.w_axis.y, view.w_axis.z);
        (rot, t)
    }

    /// Reconstructs camera position, target, and up from an *inverse* view matrix
    /// (camera-to-world) rotation and translation, preserving the given distance.
    fn camera_from_inverse_view(rot: &Quat, t: &Vec3, target_dist: f32) -> (Vec3, Vec3, Vec3) {
        let rot_mat = Mat3::from_quat(*rot);
        // For the inverse view matrix (camera-to-world), position IS the translation
        let position = *t;
        // Forward: camera looks down -Z in eye space; in world space that's -rot_mat * Z
        let forward = -(rot_mat * Vec3::Z);
        // Up: +Y in eye space; in world space that's rot_mat * Y
        let up = rot_mat * Vec3::Y;
        let target = position + forward * target_dist;
        (position, target, up)
    }

    /// Starts a smooth animated flight to the given view matrix and FOV.
    ///
    /// The `target_view` is a 4x4 view matrix (world-to-eye). The `target_fov`
    /// is in radians. `duration_secs` controls the flight length (C++ Polyscope
    /// default is 0.4 seconds).
    pub fn start_flight_to(&mut self, target_view: Mat4, target_fov: f32, duration_secs: f32) {
        let current_view = self.view_matrix();
        let current_dist = (self.position - self.target).length().max(0.01);

        // Interpolate the *inverse* view matrix (camera-to-world), then invert back.
        // This produces smoother motion when far from the origin (C++ commit 067f760).
        let (rot_start, t_start) = Self::decompose_view_matrix(&current_view.inverse());
        let (rot_end, t_end) = Self::decompose_view_matrix(&target_view.inverse());

        self.flight = Some(CameraFlight {
            start_time: Instant::now(),
            duration_secs,
            initial_rot: rot_start,
            target_rot: rot_end,
            initial_t: t_start,
            target_t: t_end,
            initial_fov: self.fov,
            target_fov,
            target_dist: current_dist,
        });
    }

    /// Updates the camera flight animation. Call once per frame.
    ///
    /// When the flight completes, the camera is set exactly to the target
    /// position and `self.flight` is cleared.
    pub fn update_flight(&mut self) {
        let Some(flight) = &self.flight else {
            return;
        };

        let elapsed = flight.start_time.elapsed().as_secs_f32();
        let t = (elapsed / flight.duration_secs).min(1.0);
        let dist = flight.target_dist;

        if t >= 1.0 {
            // Flight complete — set final position exactly
            let (position, target, up) =
                Self::camera_from_inverse_view(&flight.target_rot, &flight.target_t, dist);
            self.position = position;
            self.target = target;
            self.up = up;
            self.fov = flight.target_fov;
            self.flight = None;
        } else {
            // Smoothstep easing: 3t^2 - 2t^3
            let t_smooth = t * t * (3.0 - 2.0 * t);

            // Quaternion lerp for rotation (matching C++ glm::lerp on dualquat with dual=0)
            // Ensure shortest path
            let target_rot = if flight.initial_rot.dot(flight.target_rot) < 0.0 {
                -flight.target_rot
            } else {
                flight.target_rot
            };
            let interp_rot = flight.initial_rot.lerp(target_rot, t_smooth).normalize();

            // Linear interpolation for translation with smoothstep
            let interp_t = flight.initial_t.lerp(flight.target_t, t_smooth);

            // Interpolate FOV with raw t (matching C++ Polyscope)
            let fov = (1.0 - t) * flight.initial_fov + t * flight.target_fov;

            let (position, target, up) =
                Self::camera_from_inverse_view(&interp_rot, &interp_t, dist);
            self.position = position;
            self.target = target;
            self.up = up;
            self.fov = fov;
        }
    }

    /// Cancels any active camera flight animation.
    pub fn cancel_flight(&mut self) {
        self.flight = None;
    }

    /// Returns whether a camera flight animation is currently active.
    #[must_use]
    pub fn is_in_flight(&self) -> bool {
        self.flight.is_some()
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

// ===== View-state DTO + helpers =====

use serde::{Deserialize, Serialize};

/// Serializable snapshot of a `Camera` covering everything needed to
/// reproduce its rendered view.
///
/// Enum-string fields are DTO-local — the public enums on `Camera`
/// (`NavigationStyle`, `ProjectionMode`, `AxisDirection`) keep their
/// existing serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraState {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub projection_mode: String,
    pub ortho_scale: f32,
    pub navigation_style: String,
    pub up_direction: String,
    pub front_direction: String,
}

impl CameraState {
    /// Snapshot the current camera into a `CameraState`.
    #[must_use]
    pub fn from_camera(c: &Camera) -> Self {
        Self {
            position: c.position.into(),
            target: c.target.into(),
            up: c.up.into(),
            fov: c.fov,
            near: c.near,
            far: c.far,
            projection_mode: projection_mode_name(c.projection_mode).to_owned(),
            ortho_scale: c.ortho_scale,
            navigation_style: navigation_style_name(c.navigation_style).to_owned(),
            up_direction: axis_direction_name(c.up_direction).to_owned(),
            front_direction: axis_direction_name(c.front_direction).to_owned(),
        }
    }

    /// Apply this state to `camera`. For `Instant`, all fields snap. For
    /// `FlyTo`, position/target/up/fov animate via the existing camera flight
    /// (~0.4 s); other fields snap.
    pub fn apply(
        &self,
        camera: &mut Camera,
        transition: polyscope_core::view_state::ViewTransition,
    ) {
        use polyscope_core::view_state::ViewTransition;

        let new_pos = Vec3::from(self.position);
        let new_target = Vec3::from(self.target);
        let new_up = Vec3::from(self.up);

        camera.near = self.near;
        camera.far = self.far;
        camera.projection_mode =
            parse_projection_mode(&self.projection_mode).unwrap_or(camera.projection_mode);
        camera.ortho_scale = self.ortho_scale;
        camera.navigation_style =
            parse_navigation_style(&self.navigation_style).unwrap_or(camera.navigation_style);
        camera.up_direction =
            parse_axis_direction(&self.up_direction).unwrap_or(camera.up_direction);
        camera.front_direction =
            parse_axis_direction(&self.front_direction).unwrap_or(camera.front_direction);

        match transition {
            ViewTransition::Instant => {
                camera.position = new_pos;
                camera.target = new_target;
                camera.up = new_up;
                camera.fov = self.fov;
                camera.flight = None;
            }
            ViewTransition::FlyTo => {
                let target_view = Mat4::look_at_rh(new_pos, new_target, new_up);
                camera.start_flight_to(target_view, self.fov, 0.4);
            }
        }
    }
}

fn navigation_style_name(s: NavigationStyle) -> &'static str {
    match s {
        NavigationStyle::Turntable => "turntable",
        NavigationStyle::Free => "free",
        NavigationStyle::Planar => "planar",
        NavigationStyle::Arcball => "arcball",
        NavigationStyle::FirstPerson => "first_person",
        NavigationStyle::None => "none",
    }
}

fn parse_navigation_style(s: &str) -> Option<NavigationStyle> {
    Some(match s {
        "turntable" => NavigationStyle::Turntable,
        "free" => NavigationStyle::Free,
        "planar" => NavigationStyle::Planar,
        "arcball" => NavigationStyle::Arcball,
        "first_person" => NavigationStyle::FirstPerson,
        "none" => NavigationStyle::None,
        _ => return None,
    })
}

fn projection_mode_name(p: ProjectionMode) -> &'static str {
    match p {
        ProjectionMode::Perspective => "perspective",
        ProjectionMode::Orthographic => "orthographic",
    }
}

fn parse_projection_mode(s: &str) -> Option<ProjectionMode> {
    Some(match s {
        "perspective" => ProjectionMode::Perspective,
        "orthographic" => ProjectionMode::Orthographic,
        _ => return None,
    })
}

fn axis_direction_name(a: AxisDirection) -> &'static str {
    match a {
        AxisDirection::PosX => "pos_x",
        AxisDirection::NegX => "neg_x",
        AxisDirection::PosY => "pos_y",
        AxisDirection::NegY => "neg_y",
        AxisDirection::PosZ => "pos_z",
        AxisDirection::NegZ => "neg_z",
    }
}

fn parse_axis_direction(s: &str) -> Option<AxisDirection> {
    Some(match s {
        "pos_x" => AxisDirection::PosX,
        "neg_x" => AxisDirection::NegX,
        "pos_y" => AxisDirection::PosY,
        "neg_y" => AxisDirection::NegY,
        "pos_z" => AxisDirection::PosZ,
        "neg_z" => AxisDirection::NegZ,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use polyscope_core::view_state::ViewTransition;

    fn make_camera_state_input() -> Camera {
        let mut c = Camera::new(1.5);
        c.position = Vec3::new(2.0, 3.0, 4.0);
        c.target = Vec3::new(0.5, 0.0, 0.0);
        c.up = Vec3::new(0.0, 1.0, 0.0);
        c.fov = 0.8;
        c.near = 0.1;
        c.far = 500.0;
        c.projection_mode = ProjectionMode::Orthographic;
        c.ortho_scale = 2.5;
        c.navigation_style = NavigationStyle::Arcball;
        c.up_direction = AxisDirection::NegY;
        c.front_direction = AxisDirection::PosZ;
        c
    }

    #[test]
    fn test_camera_state_serializes_snake_case_enums() {
        let s = CameraState::from_camera(&make_camera_state_input());
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["projection_mode"], "orthographic");
        assert_eq!(v["navigation_style"], "arcball");
        assert_eq!(v["up_direction"], "neg_y");
        assert_eq!(v["front_direction"], "pos_z");
    }

    #[test]
    fn test_camera_state_roundtrip() {
        let cam = make_camera_state_input();
        let state = CameraState::from_camera(&cam);
        let json = serde_json::to_string(&state).unwrap();
        let parsed: CameraState = serde_json::from_str(&json).unwrap();

        let mut fresh = Camera::new(1.5);
        parsed.apply(&mut fresh, ViewTransition::Instant);

        let eps = 1e-5;
        assert!((fresh.position - cam.position).length() < eps);
        assert!((fresh.target - cam.target).length() < eps);
        assert!((fresh.up - cam.up).length() < eps);
        assert!((fresh.fov - cam.fov).abs() < eps);
        assert!((fresh.near - cam.near).abs() < eps);
        assert!((fresh.far - cam.far).abs() < eps);
        assert_eq!(fresh.projection_mode, cam.projection_mode);
        assert!((fresh.ortho_scale - cam.ortho_scale).abs() < eps);
        assert_eq!(fresh.navigation_style, cam.navigation_style);
        assert_eq!(fresh.up_direction, cam.up_direction);
        assert_eq!(fresh.front_direction, cam.front_direction);
    }

    #[test]
    fn test_camera_state_instant_does_not_start_flight() {
        let cam = make_camera_state_input();
        let state = CameraState::from_camera(&cam);
        let mut fresh = Camera::new(1.5);
        state.apply(&mut fresh, ViewTransition::Instant);
        assert!(fresh.flight.is_none());
    }

    #[test]
    fn test_camera_state_flyto_starts_flight() {
        let cam = make_camera_state_input();
        let state = CameraState::from_camera(&cam);
        let mut fresh = Camera::new(1.5);
        fresh.position = Vec3::new(-1.0, -1.0, -1.0);
        fresh.target = Vec3::ZERO;
        fresh.up = Vec3::Y;

        state.apply(&mut fresh, ViewTransition::FlyTo);
        assert!(fresh.flight.is_some(), "FlyTo should start a camera flight");
        // Non-pose fields apply immediately
        assert_eq!(fresh.projection_mode, cam.projection_mode);
        assert!((fresh.ortho_scale - cam.ortho_scale).abs() < 1e-5);
    }

    #[test]
    fn test_camera_state_unknown_enum_falls_back_in_apply() {
        let mut bad = CameraState::from_camera(&make_camera_state_input());
        bad.projection_mode = "definitely_not_a_mode".to_string();
        bad.navigation_style = "also_not_real".to_string();

        let mut target = Camera::new(1.5);
        let original_proj = target.projection_mode;
        let original_nav = target.navigation_style;
        bad.apply(&mut target, ViewTransition::Instant);

        assert_eq!(target.projection_mode, original_proj);
        assert_eq!(target.navigation_style, original_nav);
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

    /// Mirrors the WGSL formula used by the curve tube ortho fix:
    /// `forward = -vec3<f32>(view[0].z, view[1].z, view[2].z)`.
    /// glam's `view.x_axis` is column 0, `view.x_axis.z` is `view[0].z` in WGSL.
    fn shader_forward_from_view(view: Mat4) -> Vec3 {
        -Vec3::new(view.x_axis.z, view.y_axis.z, view.z_axis.z)
    }

    /// The shader's view-forward extraction must match `Camera::forward()` for
    /// every camera pose, otherwise ortho ray casting in `curve_network_tube.wgsl`,
    /// `reflected_curve_network_tube.wgsl`, and `pick_curve_tube.wgsl` will use
    /// the wrong ray direction.
    #[test]
    fn shader_view_forward_matches_camera_forward() {
        let cases: &[(Vec3, Vec3, Vec3)] = &[
            // (eye, target, up)
            (Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y),
            (Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, Vec3::Y),
            (Vec3::new(0.0, 5.0, 0.0), Vec3::ZERO, Vec3::Z),
            (Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO, Vec3::Y),
            (
                Vec3::new(-2.0, 1.0, -3.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::Y,
            ),
            (
                Vec3::new(7.0, -2.0, 0.5),
                Vec3::new(-1.0, 0.0, 2.0),
                Vec3::Z,
            ),
        ];
        for &(eye, target, up) in cases {
            let view = Mat4::look_at_rh(eye, target, up);
            let extracted = shader_forward_from_view(view);
            let expected = (target - eye).normalize();
            assert!(
                extracted.distance(expected) < 1e-5,
                "case eye={eye:?} target={target:?} up={up:?}: \
                 extracted={extracted:?} expected={expected:?}"
            );
        }
    }

    /// `CameraUniforms` must be exactly 272 bytes: four mat4x4 (256) + vec3 +
    /// f32 flag. If anyone changes the layout, every shader that aliases
    /// `camera_pos` as `vec4<f32>` and reads `.w` as the ortho flag breaks
    /// silently.
    #[test]
    fn camera_uniforms_layout_is_stable() {
        use crate::engine::CameraUniforms;
        assert_eq!(std::mem::size_of::<CameraUniforms>(), 272);
        assert_eq!(std::mem::align_of::<CameraUniforms>(), 4);
    }

    /// Setting `Camera::projection_mode` to `Orthographic` must propagate to
    /// the GPU uniform's `is_orthographic` field as 1.0; perspective gives 0.0.
    /// The three tube shaders branch on `camera.camera_pos.w > 0.5`, so the
    /// exact float values matter.
    ///
    /// Calls the same `CameraUniforms::from_camera` that `update_camera_uniforms`
    /// uses on the render path, so this test catches drift if either side changes.
    #[test]
    fn ortho_flag_propagates_to_uniform() {
        use crate::engine::CameraUniforms;

        let mut camera = Camera::new(1.0);
        camera.projection_mode = ProjectionMode::Perspective;
        let u_persp = CameraUniforms::from_camera(&camera);
        assert!(
            u_persp.is_orthographic < 0.5,
            "perspective should produce flag below the 0.5 shader threshold, got {}",
            u_persp.is_orthographic
        );

        camera.projection_mode = ProjectionMode::Orthographic;
        let u_ortho = CameraUniforms::from_camera(&camera);
        assert!(
            u_ortho.is_orthographic > 0.5,
            "orthographic must clear the `> 0.5` shader threshold, got {}",
            u_ortho.is_orthographic
        );
    }
}
