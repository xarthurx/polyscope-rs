//! Ground plane configuration for polyscope.

use serde::{Deserialize, Serialize};

/// Ground plane rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GroundPlaneMode {
    /// No ground plane.
    #[default]
    None,
    /// Tiled/checkered ground plane.
    Tile,
}

/// Ground plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundPlaneConfig {
    /// Rendering mode.
    pub mode: GroundPlaneMode,
    /// Height of the ground plane (Y coordinate).
    pub height: f32,
    /// Whether height is relative to scene bounds.
    pub height_is_relative: bool,
    /// Primary tile color.
    pub color1: [f32; 3],
    /// Secondary tile color (checker).
    pub color2: [f32; 3],
    /// Tile size (world units).
    pub tile_size: f32,
    /// Transparency (0 = opaque, 1 = fully transparent).
    pub transparency: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
            color1: [0.75, 0.75, 0.75],
            color2: [0.55, 0.55, 0.55],
            tile_size: 1.0,
            transparency: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_plane_mode_default() {
        let mode = GroundPlaneMode::default();
        assert_eq!(mode, GroundPlaneMode::None);
    }

    #[test]
    fn test_ground_plane_config_default() {
        let config = GroundPlaneConfig::default();
        assert_eq!(config.mode, GroundPlaneMode::None);
        assert_eq!(config.height, 0.0);
        assert!(config.height_is_relative);
        assert_eq!(config.color1, [0.75, 0.75, 0.75]);
        assert_eq!(config.color2, [0.55, 0.55, 0.55]);
        assert_eq!(config.tile_size, 1.0);
        assert_eq!(config.transparency, 0.0);
    }

    #[test]
    fn test_ground_plane_config_custom() {
        let config = GroundPlaneConfig {
            mode: GroundPlaneMode::Tile,
            height: -1.5,
            height_is_relative: false,
            color1: [0.8, 0.8, 0.8],
            color2: [0.4, 0.4, 0.4],
            tile_size: 2.0,
            transparency: 0.3,
        };
        assert_eq!(config.mode, GroundPlaneMode::Tile);
        assert_eq!(config.height, -1.5);
        assert!(!config.height_is_relative);
        assert_eq!(config.tile_size, 2.0);
        assert_eq!(config.transparency, 0.3);
    }
}
