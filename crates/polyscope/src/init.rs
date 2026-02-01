use crate::Result;

/// Initializes polyscope with default settings.
///
/// This must be called before any other polyscope functions.
pub fn init() -> Result<()> {
    polyscope_core::state::init_context()?;
    log::info!("polyscope-rs initialized");
    Ok(())
}

/// Returns whether polyscope has been initialized.
#[must_use]
pub fn is_initialized() -> bool {
    polyscope_core::state::is_initialized()
}

/// Shuts down polyscope and releases all resources.
pub fn shutdown() {
    polyscope_core::state::shutdown_context();
    log::info!("polyscope-rs shut down");
}

/// Shows the polyscope viewer window.
///
/// This function blocks until the window is closed.
pub fn show() {
    let _ = env_logger::try_init();
    crate::app::run_app();
}

/// Performs one iteration of the main loop.
///
/// Use this for integration with external event loops.
/// Not yet implemented — requires refactoring the winit event loop
/// to support non-blocking single-frame execution.
pub fn frame_tick() {
    unimplemented!("frame_tick() is not yet supported; use show() instead");
}

/// Requests a redraw of the scene.
///
/// Not yet implemented — requires access to the winit event loop proxy.
pub fn request_redraw() {
    unimplemented!("request_redraw() is not yet supported; use show() instead");
}
