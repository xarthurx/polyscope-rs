//! Minimal dual quaternion implementation for rigid body interpolation.
//!
//! A dual quaternion encodes a rigid body transformation (rotation + translation)
//! as a pair of quaternions `(real, dual)`. Used for smooth camera flight animation
//! matching C++ Polyscope's `glm::dualquat` interpolation.

use glam::{Quat, Vec3};

/// A dual quaternion representing a rigid body transformation.
///
/// `real` encodes the rotation, `dual` encodes the translation as:
/// `dual = 0.5 * Quat(t.x, t.y, t.z, 0) * real`
#[derive(Debug, Clone, Copy)]
pub struct DualQuat {
    /// Rotation part (unit quaternion).
    pub real: Quat,
    /// Translation-encoding part.
    pub dual: Quat,
}

impl DualQuat {
    /// Creates a dual quaternion from a rotation quaternion and translation vector.
    #[must_use]
    pub fn from_rotation_translation(rot: Quat, translation: Vec3) -> Self {
        let real = rot.normalize();
        // dual = 0.5 * pure_quat(t) * real
        let t_quat = Quat::from_xyzw(translation.x, translation.y, translation.z, 0.0);
        let dual = (t_quat * real) * 0.5;
        Self { real, dual }
    }

    /// Extracts the rotation quaternion and translation vector.
    #[must_use]
    pub fn to_rotation_translation(&self) -> (Quat, Vec3) {
        let rot = self.real.normalize();
        // t_quat = 2 * dual * conjugate(real)
        let t_quat = (self.dual * rot.conjugate()) * 2.0;
        (rot, Vec3::new(t_quat.x, t_quat.y, t_quat.z))
    }

    /// Dual linear blend (DLB) interpolation between two dual quaternions.
    ///
    /// Matches GLM's `glm::lerp` for dual quaternions: component-wise lerp
    /// of both real and dual parts, then normalize. Produces smooth rigid
    /// body interpolation with slight arc motion.
    #[must_use]
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // Ensure shortest path: flip b if dot product is negative
        let (b_real, b_dual) = if a.real.dot(b.real) < 0.0 {
            (-b.real, -b.dual)
        } else {
            (b.real, b.dual)
        };

        let real = a.real * (1.0 - t) + b_real * t;
        let dual = a.dual * (1.0 - t) + b_dual * t;

        Self { real, dual }.normalize()
    }

    /// Normalizes the dual quaternion.
    ///
    /// Normalizes the real part to unit length, then makes the dual part
    /// orthogonal to the real part (removes any non-rigid component).
    #[must_use]
    pub fn normalize(&self) -> Self {
        let norm = self.real.length();
        if norm < 1e-10 {
            return *self;
        }
        let inv_norm = 1.0 / norm;
        let real = self.real * inv_norm;
        let dual = self.dual * inv_norm;
        // Remove any component of dual parallel to real
        let dual = dual - real * real.dot(dual);
        Self { real, dual }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_identity_round_trip() {
        let rot = Quat::IDENTITY;
        let t = Vec3::new(1.0, 2.0, 3.0);
        let dq = DualQuat::from_rotation_translation(rot, t);
        let (rot_out, t_out) = dq.to_rotation_translation();
        assert!((rot_out.dot(rot) - 1.0).abs() < 1e-5);
        assert!((t_out - t).length() < 1e-5);
    }

    #[test]
    fn test_rotated_round_trip() {
        let rot = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let t = Vec3::new(5.0, -3.0, 1.0);
        let dq = DualQuat::from_rotation_translation(rot, t);
        let (rot_out, t_out) = dq.to_rotation_translation();
        assert!((rot_out.dot(rot).abs() - 1.0).abs() < 1e-5);
        assert!((t_out - t).length() < 1e-5);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::ZERO);
        let rot_b = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let t_b = Vec3::new(10.0, 0.0, 0.0);
        let b = DualQuat::from_rotation_translation(rot_b, t_b);

        // t=0 should give a
        let at0 = DualQuat::lerp(&a, &b, 0.0);
        let (r0, t0) = at0.to_rotation_translation();
        assert!((r0.dot(Quat::IDENTITY).abs() - 1.0).abs() < 1e-4);
        assert!(t0.length() < 1e-4);

        // t=1 should give b
        let at1 = DualQuat::lerp(&a, &b, 1.0);
        let (r1, t1) = at1.to_rotation_translation();
        assert!((r1.dot(rot_b).abs() - 1.0).abs() < 1e-4);
        assert!((t1 - t_b).length() < 1e-3);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::ZERO);
        let b = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::new(10.0, 0.0, 0.0));

        let mid = DualQuat::lerp(&a, &b, 0.5);
        let (_, t_mid) = mid.to_rotation_translation();
        assert!((t_mid - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-4);
    }
}
