//! View save / restore — DTOs, validation, and public API.
//!
//! See `docs/plans/2026-05-19-view-save-restore-design.md` for the design.

use crate::ground_plane::{GroundPlaneConfig, GroundPlaneMode};
use crate::options::{Options, TransparencyMode};
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
///
/// It mirrors the render-look fields of [`Options`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderState {
    pub background_color: [f32; 3],
    pub ground_plane: GroundPlaneState,
    pub transparency: TransparencyState,
    pub ssao: SsaoConfig,
    pub ssaa_factor: u32,
}

/// Ground plane part of a [`RenderState`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundPlaneState {
    pub mode: GroundPlaneMode,
    pub height: f32,
    pub height_is_relative: bool,
    pub shadow_darkness: f32,
    pub reflection_intensity: f32,
}

/// Transparency part of a [`RenderState`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransparencyState {
    pub mode: TransparencyMode,
    pub render_passes: u32,
}

impl RenderState {
    /// Copies the render-look fields of the options into a render state.
    #[must_use]
    pub fn from_options(options: &Options) -> Self {
        let gp = &options.ground_plane;
        Self {
            background_color: options.background_color.to_array(),
            ground_plane: GroundPlaneState {
                mode: gp.mode,
                height: gp.height,
                height_is_relative: gp.height_is_relative,
                shadow_darkness: gp.shadow_darkness,
                reflection_intensity: gp.reflection_intensity,
            },
            transparency: TransparencyState {
                mode: options.transparency_mode,
                render_passes: options.transparency_render_passes,
            },
            ssao: options.ssao.clone(),
            ssaa_factor: options.ssaa_factor,
        }
    }

    /// Writes this render state into the render-look fields of the options.
    pub fn apply_to_options(&self, options: &mut Options) {
        let gp = &self.ground_plane;
        options.background_color = glam::Vec3::from_array(self.background_color);
        options.ground_plane = GroundPlaneConfig {
            mode: gp.mode,
            height: gp.height,
            height_is_relative: gp.height_is_relative,
            shadow_darkness: gp.shadow_darkness,
            reflection_intensity: gp.reflection_intensity,
        };
        options.transparency_mode = self.transparency.mode;
        options.transparency_render_passes = self.transparency.render_passes;
        options.ssao = self.ssao.clone();
        options.ssaa_factor = self.ssaa_factor;
    }
}

use crate::error::{PolyscopeError, Result};
use crate::state::{with_context, with_context_mut};

/// Returns the latest view-state snapshot from the running App,
/// or an error if no frame has been rendered yet.
pub fn current_view_state() -> Result<ViewState> {
    with_context(|ctx| {
        ctx.view_state_snapshot()
            .cloned()
            .ok_or(PolyscopeError::NoActiveView)
    })
}

