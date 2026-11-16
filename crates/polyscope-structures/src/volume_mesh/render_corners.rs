//! Per-corner data of the rendered volume mesh triangles.
//!
//! The render data holds three corners per exterior triangle. This module
//! computes, for each corner, the wireframe edge flags and the mesh vertex and
//! cell that the corner comes from. Quantity colors use the vertex and cell.

use glam::Vec3;

use super::{VolumeMesh, cell_data};

/// The mesh vertex and the cell of each rendered triangle corner.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct CornerSources {
    /// Mesh vertex index of each corner.
    pub vertices: Vec<u32>,
    /// Cell index of each corner.
    pub cells: Vec<u32>,
}

/// Per-corner data of the rendered triangles, three entries per triangle.
pub(super) struct RenderCorners {
    /// Real-edge flags for the wireframe. Internal diagonals of quad faces get 0.0.
    pub edge_is_real: Vec<Vec3>,
    /// The mesh vertex and the cell of each corner.
    pub sources: CornerSources,
}

impl VolumeMesh {
    /// Generates the per-corner data of the rendered triangles.
    ///
    /// The triangle order matches `generate_render_geometry_with_culling(planes)`, and
    /// `generate_render_geometry()` when `planes` is empty.
    pub(super) fn generate_render_corners(&self, planes: &[(Vec3, Vec3)]) -> RenderCorners {
        let face_counts = self.compute_face_counts_with_culling(planes);
        let mut corners = RenderCorners {
            edge_is_real: Vec::new(),
            sources: CornerSources::default(),
        };
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if !self.is_cell_visible(cell, planes) {
                continue;
            }
            for face in cell_data::face_data_for(self.cell_type(cell_idx)) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts.get(&key) != Some(&1) {
                    continue;
                }
                for &tri in face.triangulation {
                    let flags = cell_data::triangle_real_edges(face.polygon, tri);
                    corners.edge_is_real.extend([flags; 3]);
                    for slot in tri {
                        corners.sources.vertices.push(cell[slot]);
                        corners.sources.cells.push(cell_idx as u32);
                    }
                }
            }
        }
        corners
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_edge_is_real_hides_quad_diagonals() {
        let vertices = (0..8).map(|i| Vec3::splat(i as f32)).collect();
        let mesh = VolumeMesh::new_hex_mesh("test", vertices, vec![[0, 1, 2, 3, 4, 5, 6, 7]]);

        let (_, faces) = mesh.generate_render_geometry();
        let corners = mesh.generate_render_corners(&[]);
        assert_eq!(corners.edge_is_real.len(), faces.len() * 3);
        // Each triangle of a quad face has two polygon edges and one diagonal.
        for flags in &corners.edge_is_real {
            assert_eq!(flags.x + flags.y + flags.z, 2.0);
        }
    }

    #[test]
    fn test_corner_sources_match_render_geometry() {
        // Two tets that share the face [1, 2, 3]: only exterior faces are rendered.
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        let mesh = VolumeMesh::new_tet_mesh("test", vertices, vec![[0, 1, 2, 3], [1, 2, 3, 4]]);

        let (positions, faces) = mesh.generate_render_geometry();
        let sources = mesh.generate_render_corners(&[]).sources;
        assert_eq!(faces.len(), 6);
        assert_eq!(sources.vertices.len(), positions.len());
        assert_eq!(sources.cells.len(), positions.len());
        for (corner, &vertex) in sources.vertices.iter().enumerate() {
            assert_eq!(positions[corner], mesh.vertices()[vertex as usize]);
        }
        // The first three triangles belong to cell 0, the last three to cell 1.
        assert!(sources.cells[..9].iter().all(|&c| c == 0));
        assert!(sources.cells[9..].iter().all(|&c| c == 1));
    }
}
