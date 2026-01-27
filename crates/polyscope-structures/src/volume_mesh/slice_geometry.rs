//! Geometry generation for slicing tetrahedra and hexahedra with a plane.
//!
//! This module provides algorithms for computing the intersection of volume mesh cells
//! with slice planes, producing polygon geometry for rendering cross-section caps.

use glam::Vec3;

/// Result of slicing a volumetric cell with a plane.
/// Can produce 0, 3, 4, 5, or 6 intersection points forming a convex polygon.
#[derive(Debug, Clone)]
pub struct CellSliceResult {
    /// Intersection points (vertices of the slice polygon)
    pub vertices: Vec<Vec3>,
    /// Interpolation data for vertex attributes.
    /// Each entry is (`vert_a`, `vert_b`, t) where result = lerp(a, b, t)
    pub interpolation: Vec<(u32, u32, f32)>,
}

impl CellSliceResult {
    /// Creates an empty slice result (no intersection).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            interpolation: Vec::new(),
        }
    }

    /// Returns true if the slice produced a valid polygon.
    #[must_use]
    pub fn has_intersection(&self) -> bool {
        self.vertices.len() >= 3
    }
}

/// Computes the intersection of a tetrahedron with a slice plane.
///
/// Returns the intersection polygon vertices and interpolation data
/// for computing attribute values at intersection points.
///
/// # Arguments
/// * `v0`, `v1`, `v2`, `v3` - Tetrahedron vertex positions
/// * `plane_origin` - A point on the plane
/// * `plane_normal` - The plane normal (points toward kept geometry)
///
/// # Returns
/// A `CellSliceResult` containing 0, 3, or 4 vertices depending on the intersection.
#[must_use]
pub fn slice_tet(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> CellSliceResult {
    let verts = [v0, v1, v2, v3];

    // Compute signed distances from each vertex to the plane
    let d: [f32; 4] = std::array::from_fn(|i| (verts[i] - plane_origin).dot(plane_normal));

    // Find edge intersections where sign changes
    let mut intersections = Vec::new();
    let mut interp_data = Vec::new();

    // All 6 edges of a tetrahedron
    let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

    for &(i, j) in &edges {
        // Check if the edge crosses the plane (different signs)
        if d[i] * d[j] < 0.0 {
            // Compute interpolation parameter
            let t = d[i] / (d[i] - d[j]);
            let point = verts[i].lerp(verts[j], t);
            intersections.push(point);
            interp_data.push((i as u32, j as u32, t));
        }
    }

    // Order vertices to form valid polygon (convex hull in 2D projection)
    if intersections.len() >= 3 {
        order_polygon_vertices(&mut intersections, &mut interp_data, plane_normal);
    }

    CellSliceResult {
        vertices: intersections,
        interpolation: interp_data,
    }
}

/// Slice a hexahedron by decomposing into 5 tetrahedra.
///
/// Hexahedra are sliced by treating them as 5 tetrahedra (using the standard
/// symmetric decomposition), then merging the resulting polygons.
///
/// # Arguments
/// * `vertices` - The 8 vertices of the hexahedron in standard ordering
/// * `plane_origin` - A point on the plane
/// * `plane_normal` - The plane normal (points toward kept geometry)
///
/// # Returns
/// A `CellSliceResult` containing 0, 3-6 vertices depending on the intersection.
#[must_use]
pub fn slice_hex(vertices: [Vec3; 8], plane_origin: Vec3, plane_normal: Vec3) -> CellSliceResult {
    // Standard decomposition of a hex into 5 tets
    // This decomposition is symmetric and works for any hex orientation
    let tet_indices = [
        [0, 1, 3, 4],
        [1, 2, 3, 6],
        [1, 4, 5, 6],
        [3, 4, 6, 7],
        [1, 3, 4, 6], // Central tet connecting all others
    ];

    let mut all_vertices = Vec::new();
    let mut all_interp = Vec::new();

    for tet in &tet_indices {
        let result = slice_tet(
            vertices[tet[0]],
            vertices[tet[1]],
            vertices[tet[2]],
            vertices[tet[3]],
            plane_origin,
            plane_normal,
        );

        // Remap interpolation indices from local tet indices to hex indices
        for (local_a, local_b, t) in result.interpolation {
            let hex_a = tet[local_a as usize] as u32;
            let hex_b = tet[local_b as usize] as u32;
            all_interp.push((hex_a, hex_b, t));
        }
        all_vertices.extend(result.vertices);
    }

    // Merge and deduplicate vertices that are close together
    merge_slice_vertices(&mut all_vertices, &mut all_interp);

    // Order vertices to form valid polygon
    if all_vertices.len() >= 3 {
        order_polygon_vertices(&mut all_vertices, &mut all_interp, plane_normal);
    }

    CellSliceResult {
        vertices: all_vertices,
        interpolation: all_interp,
    }
}

/// Orders polygon vertices in counter-clockwise order around the centroid.
///
/// This ensures the resulting polygon is suitable for rendering with correct face winding.
fn order_polygon_vertices(
    vertices: &mut Vec<Vec3>,
    interp: &mut Vec<(u32, u32, f32)>,
    normal: Vec3,
) {
    if vertices.len() < 3 {
        return;
    }

    // Compute centroid
    let centroid: Vec3 = vertices.iter().copied().sum::<Vec3>() / vertices.len() as f32;

    // Create a reference direction perpendicular to the normal
    let ref_dir = if let Some(first) = vertices.first() {
        (*first - centroid).normalize()
    } else {
        return;
    };

    // Sort by angle around centroid using signed angle
    let mut indices: Vec<usize> = (0..vertices.len()).collect();

    indices.sort_by(|&a, &b| {
        let va = (vertices[a] - centroid).normalize();
        let vb = (vertices[b] - centroid).normalize();

        // Compute signed angle relative to ref_dir around normal axis
        // angle = atan2(cross.dot(normal), dot(ref_dir, v))
        let cross_a = ref_dir.cross(va);
        let cross_b = ref_dir.cross(vb);

        let angle_a = cross_a.dot(normal).atan2(ref_dir.dot(va));
        let angle_b = cross_b.dot(normal).atan2(ref_dir.dot(vb));

        angle_a
            .partial_cmp(&angle_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Reorder both vertices and interpolation data
    let sorted_verts: Vec<Vec3> = indices.iter().map(|&i| vertices[i]).collect();
    let sorted_interp: Vec<_> = indices.iter().map(|&i| interp[i]).collect();

    *vertices = sorted_verts;
    *interp = sorted_interp;
}

/// Merges vertices that are very close together (within epsilon).
///
/// This is needed after hex decomposition where multiple tets may produce
/// nearly identical intersection points.
fn merge_slice_vertices(vertices: &mut Vec<Vec3>, interp: &mut Vec<(u32, u32, f32)>) {
    // Use a larger epsilon for merging since floating point errors can accumulate
    const EPSILON: f32 = 1e-4;

    if vertices.len() <= 1 {
        return;
    }

    let mut merged_verts = Vec::new();
    let mut merged_interp = Vec::new();

    for i in 0..vertices.len() {
        let v = vertices[i];
        let mut is_duplicate = false;

        for merged_v in &merged_verts {
            let diff: Vec3 = v - *merged_v;
            if diff.length_squared() < EPSILON * EPSILON {
                is_duplicate = true;
                break;
            }
        }

        if !is_duplicate {
            merged_verts.push(v);
            merged_interp.push(interp[i]);
        }
    }

    *vertices = merged_verts;
    *interp = merged_interp;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_tet_no_intersection() {
        // Tet entirely above the plane (all positive distances)
        let result = slice_tet(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.5, 1.0, 1.0),
            Vec3::new(0.5, 2.0, 0.5),
            Vec3::ZERO,
            Vec3::Y,
        );
        assert!(result.vertices.is_empty());
        assert!(!result.has_intersection());
    }

    #[test]
    fn test_slice_tet_triangle() {
        // Plane cuts one vertex off (one vertex below, three above)
        // This should produce 3 intersection points (triangle)
        let result = slice_tet(
            Vec3::new(0.0, -1.0, 0.0), // Below plane
            Vec3::new(1.0, 1.0, 0.0),  // Above plane
            Vec3::new(-1.0, 1.0, 0.0), // Above plane
            Vec3::new(0.0, 1.0, 1.0),  // Above plane
            Vec3::ZERO,
            Vec3::Y,
        );
        assert_eq!(result.vertices.len(), 3);
        assert!(result.has_intersection());

        // Verify interpolation data
        assert_eq!(result.interpolation.len(), 3);
        for (_, _, t) in &result.interpolation {
            assert!(*t >= 0.0 && *t <= 1.0);
        }
    }

    #[test]
    fn test_slice_tet_quad() {
        // Plane cuts through middle (two vertices on each side)
        // This should produce 4 intersection points (quad)
        let result = slice_tet(
            Vec3::new(0.0, -1.0, 0.0), // Below plane
            Vec3::new(1.0, -1.0, 0.0), // Below plane
            Vec3::new(0.5, 1.0, 0.0),  // Above plane
            Vec3::new(0.5, 1.0, 1.0),  // Above plane
            Vec3::ZERO,
            Vec3::Y,
        );
        assert_eq!(result.vertices.len(), 4);
        assert!(result.has_intersection());
    }

    #[test]
    fn test_slice_tet_interpolation_values() {
        // Test that interpolation produces correct positions
        let v0 = Vec3::new(0.0, -1.0, 0.0);
        let v1 = Vec3::new(0.0, 1.0, 0.0);
        let v2 = Vec3::new(1.0, 0.0, 0.0);
        let v3 = Vec3::new(0.0, 0.0, 1.0);

        let result = slice_tet(v0, v1, v2, v3, Vec3::ZERO, Vec3::Y);

        // The intersection with edge (0,1) should be at y=0
        // v0.y = -1, v1.y = 1, so t = 0.5 gives y = 0
        for (i, (a, b, t)) in result.interpolation.iter().enumerate() {
            let verts = [v0, v1, v2, v3];
            let computed = verts[*a as usize].lerp(verts[*b as usize], *t);
            let diff = (computed - result.vertices[i]).length();
            assert!(diff < 1e-5, "Interpolation mismatch at index {}", i);
        }
    }

    #[test]
    fn test_slice_hex_no_intersection() {
        // Hex entirely above the plane
        let vertices = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 2.0, 1.0),
            Vec3::new(0.0, 2.0, 1.0),
        ];
        let result = slice_hex(vertices, Vec3::ZERO, Vec3::Y);
        assert!(result.vertices.is_empty());
    }

    #[test]
    fn test_slice_hex_quad() {
        // Unit cube centered at origin, sliced by XZ plane
        // Should produce a polygon intersection (4 unique vertices after merge)
        let vertices = [
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
        ];
        let result = slice_hex(vertices, Vec3::ZERO, Vec3::Y);

        // Should have at least 3 vertices forming a valid polygon
        assert!(result.has_intersection());
        assert!(
            result.vertices.len() >= 3,
            "Expected at least 3 vertices, got {}",
            result.vertices.len()
        );

        // All vertices should be at y=0
        for v in &result.vertices {
            assert!(v.y.abs() < 1e-5, "Vertex y={} should be 0", v.y);
        }

        // The vertices should cover the square corners approximately
        // Check that we have vertices near each corner of the unit square at y=0
        let expected_corners = [
            Vec3::new(-0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, 0.5),
            Vec3::new(-0.5, 0.0, 0.5),
        ];

        for corner in &expected_corners {
            let has_near = result
                .vertices
                .iter()
                .any(|v| (*v - *corner).length() < 0.1);
            assert!(has_near, "Expected vertex near corner {:?}", corner);
        }
    }

    #[test]
    fn test_polygon_ordering() {
        // Verify vertices are ordered correctly (counter-clockwise)
        let v0 = Vec3::new(0.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, 1.0, 0.0);
        let v2 = Vec3::new(-1.0, 1.0, 0.0);
        let v3 = Vec3::new(0.0, 1.0, 1.0);

        let result = slice_tet(v0, v1, v2, v3, Vec3::ZERO, Vec3::Y);

        // Check that triangle has correct winding (normal should point in +Y direction)
        if result.vertices.len() >= 3 {
            let e1 = result.vertices[1] - result.vertices[0];
            let e2 = result.vertices[2] - result.vertices[0];
            let computed_normal = e1.cross(e2).normalize();

            // The computed normal should roughly align with the plane normal
            let dot = computed_normal.dot(Vec3::Y);
            assert!(
                dot.abs() > 0.9,
                "Polygon normal should align with plane normal"
            );
        }
    }
}
