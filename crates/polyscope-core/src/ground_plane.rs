//! Ground plane configuration for polyscope.

use serde::{Deserialize, Serialize};

/// Ground plane rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GroundPlaneMode {
    /// No ground plane.
    None,
    /// Tiled ground plane with subtle grid lines.
    #[default]
    Tile,
    /// Shadow only (no visible ground plane, just shadows).
    ShadowOnly,
    /// Tiled ground plane with reflections.
    TileReflection,
}

/// Ground plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundPlaneConfig {
    /// Rendering mode.
    pub mode: GroundPlaneMode,
    /// Height of the ground plane (Y coordinate), used when `height_is_relative` is false.
    pub height: f32,
    /// Whether height is relative to scene bounds (auto-placed below scene).
    pub height_is_relative: bool,
    /// Shadow blur iterations (0-5).
    pub shadow_blur_iters: u32,
    /// Shadow darkness (0.0 = no shadow, 1.0 = full black).
    pub shadow_darkness: f32,
    /// Reflection intensity (0.0 = none, 1.0 = full mirror).
    pub reflection_intensity: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::Tile,
            height: 0.0,
            height_is_relative: true,
            shadow_blur_iters: 2,
            shadow_darkness: 0.4,
            reflection_intensity: 0.25,
        }
    }
}
