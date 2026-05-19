//! View save / restore — DTOs, validation, and public API.
//!
//! See `docs/plans/2026-05-19-view-save-restore-design.md` for the design.

use crate::ground_plane::GroundPlaneMode;
use crate::options::TransparencyMode;
use crate::ssao::SsaoConfig;
use serde::{Deserialize, Serialize};

/// Whether to animate the camera transition when loading a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewTransition {
    /// Snap immediately to the loaded pose.
    #[default]
    Instant,
    /// Animate to the loaded pose using the existing camera flight (~0.4 s).
    FlyTo,
}

/// Render-look state (background, ground plane, transparency, SSAO, SSAA).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderState {
    pub background_color: [f32; 3],
    pub ground_plane: GroundPlaneState,
    pub transparency: TransparencyState,
    pub ssao: SsaoConfig,
    pub ssaa_factor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundPlaneState {
    pub enabled: bool,
    pub mode: String,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransparencyState {
    pub enabled: bool,
    pub mode: String,
    pub render_passes: u32,
}

/// Convert a `GroundPlaneMode` to the JSON string used in `RenderState`.
#[must_use]
pub fn ground_plane_mode_name(m: GroundPlaneMode) -> &'static str {
    match m {
        GroundPlaneMode::None => "none",
        GroundPlaneMode::Tile => "tile",
        GroundPlaneMode::ShadowOnly => "shadow_only",
        GroundPlaneMode::TileReflection => "tile_reflection",
    }
}

/// Parse a `GroundPlaneMode` from its JSON string form.
#[must_use]
pub fn parse_ground_plane_mode(s: &str) -> Option<GroundPlaneMode> {
    Some(match s {
        "none" => GroundPlaneMode::None,
        "tile" => GroundPlaneMode::Tile,
        "shadow_only" => GroundPlaneMode::ShadowOnly,
        "tile_reflection" => GroundPlaneMode::TileReflection,
        _ => return None,
    })
}

#[must_use]
pub fn transparency_mode_name(m: TransparencyMode) -> &'static str {
    match m {
        TransparencyMode::Simple => "simple",
        TransparencyMode::Pretty => "pretty",
        TransparencyMode::None => "none",
    }
}

#[must_use]
pub fn parse_transparency_mode(s: &str) -> Option<TransparencyMode> {
    Some(match s {
        "simple" => TransparencyMode::Simple,
        "pretty" => TransparencyMode::Pretty,
        "none" => TransparencyMode::None,
        _ => return None,
    })
}

// Placeholder — replaced in Task 5.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_plane_mode_names() {
        assert_eq!(
            ground_plane_mode_name(GroundPlaneMode::TileReflection),
            "tile_reflection"
        );
        assert_eq!(
            parse_ground_plane_mode("tile_reflection"),
            Some(GroundPlaneMode::TileReflection)
        );
        assert_eq!(parse_ground_plane_mode("xyz"), None);
    }

    #[test]
    fn test_transparency_mode_names() {
        assert_eq!(transparency_mode_name(TransparencyMode::Pretty), "pretty");
        assert_eq!(
            parse_transparency_mode("simple"),
            Some(TransparencyMode::Simple)
        );
    }

    #[test]
    fn test_render_state_roundtrip() {
        let r = RenderState {
            background_color: [0.1, 0.2, 0.3],
            ground_plane: GroundPlaneState {
                enabled: true,
                mode: "tile_reflection".to_string(),
                height: 1.5,
            },
            transparency: TransparencyState {
                enabled: true,
                mode: "simple".to_string(),
                render_passes: 6,
            },
            ssao: SsaoConfig::default(),
            ssaa_factor: 2,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: RenderState = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
