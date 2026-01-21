//! UI layer for polyscope-rs.
//!
//! This crate provides the dear-imgui integration and UI widgets:
//! - ImGui context and renderer setup
//! - Structure tree panel
//! - Quantity controls
//! - Color bar widget
//! - Transformation gizmos (via ImGuizmo)
//! - Plotting (via ImPlot)

pub mod color_bar;
pub mod gizmo;
pub mod imgui_integration;
pub mod quantity_ui;
pub mod structure_ui;

pub use imgui_integration::UiContext;
