//! Camera parameters (intrinsics and extrinsics).

use glam::{Mat3, Mat4, Vec3};

/// Camera intrinsics parameters.
#[derive(Debug, Clone, Copy)]
pub struct CameraIntrinsics {
    /// Vertical field of view in degrees.
    pub fov_vertical_degrees: f32,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f32,
}

impl CameraIntrinsics {
    /// Creates new camera intrinsics.
    pub fn new(fov_vertical_degrees: f32, aspect_ratio: f32) -> Self {
        Self {
            fov_vertical_degrees,
            aspect_ratio,
        }
    }

    /// Creates intrinsics from horizontal FoV and aspect ratio.
    pub fn from_horizontal_fov(fov_horizontal_degrees: f32, aspect_ratio: f32) -> Self {
        // Convert horizontal to vertical FoV: tan(v/2) = tan(h/2) / aspect
        let h_rad = fov_horizontal_degrees.to_radians();
        let v_rad = 2.0 * ((h_rad / 2.0).tan() / aspect_ratio).atan();
        Self {
            fov_vertical_degrees: v_rad.to_degrees(),
            aspect_ratio,
        }
    }

    /// Creates default intrinsics (60° vertical FoV, 16:9 aspect).
    pub fn default_intrinsics() -> Self {
        Self {
            fov_vertical_degrees: 60.0,
            aspect_ratio: 16.0 / 9.0,
        }
    }
}

impl Default for CameraIntrinsics {
    fn default() -> Self {
        Self::default_intrinsics()
    }
}

/// Camera extrinsics parameters (position and orientation).
#[derive(Debug, Clone, Copy)]
pub struct CameraExtrinsics {
    /// Camera position in world space.
    pub position: Vec3,
    /// Look direction (normalized).
    pub look_dir: Vec3,
    /// Up direction (normalized).
    pub up_dir: Vec3,
}

impl CameraExtrinsics {
    /// Creates new camera extrinsics from position and directions.
    pub fn new(position: Vec3, look_dir: Vec3, up_dir: Vec3) -> Self {
        Self {
            position,
            look_dir: look_dir.normalize(),
            up_dir: up_dir.normalize(),
        }
    }

    /// Creates extrinsics with camera at origin looking down -Z.
    pub fn default_extrinsics() -> Self {
        Self {
            position: Vec3::ZERO,
            look_dir: -Vec3::Z,
            up_dir: Vec3::Y,
        }
    }

    /// Creates extrinsics for a camera looking at a target point.
    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> Self {
        let look_dir = (target - position).normalize();
        Self::new(position, look_dir, up)
    }

    /// Gets the right direction (look cross up).
    pub fn right_dir(&self) -> Vec3 {
        self.look_dir.cross(self.up_dir).normalize()
    }

    /// Gets the camera frame as (look, up, right).
    pub fn camera_frame(&self) -> (Vec3, Vec3, Vec3) {
        let right = self.right_dir();
        // Re-orthogonalize up
        let up = right.cross(self.look_dir).normalize();
        (self.look_dir, up, right)
    }

    /// Returns the view matrix (world to camera space).
    pub fn view_matrix(&self) -> Mat4 {
        let (look, up, right) = self.camera_frame();
        // Camera looks down -Z in eye space
        let rotation = Mat3::from_cols(right, up, -look);
        let translation = -rotation * self.position;
        Mat4::from_cols(
            rotation.x_axis.extend(0.0),
            rotation.y_axis.extend(0.0),
            rotation.z_axis.extend(0.0),
            translation.extend(1.0),
        )
    }
}

impl Default for CameraExtrinsics {
    fn default() -> Self {
        Self::default_extrinsics()
    }
}

/// Combined camera parameters (intrinsics + extrinsics).
#[derive(Debug, Clone, Copy)]
pub struct CameraParameters {
    /// Intrinsic parameters (FoV, aspect ratio).
    pub intrinsics: CameraIntrinsics,
    /// Extrinsic parameters (position, orientation).
    pub extrinsics: CameraExtrinsics,
}

impl CameraParameters {
    /// Creates new camera parameters.
    pub fn new(intrinsics: CameraIntrinsics, extrinsics: CameraExtrinsics) -> Self {
        Self { intrinsics, extrinsics }
    }

    /// Creates camera parameters from vectors.
    pub fn from_vectors(
        position: Vec3,
        look_dir: Vec3,
        up_dir: Vec3,
        fov_vertical_degrees: f32,
        aspect_ratio: f32,
    ) -> Self {
        Self {
            intrinsics: CameraIntrinsics::new(fov_vertical_degrees, aspect_ratio),
            extrinsics: CameraExtrinsics::new(position, look_dir, up_dir),
        }
    }

    /// Creates camera parameters for looking at a target.
    pub fn look_at(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov_vertical_degrees: f32,
        aspect_ratio: f32,
    ) -> Self {
        Self {
            intrinsics: CameraIntrinsics::new(fov_vertical_degrees, aspect_ratio),
            extrinsics: CameraExtrinsics::look_at(position, target, up),
        }
    }

    /// Gets the camera position.
    pub fn position(&self) -> Vec3 {
        self.extrinsics.position
    }

    /// Gets the look direction.
    pub fn look_dir(&self) -> Vec3 {
        self.extrinsics.look_dir
    }

    /// Gets the up direction.
    pub fn up_dir(&self) -> Vec3 {
        self.extrinsics.up_dir
    }

    /// Gets the right direction.
    pub fn right_dir(&self) -> Vec3 {
        self.extrinsics.right_dir()
    }

    /// Gets the camera frame as (look, up, right).
    pub fn camera_frame(&self) -> (Vec3, Vec3, Vec3) {
        self.extrinsics.camera_frame()
    }

    /// Gets the vertical field of view in degrees.
    pub fn fov_vertical_degrees(&self) -> f32 {
        self.intrinsics.fov_vertical_degrees
    }

    /// Gets the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        self.intrinsics.aspect_ratio
    }

    /// Gets the view matrix.
    pub fn view_matrix(&self) -> Mat4 {
        self.extrinsics.view_matrix()
    }

    /// Gets the projection matrix.
    pub fn projection_matrix(&self, near: f32, far: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.intrinsics.fov_vertical_degrees.to_radians(),
            self.intrinsics.aspect_ratio,
            near,
            far,
        )
    }
}

impl Default for CameraParameters {
    fn default() -> Self {
        Self {
            intrinsics: CameraIntrinsics::default(),
            extrinsics: CameraExtrinsics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_frame() {
        let extrinsics = CameraExtrinsics::new(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let (look, up, right) = extrinsics.camera_frame();
        assert!((look - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-6);
        assert!((up - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
        assert!((right - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn test_look_at() {
        let params = CameraParameters::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::Y,
            60.0,
            1.5,
        );
        assert!((params.look_dir() - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-6);
        assert_eq!(params.fov_vertical_degrees(), 60.0);
        assert_eq!(params.aspect_ratio(), 1.5);
    }
}
