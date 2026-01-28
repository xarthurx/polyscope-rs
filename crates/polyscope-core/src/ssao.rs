//! SSAO (Screen Space Ambient Occlusion) configuration.

use serde::{Deserialize, Serialize};

/// SSAO configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsaoConfig {
    /// Whether SSAO is enabled.
    pub enabled: bool,
    /// Sample radius in world units (relative to length scale).
    pub radius: f32,
    /// Intensity/strength of the effect (0.0 = none, 1.0 = full).
    pub intensity: f32,
    /// Bias to prevent self-occlusion artifacts.
    pub bias: f32,
    /// Number of samples per pixel (higher = better quality, slower).
    pub sample_count: u32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 0.3,    // Smaller radius reduces edge artifacts on simple geometry
            intensity: 1.0, // Linear intensity avoids amplifying edge darkening
            bias: 0.025,
            sample_count: 32, // Higher quality by default
        }
    }
}
