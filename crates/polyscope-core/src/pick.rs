//! Picking and selection system.

use glam::Vec3;

/// Result of a pick/selection operation.
#[derive(Debug, Clone)]
pub struct PickResult {
    /// The type of structure that was picked.
    pub structure_type: String,

    /// The name of the structure that was picked.
    pub structure_name: String,

    /// The index of the element that was picked (vertex, face, etc.).
    pub element_index: usize,

    /// The world position of the pick point.
    pub world_position: Vec3,

    /// The depth of the pick point.
    pub depth: f32,
}

impl PickResult {
    /// Creates a new pick result.
    pub fn new(
        structure_type: impl Into<String>,
        structure_name: impl Into<String>,
        element_index: usize,
        world_position: Vec3,
        depth: f32,
    ) -> Self {
        Self {
            structure_type: structure_type.into(),
            structure_name: structure_name.into(),
            element_index,
            world_position,
            depth,
        }
    }
}

/// Trait for objects that support picking/selection.
pub trait Pickable {
    /// Encodes this object's pick data into a color.
    fn encode_pick_color(&self, element_index: usize) -> [u8; 4];

    /// Decodes a pick color back to an element index.
    fn decode_pick_color(&self, color: [u8; 4]) -> Option<usize>;

    /// Returns the total number of pickable elements.
    fn num_pickable_elements(&self) -> usize;
}
