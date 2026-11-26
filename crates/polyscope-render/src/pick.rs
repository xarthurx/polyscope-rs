//! Pick buffer rendering for element selection.
//!
//! The pick buffer is an offscreen framebuffer where each element is rendered
//! with a unique color encoding its ID. When the user clicks, we read the pixel
//! at that position and decode the color to find what was clicked.

use std::collections::HashMap;

use glam::Vec2;

/// Element type for pick results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickElementType {
    /// No element type (background or unknown).
    #[default]
    None,
    /// A point in a point cloud.
    Point,
    /// A vertex of a mesh.
    Vertex,
    /// A face of a mesh.
    Face,
    /// An edge of a mesh or curve network.
    Edge,
    /// A cell of a volume mesh.
    Cell,
}

/// Result of a pick operation.
#[derive(Debug, Clone, Default)]
pub struct PickResult {
    /// Whether something was hit.
    pub hit: bool,
    /// The type of structure that was hit (e.g., "`point_cloud`", "`surface_mesh`").
    pub structure_type: String,
    /// The name of the structure that was hit.
    pub structure_name: String,
    /// The index of the element within the structure.
    pub element_index: u64,
    /// The type of element that was hit.
    pub element_type: PickElementType,
    /// The screen position where the pick occurred.
    pub screen_pos: Vec2,
    /// The depth value at the pick location.
    pub depth: f32,
}

/// Decodes a pick color back to an index.
///
/// The color is encoded as RGB where:
/// - R contains bits 16-23
/// - G contains bits 8-15
/// - B contains bits 0-7
#[must_use]
pub fn color_to_index(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Encodes a structure ID and element ID into RGB pick color.
/// GPU uniforms for pick rendering (flat 24-bit global index encoding).
///
/// Each structure is assigned a contiguous range `[global_start, global_start + num_elements)`.
/// The shader encodes `global_start + element_index` as a 24-bit RGB color.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct PickUniforms {
    /// The starting global index for this structure's elements.
    pub global_start: u32,
    /// Point radius for sphere impostor rendering.
    pub point_radius: f32,
    /// Padding to align to 16 bytes.
    pub _padding: [f32; 2],
    /// Model transform of the point positions, as in the point render shader.
    /// Curve networks upload world-space positions and keep the identity.
    pub model: [[f32; 4]; 4],
}

