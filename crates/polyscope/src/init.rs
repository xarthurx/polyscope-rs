//! Initialization and lifecycle management for polyscope-rs.
//!
//! This module provides the core functions to initialize, run, and shut down
//! the polyscope visualization system.

use crate::Result;

/// Initializes polyscope with default settings.
///
/// This must be called before any other polyscope functions. It sets up the
/// global state required for structure registration and rendering.
///
/// # Errors
///
/// Returns an error if polyscope has already been initialized.
///
/// # Example
///
/// ```no_run
/// use polyscope_rs::*;
///
/// fn main() -> Result<()> {
///     init()?;
///     // Now you can register structures and call show()
///     Ok(())
/// }
/// ```
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
///
/// This clears all registered structures and resets the global state.
/// After calling this, you can call [`init()`] again to reinitialize.
///
/// Note: This is typically not needed as resources are cleaned up when
/// the program exits. It's mainly useful for tests or when you need to
/// reset the visualization state.
pub fn shutdown() {
    polyscope_core::state::shutdown_context();
    log::info!("polyscope-rs shut down");
}

/// Shows the polyscope viewer window.
///
/// This function opens the interactive 3D viewer and blocks until the window
/// is closed (by pressing ESC or clicking the close button).
///
/// Before calling `show()`, you should:
/// 1. Call [`init()`] to initialize polyscope
/// 2. Register structures using `register_*()` functions
/// 3. Optionally add quantities to structures
///
/// # Example
///
/// ```no_run
/// use polyscope_rs::*;
///
/// fn main() -> Result<()> {
///     init()?;
///     
///     // Register some geometry
///     let points = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
///     register_point_cloud("my points", points);
///     
///     // Open the viewer (blocks until closed)
///     show();
///     
///     Ok(())
/// }
/// ```
pub fn show() {
    let _ = env_logger::try_init();
    crate::app::run_app();
}
