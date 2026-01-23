//! Core abstractions for polyscope-rs.
//!
//! This crate provides the fundamental traits and types used throughout polyscope-rs:
//! - [`Structure`] trait for geometric objects (meshes, point clouds, etc.)
//! - [`Quantity`] trait for data associated with structures (scalars, vectors, colors)
//! - Global state management and structure registry
//! - Configuration options and persistent values

pub mod error;
pub mod ground_plane;
pub mod options;
pub mod pick;
pub mod quantity;
pub mod registry;
pub mod state;
pub mod structure;

pub use error::{PolyscopeError, Result};
pub use ground_plane::{GroundPlaneConfig, GroundPlaneMode};
pub use options::Options;
pub use pick::{PickResult, Pickable};
pub use quantity::{Quantity, QuantityKind};
pub use registry::Registry;
pub use state::{with_context, with_context_mut, Context};
pub use structure::{HasQuantities, Structure};

// Re-export glam types for convenience
pub use glam::{Mat4, Vec2, Vec3, Vec4};
