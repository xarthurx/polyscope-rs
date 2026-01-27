//! Rendering backend for polyscope-rs.
//!
//! This crate provides the wgpu-based rendering engine, including:
//! - GPU resource management (buffers, textures, pipelines)
//! - Shader compilation and management (WGSL)
//! - Material and color map systems
//! - Camera and view management

// Graphics code intentionally uses casts for indices, colors, and coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
// wgpu pipeline code uses Default::default() for struct update syntax
#![allow(clippy::default_trait_access)]
// Documentation lints - internal functions don't need exhaustive panic/error docs
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
// Builder patterns return Self which doesn't need must_use
#![allow(clippy::must_use_candidate)]
// Trait implementations may not use all params
#![allow(clippy::unused_self)]
// Large pipeline setup functions
#![allow(clippy::too_many_lines)]
// Use approximate constants for clarity
#![allow(clippy::approx_constant)]

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
pub mod reflection;
pub mod reflection_pass;
pub mod screenshot;
pub mod shader;
pub mod shadow_map;
pub mod slice_mesh_render;
pub mod slice_plane_render;
pub mod ssao_pass;
pub mod oit_pass;
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
pub use pick::{
    color_to_index, decode_pick_id, encode_pick_id, index_to_color, PickElementType, PickResult,
    PickUniforms,
};
pub use point_cloud_render::{PointCloudRenderData, PointUniforms};
pub use screenshot::{save_image, save_to_buffer, ScreenshotError, ScreenshotOptions};
pub use shader::{ShaderBuilder, ShaderProgram};
pub use surface_mesh_render::{MeshUniforms, SurfaceMeshRenderData};
pub use reflection::{ground_reflection_matrix, reflection_matrix};
pub use reflection_pass::{ReflectionPass, ReflectionUniforms};
pub use shadow_map::{LightUniforms, ShadowMapPass, SHADOW_MAP_SIZE};
pub use ssao_pass::{SsaoPass, SsaoUniforms};
pub use oit_pass::OitCompositePass;
pub use tone_mapping::{ToneMapPass, ToneMapUniforms};
pub use vector_render::{VectorRenderData, VectorUniforms};
pub use slice_mesh_render::SliceMeshRenderData;
pub use slice_plane_render::{
    create_slice_plane_bind_group_layout, create_slice_plane_pipeline, PlaneRenderUniforms,
    SlicePlaneRenderData,
};

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
