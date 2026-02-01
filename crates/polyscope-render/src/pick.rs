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
}

impl Default for PickUniforms {
    fn default() -> Self {
        Self {
            global_start: 0,
            point_radius: 0.01,
            _padding: [0.0; 2],
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

}
