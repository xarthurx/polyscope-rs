//! UI layer for polyscope-rs using egui.

// Graphics and UI code intentionally uses casts
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
// Documentation lints - internal functions don't need exhaustive docs
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
// Builder patterns return Self which doesn't need must_use
#![allow(clippy::must_use_candidate)]
// Trait implementations may not use all params
#![allow(clippy::unused_self)]
// Large functions acceptable in UI code
#![allow(clippy::too_many_lines)]
// Rendering functions may have many parameters
#![allow(clippy::too_many_arguments)]
// Reference to mut Vec is acceptable in UI code
#![allow(clippy::ptr_arg)]

pub mod gizmo;
pub mod integration;
pub mod panels;
pub mod quantity_ui;
pub mod selection_panel;
pub mod structure_ui;

pub use gizmo::TransformGizmo;
pub use integration::EguiIntegration;
pub use panels::*;
pub use quantity_ui::*;
pub use selection_panel::*;
pub use structure_ui::*;