/// Queues a view-state application for the next frame.
pub fn apply_view_state(state: &ViewState, transition: ViewTransition) -> Result<()> {
    ViewState::validate(state)?;
    with_context_mut(|ctx| {
        ctx.set_pending_view_apply(state.clone(), transition);
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

/// View state used when no snapshot is available yet: a default camera and
/// the render look of the current options.
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
        render: with_context(|ctx| RenderState::from_options(&ctx.options)),
    }
}

/// Parse a JSON view-state and queue it for application.
///
/// Missing fields preserve the current snapshot's values. If no snapshot is
/// available, missing camera fields fall back to a built-in default camera
/// and missing render fields keep the current options.
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

#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone, Default, Deserialize)]
struct PartialRenderState {
    background_color: Option<[f32; 3]>,
    ground_plane: Option<PartialGroundPlaneState>,
    transparency: Option<PartialTransparencyState>,
    ssao: Option<SsaoConfig>,
    ssaa_factor: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PartialGroundPlaneState {
    mode: Option<GroundPlaneMode>,
    height: Option<f32>,
    height_is_relative: Option<bool>,
    shadow_darkness: Option<f32>,
    reflection_intensity: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PartialTransparencyState {
    mode: Option<TransparencyMode>,
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
        let cam_p = partial.camera.unwrap_or_default();
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

        let render_p = partial.render.unwrap_or_default();
        let gp_p = render_p.ground_plane.unwrap_or_default();
        let tp_p = render_p.transparency.unwrap_or_default();

        let render = RenderState {
            background_color: render_p
                .background_color
                .unwrap_or(current.render.background_color),
            ground_plane: GroundPlaneState {
                mode: gp_p.mode.unwrap_or(current.render.ground_plane.mode),
                height: gp_p.height.unwrap_or(current.render.ground_plane.height),
                height_is_relative: gp_p
                    .height_is_relative
                    .unwrap_or(current.render.ground_plane.height_is_relative),
                shadow_darkness: gp_p
                    .shadow_darkness
                    .unwrap_or(current.render.ground_plane.shadow_darkness),
                reflection_intensity: gp_p
                    .reflection_intensity
                    .unwrap_or(current.render.ground_plane.reflection_intensity),
            },
            transparency: TransparencyState {
                mode: tp_p.mode.unwrap_or(current.render.transparency.mode),
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
        const KNOWN_SSAA: &[u32] = &[1, 2, 4];

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
        if s.render.transparency.render_passes == 0 {
            return invalid("render.transparency.render_passes must be ≥ 1");
        }
        if !s.render.ground_plane.height.is_finite() {
            return invalid("render.ground_plane.height must be finite");
        }
        if !(0.0..=1.0).contains(&s.render.ground_plane.shadow_darkness) {
            return invalid("render.ground_plane.shadow_darkness must be in [0, 1]");
        }
        if !(0.0..=1.0).contains(&s.render.ground_plane.reflection_intensity) {
            return invalid("render.ground_plane.reflection_intensity must be in [0, 1]");
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
    fn test_ground_plane_mode_serializes_snake_case() {
        // Direct enum serde — replaces removed name/parse helper pair.
        let v = serde_json::to_value(GroundPlaneMode::TileReflection).unwrap();
        assert_eq!(v, "tile_reflection");
        let parsed: GroundPlaneMode = serde_json::from_str("\"tile_reflection\"").unwrap();
        assert_eq!(parsed, GroundPlaneMode::TileReflection);
    }

    #[test]
    fn test_transparency_mode_serializes_snake_case() {
        let v = serde_json::to_value(TransparencyMode::Pretty).unwrap();
        assert_eq!(v, "pretty");
        let parsed: TransparencyMode = serde_json::from_str("\"simple\"").unwrap();
        assert_eq!(parsed, TransparencyMode::Simple);
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
                    mode: GroundPlaneMode::TileReflection,
                    height: 0.0,
                    height_is_relative: true,
                    shadow_darkness: 0.4,
                    reflection_intensity: 0.25,
                },
                transparency: TransparencyState {
                    mode: TransparencyMode::Simple,
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

    // The tests below share the global context. The lock makes them run one
    // at a time, so one test cannot change the view buffers of another test.
    static CONTEXT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Returns the guard that serializes the tests that use the global context.
    ///
    /// Keep the guard alive for the full test.
    fn lock_context() -> std::sync::MutexGuard<'static, ()> {
        // A failed test poisons the lock. The other tests must still run.
        CONTEXT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn ensure_initialized() {
        // Tests share the global OnceLock-backed context. Initialize it once.
        let _ = crate::state::init_context();
    }

    fn clear_view_buffers() {
        ensure_initialized();
        with_context_mut(crate::state::Context::clear_view_buffers);
    }

    fn install_snapshot(s: ViewState) {
        ensure_initialized();
        with_context_mut(|ctx| ctx.set_view_state_snapshot(s));
    }

    #[test]
    fn test_current_view_state_returns_error_when_no_snapshot() {
        let _guard = lock_context();
        clear_view_buffers();
        let err = current_view_state().unwrap_err();
        assert!(matches!(err, crate::error::PolyscopeError::NoActiveView));
    }

    #[test]
    fn test_save_view_to_json_roundtrip_via_buffers() {
        let _guard = lock_context();
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
        let _guard = lock_context();
        install_snapshot(dummy_view_state());
        let json = save_view_to_json().unwrap();
        load_view_from_json(&json, ViewTransition::Instant).unwrap();
        with_context(|ctx| {
            let (state, transition) = ctx.pending_view_apply().expect("pending should be set");
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
                mode: GroundPlaneMode::TileReflection,
                height: 1.5,
                height_is_relative: false,
                shadow_darkness: 0.6,
                reflection_intensity: 0.5,
            },
            transparency: TransparencyState {
                mode: TransparencyMode::Pretty,
                render_passes: 6,
            },
            ssao: SsaoConfig::default(),
            ssaa_factor: 2,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: RenderState = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn test_render_state_options_roundtrip() {
        let options = Options {
            background_color: glam::Vec3::new(0.2, 0.3, 0.4),
            ground_plane: GroundPlaneConfig {
                mode: GroundPlaneMode::ShadowOnly,
                height: -1.0,
                height_is_relative: false,
                shadow_darkness: 0.7,
                reflection_intensity: 0.1,
            },
            transparency_mode: TransparencyMode::Pretty,
            transparency_render_passes: 4,
            ssaa_factor: 2,
            ssao: SsaoConfig {
                enabled: true,
                ..SsaoConfig::default()
            },
            ..Options::default()
        };
        let state = RenderState::from_options(&options);
        let mut other = Options::default();
        state.apply_to_options(&mut other);
        assert_eq!(RenderState::from_options(&other), state);
        assert_eq!(other.ground_plane, options.ground_plane);
        assert_eq!(other.background_color, options.background_color);
    }
}
