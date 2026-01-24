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
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the exposure value.
    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.exposure = exposure;
        self
    }

    /// Sets the white level.
    pub fn with_white_level(mut self, white_level: f32) -> Self {
        self.white_level = white_level;
        self
    }

    /// Sets the gamma value.
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tone_mapping_default() {
        let config = ToneMappingConfig::default();
        assert_eq!(config.exposure, 1.0);
        assert_eq!(config.white_level, 1.0);
        assert_eq!(config.gamma, 2.2);
    }

    #[test]
    fn test_tone_mapping_builder() {
        let config = ToneMappingConfig::new()
            .with_exposure(1.5)
            .with_gamma(2.0);
        assert_eq!(config.exposure, 1.5);
        assert_eq!(config.gamma, 2.0);
    }
}
