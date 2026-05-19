//! `App`-side view-state gather/scatter: snapshot the App's render-look + camera
//! into a `ViewState`, and apply a `ViewState` back across the distributed
//! ownership.

use polyscope_core::view_state::{
    CameraStateOwned, GroundPlaneState, RenderState, TransparencyState, ViewState, ViewTransition,
};

use super::App;

impl App {
    /// Build a `ViewState` using App-owned fields + a borrowed `Options`.
    ///
    /// Lets per-frame publish gather + write inside a single context lock.
    /// Public consumers should use the free `current_view_state()` function
    /// in `polyscope_core::view_state` instead, which reads the published
    /// snapshot via the global Context.
    pub(crate) fn view_state_from_options(&self, options: &polyscope_core::Options) -> ViewState {
        let camera_state: CameraStateOwned = if let Some(engine) = self.engine.as_ref() {
            (&polyscope_render::CameraState::from_camera(&engine.camera)).into()
        } else {
            let cam = polyscope_render::Camera::new(1.0);
            (&polyscope_render::CameraState::from_camera(&cam)).into()
        };

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
                mode: self.ground_plane.mode,
                height: self.ground_plane.height,
            },
            transparency: TransparencyState {
                enabled: options.transparency_enabled,
                mode: options.transparency_mode,
                render_passes: options.transparency_render_passes,
            },
            ssao: options.ssao.clone(),
            ssaa_factor: options.ssaa_factor,
        };

        ViewState {
            version: ViewState::CURRENT_VERSION,
            camera: camera_state,
            render,
        }
    }

    /// Apply a view state to this App. For `Instant`, fields snap. For
    /// `FlyTo`, the camera animates; non-camera fields apply immediately.
    pub fn apply_view_state(&mut self, state: &ViewState, transition: ViewTransition) {
        // Only apply camera if an engine exists, and only then mark camera_fitted —
        // setting the flag without an engine would silently disable future auto-fit
        // once an engine is created.
        if let Some(engine) = self.engine.as_mut() {
            let cs: polyscope_render::CameraState = (&state.camera).into();
            cs.apply(&mut engine.camera, transition);
            self.camera_fitted = true;
        }

        self.background_color = crate::Vec3::new(
            state.render.background_color[0],
            state.render.background_color[1],
            state.render.background_color[2],
        );

        self.ground_plane.mode = state.render.ground_plane.mode;
        self.ground_plane.height = state.render.ground_plane.height;

        polyscope_core::state::with_context_mut(|ctx| {
            ctx.options.ssao = state.render.ssao.clone();
            ctx.options.ssaa_factor = state.render.ssaa_factor;
            ctx.options.transparency_enabled = state.render.transparency.enabled;
            ctx.options.transparency_render_passes = state.render.transparency.render_passes;
            ctx.options.transparency_mode = state.render.transparency.mode;
        });
    }
}
