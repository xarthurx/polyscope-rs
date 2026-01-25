//! Planar reflection utilities.

use glam::{Mat4, Vec3, Vec4};

/// Computes a reflection matrix for a plane.
///
/// The plane is defined by a point on the plane and its normal.
/// The resulting matrix reflects points across this plane.
pub fn reflection_matrix(plane_point: Vec3, plane_normal: Vec3) -> Mat4 {
    let n = plane_normal.normalize();
    let d = -plane_point.dot(n);

    // Reflection matrix formula:
    // | 1-2nx²   -2nxny   -2nxnz   -2nxd |
    // | -2nxny   1-2ny²   -2nynz   -2nyd |
    // | -2nxnz   -2nynz   1-2nz²   -2nzd |
    // |    0        0        0       1   |

    Mat4::from_cols(
        Vec4::new(1.0 - 2.0 * n.x * n.x, -2.0 * n.x * n.y, -2.0 * n.x * n.z, 0.0),
        Vec4::new(-2.0 * n.x * n.y, 1.0 - 2.0 * n.y * n.y, -2.0 * n.y * n.z, 0.0),
        Vec4::new(-2.0 * n.x * n.z, -2.0 * n.y * n.z, 1.0 - 2.0 * n.z * n.z, 0.0),
        Vec4::new(-2.0 * n.x * d, -2.0 * n.y * d, -2.0 * n.z * d, 1.0),
    )
}

/// Computes a reflection matrix for a horizontal ground plane at given height.
///
/// Assumes Y-up coordinate system.
pub fn ground_reflection_matrix(height: f32) -> Mat4 {
    reflection_matrix(Vec3::new(0.0, height, 0.0), Vec3::Y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_matrix_identity_at_origin() {
        let mat = reflection_matrix(Vec3::ZERO, Vec3::Y);

        // Point above plane should reflect below
        let point = Vec3::new(1.0, 2.0, 3.0);
        let reflected = mat.transform_point3(point);

        assert!((reflected.x - point.x).abs() < 0.001);
        assert!((reflected.y - (-point.y)).abs() < 0.001);
        assert!((reflected.z - point.z).abs() < 0.001);
    }

    #[test]
    fn test_ground_reflection_at_height() {
        let height = 1.0;
        let mat = ground_reflection_matrix(height);

        // Point at height 3 should reflect to height -1
        let point = Vec3::new(0.0, 3.0, 0.0);
        let reflected = mat.transform_point3(point);

        // Distance from plane is 2, so reflected should be 2 below plane
        assert!((reflected.y - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_reflection_is_involution() {
        let mat = reflection_matrix(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
        let double = mat * mat;

        // Reflecting twice should give identity
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((double.col(j)[i] - expected).abs() < 0.001);
            }
        }
    }
}
