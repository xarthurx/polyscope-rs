//! `App`-side view-state gather/scatter: snapshot the App's render-look + camera
//! into a `ViewState`, and apply a `ViewState` back across the distributed
//! ownership.

use polyscope_core::view_state::{
    CameraStateOwned, GroundPlaneState, RenderState, TransparencyState, ViewState, ViewTransition,
    ground_plane_mode_name, parse_ground_plane_mode, parse_transparency_mode,
    transparency_mode_name,
};

use super::App;

impl App {
    /// Snapshot the App's full view state. If a render engine is attached,
    /// camera state is taken from it; otherwise camera fields default to a
    /// fresh `Camera::new(1.0)`.
    #[allow(dead_code)] // consumed by frame loop (Task 8) and UI dispatch (Task 11)
    pub fn current_view_state(&self) -> ViewState {
        let camera_state: CameraStateOwned = if let Some(engine) = self.engine.as_ref() {
            (&polyscope_render::CameraState::from_camera(&engine.camera)).into()
        } else {
            let cam = polyscope_render::Camera::new(1.0);
            (&polyscope_render::CameraState::from_camera(&cam)).into()
        };

        let (ssao, transparency_mode, transparency_passes, transparency_enabled, ssaa_factor) =
            polyscope_core::state::with_context(|ctx| {
                (
                    ctx.options.ssao.clone(),
                    ctx.options.transparency_mode,
                    ctx.options.transparency_render_passes,
                    ctx.options.transparency_enabled,
                    ctx.options.ssaa_factor,
                )
            });

        let render = RenderState {
            background_color: [
                self.background_color.x,
                self.background_color.y,
                self.background_color.z,
            ],
            ground_plane: GroundPlaneState {
                enabled: !matches!(
                    self.ground_plane.mode,
                    polyscope_core::GroundPlaneMode::None
                ),
                mode: ground_plane_mode_name(self.ground_plane.mode).to_owned(),
                height: self.ground_plane.height,
            },
            transparency: TransparencyState {
                enabled: transparency_enabled,
                mode: transparency_mode_name(transparency_mode).to_owned(),
                render_passes: transparency_passes,
            },
            ssao,
            ssaa_factor,
        };

        ViewState {
            version: ViewState::CURRENT_VERSION,
            camera: camera_state,
            render,
        }
    }

    /// Apply a view state to this App. For `Instant`, fields snap. For
    /// `FlyTo`, the camera animates; non-camera fields apply immediately.
    #[allow(dead_code)] // consumed by frame loop (Task 8) and headless (Task 9)
    pub fn apply_view_state(&mut self, state: &ViewState, transition: ViewTransition) {
        if let Some(engine) = self.engine.as_mut() {
            let cs: polyscope_render::CameraState = (&state.camera).into();
            cs.apply(&mut engine.camera, transition);
        }

        self.background_color = crate::Vec3::new(
            state.render.background_color[0],
            state.render.background_color[1],
            state.render.background_color[2],
        );

        if let Some(mode) = parse_ground_plane_mode(&state.render.ground_plane.mode) {
            self.ground_plane.mode = mode;
            self.ground_plane.height = state.render.ground_plane.height;
        }

        polyscope_core::state::with_context_mut(|ctx| {
            ctx.options.ssao = state.render.ssao.clone();
            ctx.options.ssaa_factor = state.render.ssaa_factor;
            ctx.options.transparency_enabled = state.render.transparency.enabled;
            ctx.options.transparency_render_passes = state.render.transparency.render_passes;
            if let Some(m) = parse_transparency_mode(&state.render.transparency.mode) {
                ctx.options.transparency_mode = m;
            }
        });

        // After applying, mark camera-fitted so headless / first-frame auto-fit becomes a no-op.
        self.camera_fitted = true;
    }
}
