//! Core abstractions for polyscope-rs.
//!
//! This crate provides the fundamental traits and types used throughout polyscope-rs:
//! - [`Structure`] trait for geometric objects (meshes, point clouds, etc.)
//! - [`Quantity`] trait for data associated with structures (scalars, vectors, colors)
//! - Global state management and structure registry
//! - Configuration options and persistent values

// Documentation lints: Detailed error/panic docs will be added as the API stabilizes.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// Struct design: Options and configuration structs naturally have many boolean fields
// and field names may include the struct name for clarity.
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::struct_field_names)]

pub mod error;
pub mod gizmo;
pub mod ground_plane;
pub mod group;
pub mod marching_cubes;
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
pub use marching_cubes::{McmMesh, marching_cubes};
pub use options::Options;
pub use pick::{PickResult, Pickable};
pub use quantity::{Quantity, QuantityKind};
pub use registry::Registry;
pub use slice_plane::{MAX_SLICE_PLANES, SlicePlane, SlicePlaneUniforms};
pub use ssao::SsaoConfig;
pub use state::{Context, MaterialLoadRequest, with_context, with_context_mut};
pub use structure::{HasQuantities, Structure};
pub use tone_mapping::ToneMappingConfig;

// Re-export glam types for convenience
pub use glam::{Mat4, Vec2, Vec3, Vec4};
