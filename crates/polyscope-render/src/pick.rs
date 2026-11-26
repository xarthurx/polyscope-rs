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
/// bits of its RGB channels. Index 0 is the background. The pick shaders draw
/// a larger index as the background, so it cannot select another element.
pub(crate) const PICK_INDEX_COUNT: u32 = 1 << 24;

/// The owner of a pick range: a structure, or one quantity of a structure.
///
/// A quantity is its index in the quantities of the structure, not its name.
/// Thus two quantities with the same name cannot collide, and neither can
/// structure names that contain `/`.
#[derive(Debug, PartialEq, Eq, Hash)]
struct PickOwner {
    type_name: String,
    structure: String,
    /// The index of the quantity, or `None` for the structure itself.
    quantity: Option<usize>,
}

impl PickOwner {
    fn new(type_name: &str, structure: &str, quantity: Option<usize>) -> Self {
        Self {
            type_name: type_name.to_owned(),
            structure: structure.to_owned(),
            quantity,
        }
    }
}

/// The global pick indices `[start, start + count)` of one owner.
#[derive(Debug, Clone, Copy)]
struct PickRange {
    start: u32,
    count: u32,
}

/// The pick index ranges of one frame.
///
/// The window clears the ranges at the start of each frame. Then each owner
/// gets the next range, with one index for each pickable element. Thus the
/// ranges have no gaps, and a count change, a removed owner or an empty owner
/// leaves no unused indices for later frames.
#[derive(Debug)]
pub(crate) struct PickRanges {
    /// The range of each owner.
    ranges: HashMap<PickOwner, PickRange>,
    /// The first index after the last range.
    next: u64,
    /// True after the warning that the pick indices are not sufficient.
    overflow_warned: bool,
}

impl Default for PickRanges {
    fn default() -> Self {
        Self {
            ranges: HashMap::new(),
            next: 1, // Index 0 is the background.
            overflow_warned: false,
        }
    }
}

impl PickRanges {
    /// Removes all ranges. The next range starts at index 1.
    pub(crate) fn clear(&mut self) {
        self.ranges.clear();
        self.next = 1;
    }

    /// Returns the first index of the range of a structure with `count` elements.
    pub(crate) fn assign(&mut self, type_name: &str, name: &str, count: u32) -> u32 {
        self.assign_owner(PickOwner::new(type_name, name, None), count)
    }

    /// Returns the first index of the range of the quantity with index
    /// `quantity` of a structure, for `count` elements.
    pub(crate) fn assign_quantity(
        &mut self,
        type_name: &str,
        structure: &str,
        quantity: usize,
        count: u32,
    ) -> u32 {
        self.assign_owner(PickOwner::new(type_name, structure, Some(quantity)), count)
    }

    /// Returns the first index of the range of an owner with `count` elements.
    ///
    /// If the owner has a range of the same size, the range stays. Otherwise
    /// the owner gets the range after the last range. The indices of a range
    /// past `PICK_INDEX_COUNT` cannot be picked. A warning tells this once.
    fn assign_owner(&mut self, owner: PickOwner, count: u32) -> u32 {
        if let Some(range) = self.ranges.get(&owner)
            && range.count == count
        {
            return range.start;
        }
        let limit = u64::from(PICK_INDEX_COUNT);
        // A start past the limit stays at the limit, so `start + element`
        // cannot overflow `u32` in the shaders.
        let start = u32::try_from(self.next.min(limit)).unwrap_or(PICK_INDEX_COUNT);
        self.next += u64::from(count);
        if self.next > limit && !self.overflow_warned {
            self.overflow_warned = true;
            log::warn!(
                "The scene has more pickable elements than the {} indices of the pick buffer. \
                 The elements past the limit cannot be picked.",
                PICK_INDEX_COUNT - 1
            );
        }
        self.ranges.insert(owner, PickRange { start, count });
        start
    }

    /// Removes the ranges of a structure and of its quantities.
    pub(crate) fn remove(&mut self, type_name: &str, name: &str) {
        self.ranges
            .retain(|owner, _| owner.type_name != type_name || owner.structure != name);
    }

    /// Returns `(type_name, structure_name, local_index)` of the owner of a
    /// global pick index. For a quantity, it returns the name of its structure.
    /// Returns None for the background and for an index without an owner.
    pub(crate) fn lookup(&self, global_index: u32) -> Option<(&str, &str, u32)> {
        if global_index >= PICK_INDEX_COUNT {
            return None;
        }
        self.ranges.iter().find_map(|(owner, range)| {
            let local = global_index.checked_sub(range.start)?;
            (local < range.count).then_some((
                owner.type_name.as_str(),
                owner.structure.as_str(),
                local,
            ))
        })
    }

    /// Returns the first index of the range of a structure, if it has a range.
    pub(crate) fn start(&self, type_name: &str, name: &str) -> Option<u32> {
        self.ranges
            .get(&PickOwner::new(type_name, name, None))
            .map(|range| range.start)
    }

