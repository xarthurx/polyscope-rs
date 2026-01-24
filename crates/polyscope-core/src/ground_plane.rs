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
    /// Shadow only (no visible ground plane, just shadows).
    ShadowOnly,
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
    /// Shadow blur iterations (0-5).
    pub shadow_blur_iters: u32,
    /// Shadow darkness (0.0 = no shadow, 1.0 = full black).
    pub shadow_darkness: f32,
}

impl Default for GroundPlaneConfig {
    fn default() -> Self {
        Self {
            mode: GroundPlaneMode::None,
            height: 0.0,
            height_is_relative: true,
            shadow_blur_iters: 2,
            shadow_darkness: 0.4,
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
            shadow_blur_iters: 3,
            shadow_darkness: 0.6,
        };
        assert_eq!(config.mode, GroundPlaneMode::Tile);
        assert_eq!(config.height, -1.5);
        assert!(!config.height_is_relative);
        assert_eq!(config.shadow_blur_iters, 3);
        assert_eq!(config.shadow_darkness, 0.6);
    }
}