impl Default for PickUniforms {
    fn default() -> Self {
        Self {
            global_start: 0,
            point_radius: 0.01,
            _padding: [0.0; 2],
            model: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

/// GPU uniforms for tube-based curve network pick rendering (flat 24-bit global index encoding).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct TubePickUniforms {
    /// The starting global index for this structure's elements.
    pub global_start: u32,
    /// Tube radius for ray-cylinder intersection.
    pub radius: f32,
    /// Minimum pick radius - ensures curves are always clickable even when very thin.
    pub min_pick_radius: f32,
    /// Padding to align to 16 bytes.
    pub _padding: f32,
}

impl Default for TubePickUniforms {
    fn default() -> Self {
        Self {
            global_start: 0,
            radius: 0.01,
            min_pick_radius: 0.02, // Default minimum pick radius for easier selection
            _padding: 0.0,
        }
    }
}

/// GPU uniforms for mesh pick rendering (flat 24-bit global index encoding).
///
/// Includes the model transform since mesh positions are in object space.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct MeshPickUniforms {
    /// The starting global index for this structure's face elements.
    pub global_start: u32,
    /// Padding to align model matrix to 16-byte boundary.
    pub _padding: [f32; 3],
    /// Model transform matrix.
    pub model: [[f32; 4]; 4],
}

impl Default for MeshPickUniforms {
    fn default() -> Self {
        Self {
            global_start: 0,
            _padding: [0.0; 3],
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

/// Encodes an index as a pick color.
///
/// Returns [R, G, B] where:
/// - R contains bits 16-23
/// - G contains bits 8-15
/// - B contains bits 0-7
#[must_use]
pub fn index_to_color(index: u32) -> [u8; 3] {
    [
        ((index >> 16) & 0xFF) as u8,
        ((index >> 8) & 0xFF) as u8,
        (index & 0xFF) as u8,
    ]
}

/// Number of global pick indices. The pick buffer stores an index in the 24
/// bits of its RGB channels. Index 0 is the background.
pub(crate) const PICK_INDEX_COUNT: u32 = 1 << 24;

/// The global pick indices `[start, start + count)` of one structure.
#[derive(Debug, Clone, Copy)]
struct PickRange {
    start: u32,
    count: u32,
}

/// The pick index ranges of all structures.
///
/// Each structure owns one range of global pick indices, one index for each
/// pickable element. A structure gets a new range when its element count
/// changes. A removed range frees its indices, and later ranges can use them.
/// Thus updates and new structures do not use up the `PICK_INDEX_COUNT` indices.
#[derive(Debug, Default)]
pub(crate) struct PickRanges {
    /// The ranges, keyed by structure type name and structure name.
    ranges: HashMap<(String, String), PickRange>,
    /// True after the warning that the pick indices are not sufficient.
    overflow_warned: bool,
}

impl PickRanges {
    /// Returns the first index of the range of a structure with `count` elements.
    ///
    /// If the structure has a range of the same size, the range stays.
    /// Otherwise the structure gets the first free range of `count` indices.
    pub(crate) fn assign(&mut self, type_name: &str, name: &str, count: u32) -> u32 {
        let key = (type_name.to_owned(), name.to_owned());
        if let Some(range) = self.ranges.get(&key)
            && range.count == count
        {
            return range.start;
        }
        // Remove the old range first. The new range can then use its indices.
        self.ranges.remove(&key);
        let start = self.first_free_start(count);
        self.ranges.insert(key, PickRange { start, count });
        start
    }

    /// Removes the range of a structure. Its indices become free.
    pub(crate) fn remove(&mut self, type_name: &str, name: &str) {
        self.ranges.remove(&(type_name.to_owned(), name.to_owned()));
    }

    /// Keeps only the ranges for which `keep(type_name, name)` is true.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&str, &str) -> bool) {
        self.ranges
            .retain(|(type_name, name), _| keep(type_name, name));
    }

    /// Returns `(type_name, name, local_index)` of the structure that owns a
    /// global pick index. Returns None for the background and for free indices.
    pub(crate) fn lookup(&self, global_index: u32) -> Option<(&str, &str, u32)> {
        self.ranges.iter().find_map(|((type_name, name), range)| {
            let local = global_index.checked_sub(range.start)?;
            (local < range.count).then_some((type_name.as_str(), name.as_str(), local))
        })
    }

    /// Returns the first index of the range of a structure, if it has a range.
    pub(crate) fn start(&self, type_name: &str, name: &str) -> Option<u32> {
        self.ranges
            .get(&(type_name.to_owned(), name.to_owned()))
            .map(|range| range.start)
    }

    /// Returns the start of the first free range of `count` indices.
    ///
    /// If no free range is sufficiently large, the range starts after the last
    /// range. Its indices above the limit then show as other indices in the
    /// pick buffer, so a click can select the wrong element. A warning tells
    /// this once.
    fn first_free_start(&mut self, count: u32) -> u32 {
        let mut used: Vec<(u64, u64)> = self
            .ranges
            .values()
            .map(|range| {
                let start = u64::from(range.start);
                (start, start + u64::from(range.count))
            })
            .collect();
        used.sort_unstable();

        let count = u64::from(count);
        let mut start = 1; // Index 0 is the background.
        for (used_start, used_end) in used {
            if used_start >= start + count {
                break; // The free indices before this range hold the new range.
            }
            start = start.max(used_end);
        }

        if start + count > u64::from(PICK_INDEX_COUNT) && !self.overflow_warned {
            self.overflow_warned = true;
            log::warn!(
                "The scene has more pickable elements than the {} indices of the pick buffer. \
                 A click on some elements selects a different element.",
                PICK_INDEX_COUNT - 1
            );
        }
        u32::try_from(start).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_range_follows_the_element_count() {
        let mut ranges = PickRanges::default();
        assert_eq!(ranges.assign("PointCloud", "a", 2), 1);
        assert_eq!(ranges.assign("PointCloud", "b", 4), 3);
        // The same count keeps the range.
        assert_eq!(ranges.assign("PointCloud", "a", 2), 1);
        // A larger count gets new indices. They do not overlap the range of "b".
        assert_eq!(ranges.assign("PointCloud", "a", 4), 7);
        assert_eq!(ranges.lookup(10), Some(("PointCloud", "a", 3)));
        assert_eq!(ranges.lookup(3), Some(("PointCloud", "b", 0)));
        assert_eq!(ranges.lookup(1), None);
        // A smaller count can use the free indices before the range of "b".
        assert_eq!(ranges.assign("PointCloud", "a", 1), 1);
        assert_eq!(ranges.lookup(7), None);
        assert_eq!(ranges.lookup(0), None);
    }

    #[test]
    fn test_removed_pick_ranges_free_their_indices() {
        let mut ranges = PickRanges::default();
        ranges.assign("PointCloud", "a", 2); // [1, 3)
        ranges.assign("PointCloud", "b", 4); // [3, 7)
        ranges.assign("PointCloud", "c", 1); // [7, 8)
        ranges.retain(|_, name| name != "b");
        assert_eq!(ranges.start("PointCloud", "b"), None);
        assert_eq!(ranges.lookup(4), None);
        // A new range gets the first free indices that hold it.
        assert_eq!(ranges.assign("SurfaceMesh", "d", 3), 3);
        assert_eq!(ranges.assign("SurfaceMesh", "e", 2), 8);
        ranges.remove("PointCloud", "a");
        assert_eq!(ranges.assign("SurfaceMesh", "f", 2), 1);
        // A range without elements owns no index.
        ranges.assign("PointCloud", "empty", 0);
        assert_eq!(ranges.lookup(1), Some(("SurfaceMesh", "f", 0)));
    }

    #[test]
    fn test_pick_ranges_do_not_use_up_the_indices() {
        // Without the reuse of free indices, each loop needs more than the
        // 2^24 indices of the pick buffer.
        let mut ranges = PickRanges::default();
        // A structure that changes its element count in each frame.
        for frame in 0..200u32 {
            let count = 100_000 + (frame % 2) * 100_000;
            let start = ranges.assign("PointCloud", "animated", count);
            assert!(start + count <= PICK_INDEX_COUNT, "frame {frame}");
        }
        // In each frame, a new structure replaces the structure of the frame before.
        for frame in 0..200u32 {
            ranges.retain(|_, name| name == "animated");
            let start = ranges.assign("PointCloud", &format!("frame_{frame}"), 100_000);
            assert!(start + 100_000 <= PICK_INDEX_COUNT, "frame {frame}");
        }
    }

    #[test]
    fn test_pick_ranges_past_the_index_limit() {
        let mut ranges = PickRanges::default();
        let all = PICK_INDEX_COUNT - 1;
        assert_eq!(ranges.assign("PointCloud", "large", all), 1);
        // No free index is left. The new range starts after the last range,
        // and the first range stays as it is.
        assert_eq!(ranges.assign("PointCloud", "more", 10), PICK_INDEX_COUNT);
        assert_eq!(ranges.lookup(all), Some(("PointCloud", "large", all - 1)));
        assert_eq!(ranges.lookup(u32::MAX), None);
    }

    #[test]
    fn test_color_index_roundtrip() {
        // Test various indices
        for index in [
            0,
            1,
            255,
            256,
            65535,
            65536,
            0x00FF_FFFF,
            12_345_678 & 0x00FF_FFFF,
        ] {
            let color = index_to_color(index);
            let decoded = color_to_index(color[0], color[1], color[2]);
            assert_eq!(
                decoded,
                index & 0x00FF_FFFF,
                "Roundtrip failed for index {index}",
            );
        }
    }

    #[test]
    fn test_specific_colors() {
        // Test that specific values encode correctly
        assert_eq!(index_to_color(0), [0, 0, 0]);
        assert_eq!(index_to_color(1), [0, 0, 1]);
        assert_eq!(index_to_color(255), [0, 0, 255]);
        assert_eq!(index_to_color(256), [0, 1, 0]);
        assert_eq!(index_to_color(0x00FF_0000), [255, 0, 0]);
        assert_eq!(index_to_color(0x0000_FF00), [0, 255, 0]);
        assert_eq!(index_to_color(0x0000_00FF), [0, 0, 255]);
    }
}
