//! Pipeline creation and accessor functions for the render engine.
//!
//! This module is split into submodules by logical grouping:
//! - `structure`: Pipelines for core visualization structures (point, vector, mesh, curve)
//! - `effects`: Pipelines for visual effects (shadow, reflection)
//! - `volume`: Pipelines for volume visualization (gridcube)

mod effects;
mod structure;
mod volume;
