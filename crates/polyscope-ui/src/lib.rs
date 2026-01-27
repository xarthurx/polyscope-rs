//! UI layer for polyscope-rs using egui.

// Type casts in UI code: Conversions between pixel coordinates and
// screen dimensions (f32, f64, u32, usize) are intentional. Values
// are bounded by screen resolution.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
// Documentation lints: Detailed error/panic docs will be added as the API stabilizes.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// Function length: UI layout and rendering functions are legitimately complex.
#![allow(clippy::too_many_lines)]
// Struct design: Settings structs naturally have many boolean options.
#![allow(clippy::struct_excessive_bools)]
// Method design: Some methods take &self for API consistency even when not using it.
#![allow(clippy::unused_self)]

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
