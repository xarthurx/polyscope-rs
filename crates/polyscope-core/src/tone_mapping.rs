//! Tone mapping configuration.

use serde::{Deserialize, Serialize};

/// Tone mapping configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneMappingConfig {
    /// Exposure multiplier (default 1.0).
    pub exposure: f32,
    /// White level for highlight compression (default 1.0).
    pub white_level: f32,
    /// Gamma correction exponent (default 2.2).
    pub gamma: f32,
}

impl Default for ToneMappingConfig {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            white_level: 1.0,
            gamma: 2.2,
        }
    }
}

impl ToneMappingConfig {
    /// Creates a new tone mapping configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the exposure value.
    #[must_use]
    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.exposure = exposure;
        self
    }

    /// Sets the white level.
    #[must_use]
    pub fn with_white_level(mut self, white_level: f32) -> Self {
        self.white_level = white_level;
        self
    }

    /// Sets the gamma value.
    #[must_use]
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma;
        self
    }
}
