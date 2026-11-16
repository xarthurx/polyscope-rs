//! Configuration options for polyscope.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{GroundPlaneConfig, SsaoConfig};

/// Global configuration options for polyscope.
///
/// The options are the single source of truth for the render look. Change them
/// with `with_context_mut(|ctx| ctx.options...)`. The next frame uses the new
/// values, in the windowed viewer and in headless rendering. The UI panels show
/// and edit the same values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    /// Whether to automatically compute scene extents.
    pub auto_compute_scene_extents: bool,

    /// Ground plane configuration (mode, height, shadow and reflection).
    ///
    /// The window, screenshots and headless rendering draw the ground plane in
    /// the same way. A screenshot with a transparent background keeps the
    /// ground plane, with the alpha of the ground plane shader (as C++
    /// Polyscope does). Set the mode to `GroundPlaneMode::None` to get only
    /// the structures.
    pub ground_plane: GroundPlaneConfig,

    /// Background color (linear RGB).
    pub background_color: Vec3,

    /// Transparency mode.
    ///
    /// `Pretty` renders surface meshes with depth peeling.
    pub transparency_mode: TransparencyMode,

    /// Number of render passes for depth-peeling transparency (`Pretty` mode).
    pub transparency_render_passes: u32,

    /// SSAA (supersampling) factor: 1 (off), 2 or 4. The renderer clamps
    /// the value to the range 1 to 4.
    pub ssaa_factor: u32,

    /// Maximum frame rate of the windowed viewer (0 = unlimited).
    ///
    /// Headless rendering ignores this value.
    pub max_fps: u32,

    /// SSAO configuration. SSAO has no effect when `ssaa_factor` is above 1.
    pub ssao: SsaoConfig,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_compute_scene_extents: true,
            ground_plane: GroundPlaneConfig::default(),
            // White, as in C++ Polyscope (`bgColor`).
            background_color: Vec3::ONE,
            transparency_mode: TransparencyMode::Simple,
            transparency_render_passes: 8,
            ssaa_factor: 1,
            max_fps: 60,
            ssao: SsaoConfig::default(),
        }
    }
}

/// Mode for transparency rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransparencyMode {
    /// Simple transparency (order-dependent, default).
    #[default]
    Simple,
    /// Depth-peeled transparency - correct front-to-back ordering via multiple passes.
    Pretty,
    /// No transparency. The renderer currently treats it like `Simple`.
    None,
}
