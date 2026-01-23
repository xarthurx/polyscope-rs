//! Ground plane configuration for polyscope.

use serde::{Deserialize, Serialize};

/// Ground plane rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GroundPlaneMode {
    /// No ground plane.
    #[default]
    None,
    /// Tiled ground plane with subtle grid lines.
    Tile,
}

/// Ground plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundPlaneConfig {
    /// Rendering mode.
    pub mode: GroundPlaneMode,
    /// Height of the ground plane (Y coordinate), used when height_is_relative is false.
    pub height: f32,
    /// Whether height is relative to scene bounds (auto-placed below scene).
    pub height_is_relative: bool,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
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
    }

    #[test]
    fn test_ground_plane_config_custom() {
        let config = GroundPlaneConfig {
            mode: GroundPlaneMode::Tile,
            height: -1.5,
            height_is_relative: false,
        };
        assert_eq!(config.mode, GroundPlaneMode::Tile);
        assert_eq!(config.height, -1.5);
        assert!(!config.height_is_relative);
    }
}
