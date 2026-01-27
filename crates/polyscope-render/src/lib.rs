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
pub mod oit_pass;
pub mod pick;
pub mod point_cloud_render;
pub mod reflection;
pub mod reflection_pass;
pub mod screenshot;
pub mod shader;
pub mod shadow_map;
pub mod slice_mesh_render;
pub mod slice_plane_render;
pub mod ssao_pass;
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
pub use oit_pass::OitCompositePass;
pub use pick::{
    color_to_index, decode_pick_id, encode_pick_id, index_to_color, PickElementType, PickResult,
    PickUniforms,
};
pub use point_cloud_render::{PointCloudRenderData, PointUniforms};
pub use reflection::{ground_reflection_matrix, reflection_matrix};
pub use reflection_pass::{ReflectionPass, ReflectionUniforms};
pub use screenshot::{save_image, save_to_buffer, ScreenshotError, ScreenshotOptions};
pub use shader::{ShaderBuilder, ShaderProgram};
pub use shadow_map::{LightUniforms, ShadowMapPass, SHADOW_MAP_SIZE};
pub use slice_mesh_render::SliceMeshRenderData;
pub use slice_plane_render::{
    create_slice_plane_bind_group_layout, create_slice_plane_pipeline, PlaneRenderUniforms,
    SlicePlaneRenderData,
};
pub use ssao_pass::{SsaoPass, SsaoUniforms};
pub use surface_mesh_render::{MeshUniforms, SurfaceMeshRenderData};
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
