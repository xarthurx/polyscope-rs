//! Core abstractions for polyscope-rs.
//!
//! This crate provides the fundamental traits and types used throughout polyscope-rs:
//! - [`Structure`] trait for geometric objects (meshes, point clouds, etc.)
//! - [`Quantity`] trait for data associated with structures (scalars, vectors, colors)
//! - Global state management and structure registry
//! - Configuration options and persistent values

// Documentation lints - internal functions don't need exhaustive panic/error docs
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
// Options structs legitimately have many boolean flags
#![allow(clippy::struct_excessive_bools)]
// Builder patterns return Self which doesn't need must_use
#![allow(clippy::must_use_candidate)]
// Field names like parent_group are descriptive
#![allow(clippy::struct_field_names)]

pub mod error;
pub mod gizmo;
pub mod ground_plane;
pub mod group;
pub mod options;
pub mod pick;
pub mod quantity;
pub mod registry;
pub mod slice_plane;
pub mod ssao;
pub mod state;
pub mod structure;
pub mod tone_mapping;

pub use error::{PolyscopeError, Result};
pub use gizmo::{GizmoAxis, GizmoConfig, GizmoMode, GizmoSpace, GizmoUniforms, Transform};
pub use ground_plane::{GroundPlaneConfig, GroundPlaneMode};
pub use group::Group;
pub use options::Options;
pub use pick::{PickResult, Pickable};
pub use quantity::{Quantity, QuantityKind};
pub use registry::Registry;
pub use slice_plane::{SlicePlane, SlicePlaneUniforms, MAX_SLICE_PLANES};
pub use ssao::SsaoConfig;
pub use state::{with_context, with_context_mut, Context};
pub use structure::{HasQuantities, Structure};
pub use tone_mapping::ToneMappingConfig;

// Re-export glam types for convenience
pub use glam::{Mat4, Vec2, Vec3, Vec4};
