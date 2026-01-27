//! Pick buffer rendering for element selection.
//!
//! The pick buffer is an offscreen framebuffer where each element is rendered
//! with a unique color encoding its ID. When the user clicks, we read the pixel
//! at that position and decode the color to find what was clicked.

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
}

/// Result of a pick operation.
#[derive(Debug, Clone, Default)]
pub struct PickResult {
    /// Whether something was hit.
    pub hit: bool,
    /// The type of structure that was hit (e.g., "point_cloud", "surface_mesh").
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
pub fn color_to_index(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Encodes a structure ID and element ID into RGB pick color.
///
/// Uses 12 bits for structure ID (max 4096) and 12 bits for element ID (max 4096).
/// Layout: R[7:0] = struct[11:4], G[7:4] = struct[3:0], G[3:0] = elem[11:8], B[7:0] = elem[7:0]
pub fn encode_pick_id(structure_id: u16, element_id: u16) -> [u8; 3] {
    let s = structure_id & 0xFFF; // 12 bits max
    let e = element_id & 0xFFF; // 12 bits max
    [
        (s >> 4) as u8,                        // R: struct bits 11-4
        (((s & 0xF) << 4) | (e >> 8)) as u8,   // G: struct bits 3-0 + elem bits 11-8
        (e & 0xFF) as u8,                      // B: elem bits 7-0
    ]
}

/// Decodes RGB pick color back to structure ID and element ID.
pub fn decode_pick_id(r: u8, g: u8, b: u8) -> (u16, u16) {
    let structure_id = ((r as u16) << 4) | ((g as u16) >> 4);
    let element_id = (((g & 0xF) as u16) << 8) | (b as u16);
    (structure_id, element_id)
}

/// GPU uniforms for pick rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct PickUniforms {
    /// The structure ID to encode in pick colors.
    pub structure_id: u32,
    /// Point radius for sphere impostor rendering.
    pub point_radius: f32,
    /// Padding to align to 16 bytes.
    pub _padding: [f32; 2],
}

impl Default for PickUniforms {
    fn default() -> Self {
        Self {
            structure_id: 0,
            point_radius: 0.01,
            _padding: [0.0; 2],
        }
    }
}

/// Encodes an index as a pick color.
///
/// Returns [R, G, B] where:
/// - R contains bits 16-23
/// - G contains bits 8-15
/// - B contains bits 0-7
pub fn index_to_color(index: u32) -> [u8; 3] {
    [
        ((index >> 16) & 0xFF) as u8,
        ((index >> 8) & 0xFF) as u8,
        (index & 0xFF) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_index_roundtrip() {
        // Test various indices
        for index in [0, 1, 255, 256, 65535, 65536, 0xFFFFFF, 12345678 & 0xFFFFFF] {
            let color = index_to_color(index);
            let decoded = color_to_index(color[0], color[1], color[2]);
            assert_eq!(
                decoded,
                index & 0xFFFFFF,
                "Roundtrip failed for index {}",
                index
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
        assert_eq!(index_to_color(0xFF0000), [255, 0, 0]);
        assert_eq!(index_to_color(0x00FF00), [0, 255, 0]);
        assert_eq!(index_to_color(0x0000FF), [0, 0, 255]);
    }

    #[test]
    fn test_pick_result_default() {
        let result = PickResult::default();
        assert!(!result.hit);
        assert!(result.structure_type.is_empty());
        assert!(result.structure_name.is_empty());
        assert_eq!(result.element_index, 0);
        assert_eq!(result.element_type, PickElementType::None);
        assert_eq!(result.screen_pos, Vec2::ZERO);
        assert_eq!(result.depth, 0.0);
    }

    #[test]
    fn test_pick_element_type() {
        // Test that all variants exist and are distinct
        assert_ne!(PickElementType::None, PickElementType::Point);
        assert_ne!(PickElementType::Point, PickElementType::Vertex);
        assert_ne!(PickElementType::Vertex, PickElementType::Face);
        assert_ne!(PickElementType::Face, PickElementType::Edge);

        // Test default
        assert_eq!(PickElementType::default(), PickElementType::None);

        // Test copy
        let element = PickElementType::Face;
        let copied = element;
        assert_eq!(element, copied);
    }

    #[test]
    fn test_encode_decode_pick_id_roundtrip() {
        // Test various combinations
        let cases = [
            (1, 0),
            (1, 1),
            (0xFFF, 0xFFF), // max values
            (123, 456),
            (4095, 4095),
        ];
        for (struct_id, elem_id) in cases {
            let encoded = encode_pick_id(struct_id, elem_id);
            let (decoded_struct, decoded_elem) = decode_pick_id(encoded[0], encoded[1], encoded[2]);
            assert_eq!(
                decoded_struct, struct_id,
                "struct_id mismatch for ({}, {})",
                struct_id, elem_id
            );
            assert_eq!(
                decoded_elem, elem_id,
                "elem_id mismatch for ({}, {})",
                struct_id, elem_id
            );
        }
    }

    #[test]
    fn test_encode_pick_id_background() {
        let encoded = encode_pick_id(0, 0);
        assert_eq!(encoded, [0, 0, 0], "Background should encode to black");
    }
}
