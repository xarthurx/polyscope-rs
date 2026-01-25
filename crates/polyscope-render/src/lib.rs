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
pub mod curve_network_render;
pub mod engine;
pub mod error;
pub mod ground_plane;
pub mod materials;
pub mod pick;
pub mod point_cloud_render;
pub mod screenshot;
pub mod shader;
pub mod shadow_map;
pub mod surface_mesh_render;
pub mod tone_mapping;
pub mod vector_render;

pub use camera::{AxisDirection, Camera, NavigationStyle, ProjectionMode};
pub use color_maps::{ColorMap, ColorMapRegistry};
pub use curve_network_render::{CurveNetworkRenderData, CurveNetworkUniforms};
pub use engine::RenderEngine;
pub use error::{RenderError, RenderResult};
pub use ground_plane::{GroundPlaneRenderData, GroundPlaneUniforms};
pub use materials::{Material, MaterialRegistry, MaterialUniforms};
pub use pick::{color_to_index, index_to_color, PickElementType, PickResult};
pub use point_cloud_render::{PointCloudRenderData, PointUniforms};
pub use screenshot::{save_image, save_to_buffer, ScreenshotError, ScreenshotOptions};
pub use shader::{ShaderBuilder, ShaderProgram};
pub use surface_mesh_render::{MeshUniforms, SurfaceMeshRenderData};
pub use shadow_map::{LightUniforms, ShadowMapPass, SHADOW_MAP_SIZE};
pub use tone_mapping::{ToneMapPass, ToneMapUniforms};
pub use vector_render::{VectorRenderData, VectorUniforms};

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
