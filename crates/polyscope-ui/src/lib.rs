//! UI layer for polyscope-rs using egui.

pub mod integration;
pub mod panels;
pub mod structure_ui;

pub use integration::EguiIntegration;
pub use panels::*;
pub use structure_ui::*;
