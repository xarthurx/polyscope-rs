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

use crate::error::{PolyscopeError, Result};
use crate::state::{with_context, with_context_mut};

/// Returns the latest view-state snapshot from the running App,
/// or an error if no frame has been rendered yet.
pub fn current_view_state() -> Result<ViewState> {
    with_context(|ctx| {
        ctx.view_state_snapshot
            .clone()
            .ok_or(PolyscopeError::NoActiveView)
    })
}

/// Queues a view-state application for the next frame.
pub fn apply_view_state(state: &ViewState, transition: ViewTransition) -> Result<()> {
    ViewState::validate(state)?;
    with_context_mut(|ctx| {
        ctx.pending_view_apply = Some((state.clone(), transition));
    });
    Ok(())
}

/// Serialize the current view state to a JSON string.
pub fn save_view_to_json() -> Result<String> {
    let state = current_view_state()?;
    Ok(serde_json::to_string_pretty(&state)?)
}

/// Save the current view state to a file.
pub fn save_view_to_file(path: impl AsRef<std::path::Path>) -> Result<()> {
    let json = save_view_to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Built-in default view state used when no snapshot is available yet.
fn fallback_view_state() -> ViewState {
    ViewState {
        version: ViewState::CURRENT_VERSION,
        camera: CameraStateOwned {
            position: [0.0, 0.0, 3.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
            near: 0.01,
            far: 1000.0,
            projection_mode: "perspective".to_string(),
            ortho_scale: 1.0,
            navigation_style: "turntable".to_string(),
            up_direction: "pos_y".to_string(),
            front_direction: "neg_z".to_string(),
        },
        render: RenderState {
            background_color: [1.0, 1.0, 1.0],
            ground_plane: GroundPlaneState {
                enabled: true,
                mode: "tile_reflection".to_string(),
                height: 0.0,
            },
            transparency: TransparencyState {
                enabled: true,
                mode: "simple".to_string(),
                render_passes: 8,
            },
            ssao: SsaoConfig::default(),
            ssaa_factor: 1,
        },
    }
}

/// Parse a JSON view-state and queue it for application.
///
/// Missing fields preserve the current snapshot's values. If no snapshot is
/// available, missing fields fall back to a built-in default.
pub fn load_view_from_json(json: &str, transition: ViewTransition) -> Result<()> {
    let current = current_view_state().unwrap_or_else(|_| fallback_view_state());
    let merged = ViewState::from_json_partial_then_validate(json, &current)?;
    apply_view_state(&merged, transition)
}

/// Load and queue a view state from a file path.
pub fn load_view_from_file(
    path: impl AsRef<std::path::Path>,
    transition: ViewTransition,
) -> Result<()> {
    let json = std::fs::read_to_string(path)?;
    load_view_from_json(&json, transition)
}

/// Plain DTO mirroring `polyscope_render::CameraState` but living in core
/// (avoids a render → core dependency for the wrapper).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraStateOwned {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub projection_mode: String,
    pub ortho_scale: f32,
    pub navigation_style: String,
    pub up_direction: String,
    pub front_direction: String,
}

/// Top-level view state. Composes camera + render-look.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewState {
    pub version: u32,
    pub camera: CameraStateOwned,
    pub render: RenderState,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialViewState {
    version: u32,
    #[serde(default)]
    camera: Option<PartialCameraState>,
    #[serde(default)]
    render: Option<PartialRenderState>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialCameraState {
    position: Option<[f32; 3]>,
    target: Option<[f32; 3]>,
    up: Option<[f32; 3]>,
    fov: Option<f32>,
    near: Option<f32>,
    far: Option<f32>,
    projection_mode: Option<String>,
    ortho_scale: Option<f32>,
    navigation_style: Option<String>,
    up_direction: Option<String>,
    front_direction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialRenderState {
    background_color: Option<[f32; 3]>,
    ground_plane: Option<PartialGroundPlaneState>,
    transparency: Option<PartialTransparencyState>,
    ssao: Option<SsaoConfig>,
    ssaa_factor: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialGroundPlaneState {
    enabled: Option<bool>,
    mode: Option<String>,
    height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialTransparencyState {
    enabled: Option<bool>,
    mode: Option<String>,
    render_passes: Option<u32>,
}

impl ViewState {
    /// JSON format version this build writes and accepts.
    pub const CURRENT_VERSION: u32 = 1;

    /// Parse a JSON string permissively (missing fields preserve `current`),
    /// then validate the resulting full state.
    pub fn from_json_partial_then_validate(json: &str, current: &ViewState) -> Result<ViewState> {
        let partial: PartialViewState = serde_json::from_str(json)?;
        if partial.version != Self::CURRENT_VERSION {
            return Err(PolyscopeError::InvalidViewState(format!(
                "unsupported view version {} (this build accepts {})",
                partial.version,
                Self::CURRENT_VERSION
            )));
        }
        let merged = Self::merge(partial, current);
        Self::validate(&merged)?;
        Ok(merged)
    }

    fn merge(partial: PartialViewState, current: &ViewState) -> ViewState {
        let cam_p = partial.camera.unwrap_or(PartialCameraState {
            position: None,
            target: None,
            up: None,
            fov: None,
            near: None,
            far: None,
            projection_mode: None,
            ortho_scale: None,
            navigation_style: None,
            up_direction: None,
            front_direction: None,
        });
        let camera = CameraStateOwned {
            position: cam_p.position.unwrap_or(current.camera.position),
            target: cam_p.target.unwrap_or(current.camera.target),
            up: cam_p.up.unwrap_or(current.camera.up),
            fov: cam_p.fov.unwrap_or(current.camera.fov),
            near: cam_p.near.unwrap_or(current.camera.near),
            far: cam_p.far.unwrap_or(current.camera.far),
            projection_mode: cam_p
                .projection_mode
                .unwrap_or_else(|| current.camera.projection_mode.clone()),
            ortho_scale: cam_p.ortho_scale.unwrap_or(current.camera.ortho_scale),
            navigation_style: cam_p
                .navigation_style
                .unwrap_or_else(|| current.camera.navigation_style.clone()),
            up_direction: cam_p
                .up_direction
                .unwrap_or_else(|| current.camera.up_direction.clone()),
            front_direction: cam_p
                .front_direction
                .unwrap_or_else(|| current.camera.front_direction.clone()),
        };

        let render_p = partial.render.unwrap_or(PartialRenderState {
            background_color: None,
            ground_plane: None,
            transparency: None,
            ssao: None,
            ssaa_factor: None,
        });
        let gp_p = render_p.ground_plane.unwrap_or(PartialGroundPlaneState {
            enabled: None,
            mode: None,
            height: None,
        });
        let tp_p = render_p.transparency.unwrap_or(PartialTransparencyState {
            enabled: None,
            mode: None,
            render_passes: None,
        });

        let render = RenderState {
            background_color: render_p
                .background_color
                .unwrap_or(current.render.background_color),
            ground_plane: GroundPlaneState {
                enabled: gp_p.enabled.unwrap_or(current.render.ground_plane.enabled),
                mode: gp_p
                    .mode
                    .unwrap_or_else(|| current.render.ground_plane.mode.clone()),
                height: gp_p.height.unwrap_or(current.render.ground_plane.height),
            },
            transparency: TransparencyState {
                enabled: tp_p.enabled.unwrap_or(current.render.transparency.enabled),
                mode: tp_p
                    .mode
                    .unwrap_or_else(|| current.render.transparency.mode.clone()),
                render_passes: tp_p
                    .render_passes
                    .unwrap_or(current.render.transparency.render_passes),
            },
            ssao: render_p.ssao.unwrap_or_else(|| current.render.ssao.clone()),
            ssaa_factor: render_p.ssaa_factor.unwrap_or(current.render.ssaa_factor),
        };

        ViewState {
            version: Self::CURRENT_VERSION,
            camera,
            render,
        }
    }

    /// Validate that all field values are sensible.
    pub fn validate(s: &ViewState) -> Result<()> {
        const KNOWN_PROJ: &[&str] = &["perspective", "orthographic"];
        const KNOWN_NAV: &[&str] = &[
            "turntable",
            "free",
            "planar",
            "arcball",
            "first_person",
            "none",
        ];
        const KNOWN_AXIS: &[&str] = &["pos_x", "neg_x", "pos_y", "neg_y", "pos_z", "neg_z"];
        const KNOWN_GP: &[&str] = &["none", "tile", "shadow_only", "tile_reflection"];
        const KNOWN_TR: &[&str] = &["simple", "pretty", "none"];
        const KNOWN_SSAA: &[u32] = &[1, 2, 4, 8];

        let invalid = |reason: &str| Err(PolyscopeError::InvalidViewState(reason.to_string()));

        let finite_arr = |a: &[f32; 3]| a.iter().all(|x| x.is_finite());
        if !finite_arr(&s.camera.position) {
            return invalid("camera.position has non-finite values");
        }
        if !finite_arr(&s.camera.target) {
            return invalid("camera.target has non-finite values");
        }
        if !finite_arr(&s.camera.up) {
            return invalid("camera.up has non-finite values");
        }
        if !s.camera.fov.is_finite() || s.camera.fov <= 0.0 || s.camera.fov >= std::f32::consts::PI
        {
            return invalid("camera.fov must be in (0, π)");
        }
        if !s.camera.near.is_finite() || s.camera.near <= 0.0 {
            return invalid("camera.near must be > 0");
        }
        if !s.camera.far.is_finite() || s.camera.far <= s.camera.near {
            return invalid("camera.far must be > camera.near");
        }
        if !s.camera.ortho_scale.is_finite() || s.camera.ortho_scale <= 0.0 {
            return invalid("camera.ortho_scale must be > 0");
        }
        if !KNOWN_PROJ.contains(&s.camera.projection_mode.as_str()) {
            return invalid(&format!(
                "unknown projection_mode: {}",
                s.camera.projection_mode
            ));
        }
        if !KNOWN_NAV.contains(&s.camera.navigation_style.as_str()) {
            return invalid(&format!(
                "unknown navigation_style: {}",
                s.camera.navigation_style
            ));
        }
        if !KNOWN_AXIS.contains(&s.camera.up_direction.as_str()) {
            return invalid(&format!("unknown up_direction: {}", s.camera.up_direction));
        }
        if !KNOWN_AXIS.contains(&s.camera.front_direction.as_str()) {
            return invalid(&format!(
                "unknown front_direction: {}",
                s.camera.front_direction
            ));
        }

        if !s.render.background_color.iter().all(|x| x.is_finite()) {
            return invalid("render.background_color has non-finite values");
        }
        if !KNOWN_GP.contains(&s.render.ground_plane.mode.as_str()) {
            return invalid(&format!(
                "unknown ground_plane.mode: {}",
                s.render.ground_plane.mode
            ));
        }
        if !KNOWN_TR.contains(&s.render.transparency.mode.as_str()) {
            return invalid(&format!(
                "unknown transparency.mode: {}",
                s.render.transparency.mode
            ));
        }
        if s.render.transparency.render_passes == 0 {
            return invalid("render.transparency.render_passes must be ≥ 1");
        }
        if !s.render.ground_plane.height.is_finite() {
            return invalid("render.ground_plane.height must be finite");
        }
        if !KNOWN_SSAA.contains(&s.render.ssaa_factor) {
            return invalid(&format!("render.ssaa_factor must be one of {KNOWN_SSAA:?}"));
        }
        if s.render.ssao.sample_count == 0 || s.render.ssao.sample_count > 256 {
            return invalid("render.ssao.sample_count must be in [1, 256]");
        }
        if !(s.render.ssao.radius.is_finite() && s.render.ssao.radius > 0.0) {
            return invalid("render.ssao.radius must be > 0");
        }
        Ok(())
    }
}

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

    fn dummy_view_state() -> ViewState {
        ViewState {
            version: ViewState::CURRENT_VERSION,
            camera: CameraStateOwned {
                position: [0.0, 0.0, 3.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                fov: 0.7854,
                near: 0.01,
                far: 1000.0,
                projection_mode: "perspective".to_string(),
                ortho_scale: 1.0,
                navigation_style: "turntable".to_string(),
                up_direction: "pos_y".to_string(),
                front_direction: "neg_z".to_string(),
            },
            render: RenderState {
                background_color: [1.0, 1.0, 1.0],
                ground_plane: GroundPlaneState {
                    enabled: true,
                    mode: "tile_reflection".to_string(),
                    height: 0.0,
                },
                transparency: TransparencyState {
                    enabled: true,
                    mode: "simple".to_string(),
                    render_passes: 8,
                },
                ssao: SsaoConfig::default(),
                ssaa_factor: 1,
            },
        }
    }

    fn make_view_state_with_fov(fov: f32) -> ViewState {
        let mut s = dummy_view_state();
        s.camera.fov = fov;
        s
    }

    #[test]
    fn test_view_state_unknown_version_rejected() {
        let json = r#"{"version": 99}"#;
        let err =
            ViewState::from_json_partial_then_validate(json, &dummy_view_state()).unwrap_err();
        match err {
            crate::error::PolyscopeError::InvalidViewState(reason) => {
                assert!(reason.contains("version"), "{reason}");
            }
            other => panic!("expected InvalidViewState, got {other:?}"),
        }
    }

    #[test]
    fn test_view_state_missing_version_rejected() {
        let json = r#"{}"#;
        let err =
            ViewState::from_json_partial_then_validate(json, &dummy_view_state()).unwrap_err();
        match err {
            crate::error::PolyscopeError::InvalidViewState(_)
            | crate::error::PolyscopeError::JsonError(_) => {}
            other => panic!("expected InvalidViewState or JsonError, got {other:?}"),
        }
    }

    #[test]
    fn test_view_state_missing_fields_preserve_current() {
        let json = r#"{"version": 1}"#;
        let current = make_view_state_with_fov(0.5);
        let merged = ViewState::from_json_partial_then_validate(json, &current).unwrap();
        assert!((merged.camera.fov - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_view_state_partial_camera_overrides_only_specified() {
        let current = make_view_state_with_fov(0.5);
        let json = r#"{"version": 1, "camera": {"fov": 1.2}}"#;
        let merged = ViewState::from_json_partial_then_validate(json, &current).unwrap();
        assert!((merged.camera.fov - 1.2).abs() < 1e-6);
        assert_eq!(
            merged.camera.projection_mode,
            current.camera.projection_mode
        );
    }

    #[test]
    fn test_validation_rejects_nonfinite_fov() {
        let mut s = dummy_view_state();
        s.camera.fov = f32::NAN;
        let err = ViewState::validate(&s).unwrap_err();
        assert!(matches!(
            err,
            crate::error::PolyscopeError::InvalidViewState(_)
        ));
    }

    #[test]
    fn test_validation_rejects_fov_out_of_range() {
        let mut s = dummy_view_state();
        s.camera.fov = 0.0;
        assert!(ViewState::validate(&s).is_err());
        s.camera.fov = std::f32::consts::PI + 0.1;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_near_geq_far() {
        let mut s = dummy_view_state();
        s.camera.near = 10.0;
        s.camera.far = 1.0;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_zero_ssaa_factor() {
        let mut s = dummy_view_state();
        s.render.ssaa_factor = 0;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_unknown_enum_strings() {
        let mut s = dummy_view_state();
        s.camera.projection_mode = "definitely_not_a_mode".to_string();
        assert!(ViewState::validate(&s).is_err());
    }

    fn ensure_initialized() {
        // Tests share the global OnceLock-backed context. Initialize it once.
        let _ = crate::state::init_context();
    }

    fn clear_view_buffers() {
        ensure_initialized();
        with_context_mut(|ctx| {
            ctx.view_state_snapshot = None;
            ctx.pending_view_apply = None;
        });
    }

    fn install_snapshot(s: ViewState) {
        ensure_initialized();
        with_context_mut(|ctx| {
            ctx.view_state_snapshot = Some(s);
        });
    }

    #[test]
    fn test_current_view_state_returns_error_when_no_snapshot() {
        clear_view_buffers();
        let err = current_view_state().unwrap_err();
        assert!(matches!(err, crate::error::PolyscopeError::NoActiveView));
    }

    #[test]
    fn test_save_view_to_json_roundtrip_via_buffers() {
        install_snapshot(dummy_view_state());
        let json = save_view_to_json().unwrap();
        assert!(json.contains("\"version\""));
        // Reparse via partial (the public surface) — confirms the JSON is well-formed.
        let _: ViewState =
            ViewState::from_json_partial_then_validate(&json, &dummy_view_state()).unwrap();
        clear_view_buffers();
    }

    #[test]
    fn test_load_view_from_json_queues_pending() {
        install_snapshot(dummy_view_state());
        let json = save_view_to_json().unwrap();
        load_view_from_json(&json, ViewTransition::Instant).unwrap();
        with_context(|ctx| {
            let (state, transition) = ctx
                .pending_view_apply
                .as_ref()
                .expect("pending should be set");
            assert_eq!(*transition, ViewTransition::Instant);
            assert!((state.camera.fov - 0.7854).abs() < 1e-5);
        });
        clear_view_buffers();
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