    /// Returns the first index of the range of the quantity with index
    /// `quantity` of a structure, if it has a range.
    pub(crate) fn quantity_start(
        &self,
        type_name: &str,
        structure: &str,
        quantity: usize,
    ) -> Option<u32> {
        self.ranges
            .get(&PickOwner::new(type_name, structure, Some(quantity)))
            .map(|range| range.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gives the ranges of one frame to point clouds with these element counts.
    fn frame(ranges: &mut PickRanges, owners: &[(&str, u32)]) -> Vec<u32> {
        ranges.clear();
        owners
            .iter()
            .map(|&(name, count)| ranges.assign("PointCloud", name, count))
            .collect()
    }

    #[test]
    fn test_pick_ranges_of_a_frame_have_no_gaps() {
        let mut ranges = PickRanges::default();
        assert_eq!(frame(&mut ranges, &[("a", 2), ("b", 4)]), [1, 3]);
        // The same owner and count keep the range.
        assert_eq!(ranges.assign("PointCloud", "a", 2), 1);
        // A new count in the same frame gets the range after the last range.
        assert_eq!(ranges.assign("PointCloud", "a", 4), 7);
        assert_eq!(ranges.lookup(10), Some(("PointCloud", "a", 3)));
        assert_eq!(ranges.lookup(3), Some(("PointCloud", "b", 0)));
        assert_eq!(ranges.lookup(1), None);
        assert_eq!(ranges.lookup(0), None);
        // The next frame gives the ranges again from index 1.
        assert_eq!(frame(&mut ranges, &[("a", 4), ("b", 4)]), [1, 5]);
        // A removed owner and an empty owner leave no gap.
        assert_eq!(frame(&mut ranges, &[("empty", 0), ("b", 4)]), [1, 1]);
        assert_eq!(ranges.start("PointCloud", "a"), None);
        assert_eq!(ranges.lookup(1), Some(("PointCloud", "b", 0)));
    }

    /// A range that grows does not move past the 24 bits of the pick buffer
    /// while all elements fit. Before, it could start after a later range, and
    /// its indices past 2^24 wrapped to the indices of another structure.
    #[test]
    fn test_count_changes_do_not_wrap_pick_indices() {
        let mut ranges = PickRanges::default();
        frame(&mut ranges, &[("a", 1), ("b", 4_000_000), ("c", 8_000_000)]);
        let starts = frame(&mut ranges, &[("a", 1), ("b", 6_000_000), ("c", 8_000_000)]);
        assert_eq!(starts, [1, 2, 6_000_002]);
        assert!(starts[2] + 8_000_000 <= PICK_INDEX_COUNT);
        // The element that wrapped to index 1 before reads back as itself.
        let color = index_to_color(starts[1] + 4_777_215);
        let read_back = color_to_index(color[0], color[1], color[2]);
        assert_eq!(
            ranges.lookup(read_back),
            Some(("PointCloud", "b", 4_777_215))
        );
    }

    /// A quantity range belongs to the index of the quantity, so two
    /// quantities with the same name cannot collide, and neither can
    /// structure names with `/`. A pick reports the structure.
    #[test]
    fn test_quantity_ranges_report_the_structure() {
        let mut ranges = PickRanges::default();
        // A node quantity and a cell quantity with the same name.
        assert_eq!(ranges.assign_quantity("VolumeGrid", "g", 0, 8), 1);
        assert_eq!(ranges.assign_quantity("VolumeGrid", "g", 1, 1), 9);
        assert_eq!(ranges.lookup(8), Some(("VolumeGrid", "g", 7)));
        assert_eq!(ranges.lookup(9), Some(("VolumeGrid", "g", 0)));
        assert_eq!(ranges.quantity_start("VolumeGrid", "g", 0), Some(1));
        assert_eq!(ranges.start("VolumeGrid", "g"), None);
        // Structure names that contain `/`.
        assert_eq!(ranges.assign_quantity("VolumeGrid", "a/b", 0, 2), 10);
        assert_eq!(ranges.assign_quantity("VolumeGrid", "a", 1, 3), 12);
        assert_eq!(ranges.lookup(11), Some(("VolumeGrid", "a/b", 1)));
        assert_eq!(ranges.lookup(14), Some(("VolumeGrid", "a", 2)));
        // The removal of a structure removes the ranges of its quantities.
        ranges.remove("VolumeGrid", "a");
        assert_eq!(ranges.lookup(14), None);
        assert_eq!(ranges.lookup(11), Some(("VolumeGrid", "a/b", 1)));
    }

    #[test]
    fn test_pick_indices_past_the_limit_cannot_be_picked() {
        let mut ranges = PickRanges::default();
        let last = PICK_INDEX_COUNT - 1;
        assert_eq!(ranges.assign("PointCloud", "large", last - 4), 1);
        // A range can end past the limit. Its first indices stay pickable.
        assert_eq!(ranges.assign("PointCloud", "partial", 10), last - 3);
        assert_eq!(ranges.lookup(last), Some(("PointCloud", "partial", 3)));
        // A range after the limit starts at the limit, so no index wraps.
        assert_eq!(ranges.assign("PointCloud", "past", 10), PICK_INDEX_COUNT);
        assert_eq!(ranges.lookup(PICK_INDEX_COUNT), None);
        assert_eq!(ranges.lookup(1), Some(("PointCloud", "large", 0)));
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
