//! Configuration options for polyscope.

use glam::Vec4;
use serde::{Deserialize, Serialize};

use crate::SsaoConfig;

/// Global configuration options for polyscope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    /// Whether to automatically compute scene extents.
    pub auto_compute_scene_extents: bool,

    /// Whether to invoke user callback during rendering.
    pub invoke_user_callback_for_nested_show: bool,

    /// Whether to give focus to the polyscope window.
    pub give_focus_on_show: bool,

    /// Whether the ground plane is enabled.
    pub ground_plane_enabled: bool,

    /// Ground plane mode (shadow, tile, etc.).
    pub ground_plane_mode: GroundPlaneMode,

    /// Ground plane height (world coordinates).
    pub ground_plane_height: f32,

    /// Background color.
    pub background_color: Vec4,

    /// Whether to enable transparency.
    pub transparency_enabled: bool,

    /// Transparency mode.
    pub transparency_mode: TransparencyMode,

    /// Number of render passes for depth-peeling transparency (Pretty mode).
    pub transparency_render_passes: u32,

    /// SSAA (supersampling) factor.
    pub ssaa_factor: u32,

    /// Maximum frames per second (0 = unlimited).
    pub max_fps: u32,

    /// SSAO configuration.
    pub ssao: SsaoConfig,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_compute_scene_extents: true,
            invoke_user_callback_for_nested_show: false,
            give_focus_on_show: true,
            ground_plane_enabled: true,
            ground_plane_mode: GroundPlaneMode::ShadowOnly,
            ground_plane_height: 0.0,
            background_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            transparency_enabled: true,
            transparency_mode: TransparencyMode::Simple,
            transparency_render_passes: 8,
            ssaa_factor: 1,
            max_fps: 60,
            ssao: SsaoConfig::default(),
        }
    }
}

/// Mode for the ground plane rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GroundPlaneMode {
    /// No ground plane.
    None,
    /// Ground plane with shadow only.
    #[default]
    ShadowOnly,
    /// Ground plane with tile pattern.
    Tile,
    /// Ground plane with solid color.
    SolidColor,
}

/// Mode for transparency rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransparencyMode {
    /// Simple transparency (order-dependent, default).
    #[default]
    Simple,
    /// Depth-peeled transparency - correct front-to-back ordering via multiple passes.
    Pretty,
    /// No transparency.
    None,
}
