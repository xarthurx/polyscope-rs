//! UI layer for polyscope-rs using egui.

pub mod integration;
pub mod panels;
pub mod quantity_ui;
pub mod selection_panel;
pub mod structure_ui;

pub use integration::EguiIntegration;
pub use panels::*;
pub use quantity_ui::*;
pub use selection_panel::*;
pub use structure_ui::*;
