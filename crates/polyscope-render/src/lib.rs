//! Rendering backend for polyscope-rs.
//!
//! This crate provides the wgpu-based rendering engine, including:
//! - GPU resource management (buffers, textures, pipelines)
//! - Shader compilation and management (WGSL)
//! - Material and color map systems
//! - Camera and view management

pub mod buffer;
pub mod camera;
pub mod color_maps;
pub mod engine;
pub mod error;
pub mod ground_plane;
pub mod materials;
pub mod point_cloud_render;
pub mod shader;

pub use camera::Camera;
pub use color_maps::{ColorMap, ColorMapRegistry};
pub use engine::RenderEngine;
pub use error::{RenderError, RenderResult};
pub use materials::{Material, MaterialRegistry};
pub use point_cloud_render::{PointCloudRenderData, PointUniforms};
pub use shader::{ShaderBuilder, ShaderProgram};

/// Render context passed to structures during drawing.
pub struct RenderContext<'a> {
    /// The wgpu device.
    pub device: &'a wgpu::Device,
    /// The wgpu queue.
    pub queue: &'a wgpu::Queue,
    /// The command encoder.
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// The target texture view.
    pub view: &'a wgpu::TextureView,
    /// The depth texture view.
    pub depth_view: &'a wgpu::TextureView,
    /// The current camera.
    pub camera: &'a Camera,
    /// The material registry.
    pub materials: &'a MaterialRegistry,
    /// The color map registry.
    pub color_maps: &'a ColorMapRegistry,
}

impl polyscope_core::structure::RenderContext for RenderContext<'_> {}
