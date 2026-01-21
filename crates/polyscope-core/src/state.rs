//! Global state management for polyscope.

use std::sync::{OnceLock, RwLock};

use glam::Vec3;

use crate::error::{PolyscopeError, Result};
use crate::options::Options;
use crate::registry::Registry;

/// Global context singleton.
static CONTEXT: OnceLock<RwLock<Context>> = OnceLock::new();

/// The global context containing all polyscope state.
pub struct Context {
    /// Whether polyscope has been initialized.
    pub initialized: bool,

    /// The structure registry.
    pub registry: Registry,

    /// Global options.
    pub options: Options,

    /// Representative length scale for all registered structures.
    pub length_scale: f32,

    /// Axis-aligned bounding box for all registered structures.
    pub bounding_box: (Vec3, Vec3),

    // User callback will be added later with proper thread-safety handling
}

impl Default for Context {
    fn default() -> Self {
        Self {
            initialized: false,
            registry: Registry::new(),
            options: Options::default(),
            length_scale: 1.0,
            bounding_box: (Vec3::ZERO, Vec3::ONE),
        }
    }
}

impl Context {
    /// Computes the center of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.bounding_box.0 + self.bounding_box.1) * 0.5
    }

    /// Updates the global bounding box and length scale from all structures.
    pub fn update_extents(&mut self) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut has_extent = false;

        for structure in self.registry.iter() {
            if let Some((bb_min, bb_max)) = structure.bounding_box() {
                min = min.min(bb_min);
                max = max.max(bb_max);
                has_extent = true;
            }
        }

        if has_extent {
            self.bounding_box = (min, max);
            self.length_scale = (max - min).length();
        } else {
            self.bounding_box = (Vec3::ZERO, Vec3::ONE);
            self.length_scale = 1.0;
        }
    }
}

/// Initializes the global context.
///
/// This should be called once at the start of the program.
pub fn init_context() -> Result<()> {
    let context = RwLock::new(Context::default());

    CONTEXT
        .set(context)
        .map_err(|_| PolyscopeError::AlreadyInitialized)?;

    with_context_mut(|ctx| {
        ctx.initialized = true;
    });

    Ok(())
}

/// Returns whether the context has been initialized.
pub fn is_initialized() -> bool {
    CONTEXT
        .get()
        .and_then(|lock| lock.read().ok())
        .map_or(false, |ctx| ctx.initialized)
}

/// Access the global context for reading.
///
/// # Panics
///
/// Panics if polyscope has not been initialized.
pub fn with_context<F, R>(f: F) -> R
where
    F: FnOnce(&Context) -> R,
{
    let lock = CONTEXT.get().expect("polyscope not initialized");
    let guard = lock.read().expect("context lock poisoned");
    f(&guard)
}

/// Access the global context for writing.
///
/// # Panics
///
/// Panics if polyscope has not been initialized.
pub fn with_context_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Context) -> R,
{
    let lock = CONTEXT.get().expect("polyscope not initialized");
    let mut guard = lock.write().expect("context lock poisoned");
    f(&mut guard)
}

/// Try to access the global context for reading.
///
/// Returns `None` if polyscope has not been initialized.
pub fn try_with_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Context) -> R,
{
    let lock = CONTEXT.get()?;
    let guard = lock.read().ok()?;
    Some(f(&guard))
}

/// Try to access the global context for writing.
///
/// Returns `None` if polyscope has not been initialized.
pub fn try_with_context_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Context) -> R,
{
    let lock = CONTEXT.get()?;
    let mut guard = lock.write().ok()?;
    Some(f(&mut guard))
}

/// Shuts down the global context.
///
/// Note: Due to `OnceLock` semantics, the context cannot be re-initialized
/// after shutdown in the same process.
pub fn shutdown_context() {
    if let Some(lock) = CONTEXT.get() {
        if let Ok(mut ctx) = lock.write() {
            ctx.initialized = false;
            ctx.registry.clear();
        }
    }
}
