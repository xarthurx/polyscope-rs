//! Coloring of volume meshes by scalar and color quantities.
//!
//! At most one scalar or color quantity colors the mesh. Vertex quantities give
//! one color per vertex, which the GPU interpolates over each face. Cell
//! quantities give one flat color per cell.

use glam::Vec4;
use polyscope_core::quantity::QuantityKind;
use polyscope_render::{ColorMap, ColorMapRegistry};

use super::{
    VolumeMesh, VolumeMeshCellColorQuantity, VolumeMeshCellScalarQuantity,
    VolumeMeshVertexColorQuantity, VolumeMeshVertexScalarQuantity,
};

/// The quantity that colors the mesh.
enum ColorSource<'a> {
    VertexScalar(&'a VolumeMeshVertexScalarQuantity),
    CellScalar(&'a VolumeMeshCellScalarQuantity),
    VertexColor(&'a VolumeMeshVertexColorQuantity),
    CellColor(&'a VolumeMeshCellColorQuantity),
}

/// Returns whether quantities of this kind color the mesh.
fn colors_mesh(kind: QuantityKind) -> bool {
    matches!(kind, QuantityKind::Scalar | QuantityKind::Color)
}

impl VolumeMesh {
    /// Returns the first enabled scalar or color quantity.
    fn active_color_source(&self) -> Option<ColorSource<'_>> {
        self.quantities
            .iter()
            .filter(|q| q.is_enabled())
            .find_map(|q| {
                let q = q.as_any();
                q.downcast_ref()
                    .map(ColorSource::VertexScalar)
                    .or_else(|| q.downcast_ref().map(ColorSource::CellScalar))
                    .or_else(|| q.downcast_ref().map(ColorSource::VertexColor))
                    .or_else(|| q.downcast_ref().map(ColorSource::CellColor))
            })
    }

    /// Returns whether an enabled scalar or color quantity colors the mesh.
    #[must_use]
    pub fn has_active_color_quantity(&self) -> bool {
        self.active_color_source().is_some()
    }

    /// Enables or disables a quantity by name.
    ///
    /// Scalar and color quantities color the mesh, so at most one of them is
    /// enabled. Enabling one disables the others, as `setDominantQuantity` does
    /// in C++ Polyscope. Vector quantities do not affect other quantities.
    pub fn set_quantity_enabled(&mut self, name: &str, enabled: bool) -> &mut Self {
        if let Some(index) = self.quantities.iter().position(|q| q.name() == name) {
            self.quantities[index].set_enabled(enabled);
            if enabled {
                self.disable_other_color_quantities(index);
            }
        }
        self
    }

    /// Disables all scalar and color quantities except the one at `keep`.
    ///
    /// Does nothing if the quantity at `keep` does not color the mesh.
    pub(super) fn disable_other_color_quantities(&mut self, keep: usize) {
        if !colors_mesh(self.quantities[keep].kind()) {
            return;
        }
        for (index, quantity) in self.quantities.iter_mut().enumerate() {
            if index != keep && colors_mesh(quantity.kind()) {
                quantity.set_enabled(false);
            }
        }
    }

    /// Uploads the colors of the enabled scalar or color quantity to the GPU.
    ///
    /// Call this every frame after `update_gpu_buffers`, so that changes of the
    /// color map or the data range show at once.
    pub fn update_quantity_colors(&self, queue: &wgpu::Queue, color_maps: &ColorMapRegistry) {
        if let Some(render_data) = &self.render_data
            && let Some(colors) = self.corner_colors(color_maps)
        {
            render_data.update_corner_colors(queue, &colors);
        }
    }

    /// Computes one color per rendered triangle corner from the enabled quantity.
    fn corner_colors(&self, color_maps: &ColorMapRegistry) -> Option<Vec<Vec4>> {
        let sources = &self.corner_sources;
        let colors = match self.active_color_source()? {
            ColorSource::VertexScalar(q) => scalar_colors(
                q.values(),
                q.data_range(),
                color_maps.get(q.color_map())?,
                &sources.vertices,
            ),
            ColorSource::CellScalar(q) => scalar_colors(
                q.values(),
                q.data_range(),
                color_maps.get(q.color_map())?,
                &sources.cells,
            ),
            ColorSource::VertexColor(q) => indexed_colors(q.colors(), &sources.vertices),
            ColorSource::CellColor(q) => indexed_colors(q.colors(), &sources.cells),
        };
        Some(colors)
    }
}

/// Maps the scalar value of each index through a color map.
fn scalar_colors(
    values: &[f32],
    (min, max): (f32, f32),
    color_map: &ColorMap,
    indices: &[u32],
) -> Vec<Vec4> {
    let range = if (max - min).abs() < 1e-10 {
        1.0
    } else {
        max - min
    };
    indices
        .iter()
        .map(|&i| {
            let value = values.get(i as usize).copied().unwrap_or(min);
            color_map.sample((value - min) / range).extend(1.0)
        })
        .collect()
}

/// Looks up the color of each index. Missing entries are white.
fn indexed_colors(colors: &[Vec4], indices: &[u32]) -> Vec<Vec4> {
    indices
        .iter()
        .map(|&i| colors.get(i as usize).copied().unwrap_or(Vec4::ONE))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use polyscope_core::structure::HasQuantities;

    fn two_tet_mesh() -> VolumeMesh {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, 0.0),
            Vec3::new(5.0, 1.0, 0.0),
            Vec3::new(5.0, 0.0, 1.0),
        ];
        let mut mesh = VolumeMesh::new_tet_mesh("test", vertices, vec![[0, 1, 2, 3], [4, 5, 6, 7]]);
        mesh.corner_sources = mesh.generate_render_corners(&[]).sources;
        mesh
    }

    #[test]
    fn test_enabling_a_color_quantity_disables_the_others() {
        let mut mesh = two_tet_mesh();
        mesh.add_vertex_scalar_quantity("vs", vec![0.0; 8]);
        mesh.add_cell_color_quantity("cc", vec![Vec3::ONE; 2]);
        mesh.add_vertex_vector_quantity("vv", vec![Vec3::X; 8]);

        mesh.set_quantity_enabled("vv", true);
        mesh.set_quantity_enabled("vs", true);
        mesh.set_quantity_enabled("cc", true);
        assert!(!mesh.get_quantity("vs").unwrap().is_enabled());
        assert!(mesh.get_quantity("cc").unwrap().is_enabled());
        assert!(mesh.get_quantity("vv").unwrap().is_enabled());
        assert!(mesh.has_active_color_quantity());

        mesh.set_quantity_enabled("cc", false);
        assert!(!mesh.has_active_color_quantity());
    }

    #[test]
    fn test_cell_scalar_colors_are_flat_per_cell() {
        let mut mesh = two_tet_mesh();
        mesh.add_cell_scalar_quantity("cs", vec![0.0, 1.0]);
        mesh.set_quantity_enabled("cs", true);

        let color_maps = ColorMapRegistry::new();
        let viridis = color_maps.get("viridis").unwrap();
        let colors = mesh.corner_colors(&color_maps).unwrap();
        // Each tet has 4 exterior triangles, so 12 corners.
        assert_eq!(colors.len(), 24);
        assert!(
            colors[..12]
                .iter()
                .all(|&c| c == viridis.sample(0.0).extend(1.0))
        );
        assert!(
            colors[12..]
                .iter()
                .all(|&c| c == viridis.sample(1.0).extend(1.0))
        );
    }

    #[test]
    fn test_vertex_quantities_follow_the_corner_vertex() {
        let mut mesh = two_tet_mesh();
        let vertex_colors: Vec<Vec3> = (0..8).map(|i| Vec3::splat(i as f32 / 7.0)).collect();
        mesh.add_vertex_color_quantity("vc", vertex_colors.clone());
        mesh.set_quantity_enabled("vc", true);

        let colors = mesh.corner_colors(&ColorMapRegistry::new()).unwrap();
        for (corner, &vertex) in mesh.corner_sources.vertices.iter().enumerate() {
            assert_eq!(colors[corner], vertex_colors[vertex as usize].extend(1.0));
        }
    }
}
