//! The main rendering engine.

mod pick;
mod pipelines;
mod postprocessing;
mod rendering;
mod textures;

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use wgpu::util::DeviceExt;

use polyscope_core::slice_plane::{SlicePlaneUniforms, MAX_SLICE_PLANES};

use crate::camera::Camera;
use crate::color_maps::ColorMapRegistry;
use crate::error::{RenderError, RenderResult};
use crate::ground_plane::GroundPlaneRenderData;
use crate::materials::{self, MatcapTextureSet, MaterialRegistry};
use crate::shadow_map::ShadowMapPass;
use crate::slice_plane_render::SlicePlaneRenderData;
use crate::tone_mapping::ToneMapPass;

/// Camera uniforms for GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct CameraUniforms {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _padding: f32,
}

impl Default for CameraUniforms {
    fn default() -> Self {
        Self {
            view: glam::Mat4::IDENTITY.to_cols_array_2d(),
            proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 5.0],
            _padding: 0.0,
        }
    }
}

/// The main rendering engine backed by wgpu.
pub struct RenderEngine {
    /// The wgpu instance.
    pub instance: wgpu::Instance,
    /// The wgpu adapter.
    pub adapter: wgpu::Adapter,
    /// The wgpu device.
    pub device: wgpu::Device,
    /// The wgpu queue.
    pub queue: wgpu::Queue,
    /// The render surface (None for headless).
    pub surface: Option<wgpu::Surface<'static>>,
    /// Surface configuration.
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Depth texture.
    pub depth_texture: wgpu::Texture,
    /// Depth texture view.
    pub depth_view: wgpu::TextureView,
    /// Depth-only texture view (for SSAO sampling, excludes stencil aspect).
    pub(crate) depth_only_view: wgpu::TextureView,
    /// Material registry.
    pub materials: MaterialRegistry,
    /// Color map registry.
    pub color_maps: ColorMapRegistry,
    /// Matcap bind group layout (Group 2: 4 textures + 1 sampler).
    pub matcap_bind_group_layout: wgpu::BindGroupLayout,
    /// Matcap texture sets keyed by material name.
    pub matcap_textures: HashMap<String, MatcapTextureSet>,
    /// Main camera.
    pub camera: Camera,
    /// Current viewport width.
    pub width: u32,
    /// Current viewport height.
    pub height: u32,
    /// Point cloud render pipeline.
    pub point_pipeline: Option<wgpu::RenderPipeline>,
    /// Point cloud bind group layout.
    pub point_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Camera uniform buffer.
    pub camera_buffer: wgpu::Buffer,
    /// Slice plane uniform buffer.
    pub slice_plane_buffer: wgpu::Buffer,
    /// Slice plane bind group layout (shared by all structure shaders).
    pub slice_plane_bind_group_layout: wgpu::BindGroupLayout,
    /// Slice plane bind group (updated each frame).
    pub slice_plane_bind_group: wgpu::BindGroup,
    /// Vector arrow render pipeline.
    pub vector_pipeline: Option<wgpu::RenderPipeline>,
    /// Vector bind group layout.
    pub vector_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Surface mesh render pipeline (alpha blending, depth write enabled).
    pub mesh_pipeline: Option<wgpu::RenderPipeline>,
    /// Surface mesh depth/normal-only pipeline (Pretty mode prepass).
    pub mesh_depth_normal_pipeline: Option<wgpu::RenderPipeline>,
    /// Mesh bind group layout.
    pub(crate) mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network edge render pipeline (line rendering).
    pub curve_network_edge_pipeline: Option<wgpu::RenderPipeline>,
    /// Curve network edge bind group layout.
    pub(crate) curve_network_edge_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network tube render pipeline (cylinder impostor rendering).
    pub curve_network_tube_pipeline: Option<wgpu::RenderPipeline>,
    /// Curve network tube compute pipeline (generates bounding box geometry).
    pub curve_network_tube_compute_pipeline: Option<wgpu::ComputePipeline>,
    /// Curve network tube render bind group layout.
    pub(crate) curve_network_tube_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network tube compute bind group layout.
    pub(crate) curve_network_tube_compute_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Ground plane render pipeline.
    pub(crate) ground_plane_pipeline: wgpu::RenderPipeline,
    /// Ground plane bind group layout.
    pub(crate) ground_plane_bind_group_layout: wgpu::BindGroupLayout,
    /// Ground plane render data (lazily initialized).
    pub(crate) ground_plane_render_data: Option<GroundPlaneRenderData>,
    /// Slice plane visualization pipeline.
    pub(crate) slice_plane_vis_pipeline: wgpu::RenderPipeline,
    /// Slice plane visualization bind group layout.
    pub(crate) slice_plane_vis_bind_group_layout: wgpu::BindGroupLayout,
    /// Slice plane render data (per-plane, lazily initialized).
    pub(crate) slice_plane_render_data: Vec<SlicePlaneRenderData>,
    /// Screenshot capture texture (lazily initialized).
    pub(crate) screenshot_texture: Option<wgpu::Texture>,
    /// Screenshot capture buffer (lazily initialized).
    pub(crate) screenshot_buffer: Option<wgpu::Buffer>,
    /// Screenshot HDR texture for rendering (lazily initialized).
    pub(crate) screenshot_hdr_texture: Option<wgpu::Texture>,
    /// Screenshot HDR texture view.
    pub(crate) screenshot_hdr_view: Option<wgpu::TextureView>,
    /// HDR intermediate texture for tone mapping.
    pub(crate) hdr_texture: Option<wgpu::Texture>,
    /// HDR texture view.
    pub(crate) hdr_view: Option<wgpu::TextureView>,
    /// Normal G-buffer texture for SSAO.
    pub(crate) normal_texture: Option<wgpu::Texture>,
    /// Normal G-buffer texture view.
    pub(crate) normal_view: Option<wgpu::TextureView>,
    /// SSAO noise texture (4x4 random rotation vectors).
    pub(crate) ssao_noise_texture: Option<wgpu::Texture>,
    /// SSAO noise texture view.
    pub(crate) ssao_noise_view: Option<wgpu::TextureView>,
    /// SSAO pass.
    pub(crate) ssao_pass: Option<crate::ssao_pass::SsaoPass>,
    /// SSAO output texture (blurred result).
    pub(crate) ssao_output_texture: Option<wgpu::Texture>,
    /// SSAO output texture view.
    pub(crate) ssao_output_view: Option<wgpu::TextureView>,
    /// Depth peeling transparency pass.
    pub(crate) depth_peel_pass: Option<crate::depth_peel_pass::DepthPeelPass>,
    /// Tone mapping post-processing pass.
    pub(crate) tone_map_pass: Option<ToneMapPass>,
    /// SSAA (supersampling) pass for anti-aliasing.
    pub(crate) ssaa_pass: Option<crate::ssaa_pass::SsaaPass>,
    /// Current SSAA factor (1 = off, 2 = 2x, 4 = 4x).
    pub(crate) ssaa_factor: u32,
    /// Intermediate HDR texture for SSAA (screen resolution, used after downsampling).
    pub(crate) ssaa_intermediate_texture: Option<wgpu::Texture>,
    /// Intermediate HDR texture view.
    pub(crate) ssaa_intermediate_view: Option<wgpu::TextureView>,
    /// Shadow map pass for ground plane shadows.
    pub(crate) shadow_map_pass: Option<ShadowMapPass>,
    /// Shadow render pipeline (depth-only, renders objects from light's perspective).
    pub(crate) shadow_pipeline: Option<wgpu::RenderPipeline>,
    /// Shadow bind group layout for shadow pass rendering.
    pub(crate) shadow_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Reflection pass for ground plane reflections.
    pub(crate) reflection_pass: Option<crate::reflection_pass::ReflectionPass>,
    /// Stencil pipeline for ground plane reflection mask.
    pub(crate) ground_stencil_pipeline: Option<wgpu::RenderPipeline>,
    /// Pipeline for rendering reflected surface meshes.
    pub(crate) reflected_mesh_pipeline: Option<wgpu::RenderPipeline>,
    /// Bind group layout for reflected mesh (includes reflection uniforms).
    pub(crate) reflected_mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Pipeline for rendering reflected point clouds.
    pub(crate) reflected_point_cloud_pipeline: Option<wgpu::RenderPipeline>,
    /// Bind group layout for reflected point cloud.
    pub(crate) reflected_point_cloud_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Pipeline for rendering reflected curve networks.
    pub(crate) reflected_curve_network_pipeline: Option<wgpu::RenderPipeline>,
    /// Bind group layout for reflected curve network.
    pub(crate) reflected_curve_network_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Simple mesh pipeline (for isosurface rendering).
    pub simple_mesh_pipeline: Option<wgpu::RenderPipeline>,
    /// Simple mesh bind group layout.
    pub(crate) simple_mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Gridcube pipeline (for volume grid scalar visualization).
    pub gridcube_pipeline: Option<wgpu::RenderPipeline>,
    /// Gridcube bind group layout.
    pub(crate) gridcube_bind_group_layout: Option<wgpu::BindGroupLayout>,

    // Pick system - range-based ID management (flat 24-bit global index)
    /// Map from (`type_name`, name) to pick range.
    pub(crate) pick_ranges: HashMap<(String, String), pick::PickRange>,
    /// Next available global index (0 is reserved for background).
    pub(crate) next_global_index: u32,

    // Pick system - GPU resources
    /// Pick color texture for element selection.
    pub(crate) pick_texture: Option<wgpu::Texture>,
    /// Pick color texture view.
    pub(crate) pick_texture_view: Option<wgpu::TextureView>,
    /// Pick depth texture.
    pub(crate) pick_depth_texture: Option<wgpu::Texture>,
    /// Pick depth texture view.
    pub(crate) pick_depth_view: Option<wgpu::TextureView>,
    /// Staging buffer for pick pixel readback.
    pub(crate) pick_staging_buffer: Option<wgpu::Buffer>,
    /// Current size of pick buffers (for resize detection).
    pub(crate) pick_buffer_size: (u32, u32),
    /// Pick pipeline for point clouds.
    pub(crate) point_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Pick pipeline for curve networks (line mode).
    pub(crate) curve_network_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Pick pipeline for curve networks (tube mode) - uses ray-cylinder intersection.
    pub(crate) curve_network_tube_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Tube pick bind group layout.
    pub(crate) curve_network_tube_pick_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Pick bind group layout (shared across point cloud and curve network pick pipelines).
    pub(crate) pick_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Pick pipeline for surface meshes (face picking).
    pub(crate) mesh_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Mesh pick bind group layout (has extra `face_indices` binding).
    pub(crate) mesh_pick_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Pick pipeline for volume grid gridcube instances.
    pub(crate) gridcube_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Gridcube pick bind group layout (camera, pick uniforms, positions).
    pub(crate) gridcube_pick_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl RenderEngine {
    /// Creates a new windowed render engine.
    pub async fn new_windowed(window: Arc<winit::window::Window>) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RenderError::AdapterCreationFailed)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("polyscope device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let (depth_texture, depth_view, depth_only_view) =
            Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniforms"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create slice plane buffer and bind group
        let slice_planes_data = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
        let slice_plane_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slice Plane Buffer"),
            contents: bytemuck::cast_slice(&slice_planes_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let slice_plane_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Slice Plane Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(128),
                    },
                    count: None,
                }],
            });

        let slice_plane_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Plane Bind Group"),
            layout: &slice_plane_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: slice_plane_buffer.as_entire_binding(),
            }],
        });

        // Create shadow map pass first (needed for bind group)
        let shadow_map_pass = ShadowMapPass::new(&device);

        // Ground plane bind group layout (includes shadow bindings)
        let ground_plane_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ground Plane Bind Group Layout"),
                entries: &[
                    // Camera uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(272),
                        },
                        count: None,
                    },
                    // Ground uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(96),
                        },
                        count: None,
                    },
                    // Light uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(80),
                        },
                        count: None,
                    },
                    // Shadow map texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Shadow comparison sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Ground plane shader
        let ground_plane_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ground Plane Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ground_plane.wgsl").into()),
        });

        // Ground plane pipeline layout
        let ground_plane_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ground Plane Pipeline Layout"),
                bind_group_layouts: &[&ground_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Ground plane render pipeline (with alpha blending)
        let ground_plane_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Ground Plane Pipeline"),
                layout: Some(&ground_plane_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &ground_plane_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ground_plane_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Slice plane visualization pipeline
        let slice_plane_vis_bind_group_layout =
            crate::slice_plane_render::create_slice_plane_bind_group_layout(&device);
        let slice_plane_vis_pipeline = crate::slice_plane_render::create_slice_plane_pipeline(
            &device,
            &slice_plane_vis_bind_group_layout,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Depth24PlusStencil8,
        );

        // Create matcap bind group layout and load all matcap textures
        let matcap_bind_group_layout = materials::create_matcap_bind_group_layout(&device);
        let matcap_textures =
            materials::init_matcap_textures(&device, &queue, &matcap_bind_group_layout);

        let mut engine = Self {
            instance,
            adapter,
            device,
            queue,
            surface: Some(surface),
            surface_config,
            depth_texture,
            depth_view,
            depth_only_view,
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            matcap_bind_group_layout,
            matcap_textures,
            camera,
            width,
            height,
            point_pipeline: None,
            point_bind_group_layout: None,
            camera_buffer,
            slice_plane_buffer,
            slice_plane_bind_group_layout,
            slice_plane_bind_group,
            vector_pipeline: None,
            vector_bind_group_layout: None,
            mesh_pipeline: None,

            mesh_depth_normal_pipeline: None,
            mesh_bind_group_layout: None,
            curve_network_edge_pipeline: None,
            curve_network_edge_bind_group_layout: None,
            curve_network_tube_pipeline: None,
            curve_network_tube_compute_pipeline: None,
            curve_network_tube_bind_group_layout: None,
            curve_network_tube_compute_bind_group_layout: None,
            ground_plane_pipeline,
            ground_plane_bind_group_layout,
            ground_plane_render_data: None,
            slice_plane_vis_pipeline,
            slice_plane_vis_bind_group_layout,
            slice_plane_render_data: Vec::new(),
            screenshot_texture: None,
            screenshot_buffer: None,
            screenshot_hdr_texture: None,
            screenshot_hdr_view: None,
            hdr_texture: None,
            hdr_view: None,
            normal_texture: None,
            normal_view: None,
            ssao_noise_texture: None,
            ssao_noise_view: None,
            ssao_pass: None,
            ssao_output_texture: None,
            ssao_output_view: None,
            depth_peel_pass: None,
            tone_map_pass: None,
            ssaa_pass: None,
            ssaa_factor: 1,
            ssaa_intermediate_texture: None,
            ssaa_intermediate_view: None,
            shadow_map_pass: Some(shadow_map_pass),
            shadow_pipeline: None,
            shadow_bind_group_layout: None,
            reflection_pass: None,
            ground_stencil_pipeline: None,
            reflected_mesh_pipeline: None,
            reflected_mesh_bind_group_layout: None,
            reflected_point_cloud_pipeline: None,
            reflected_point_cloud_bind_group_layout: None,
            reflected_curve_network_pipeline: None,
            reflected_curve_network_bind_group_layout: None,
            simple_mesh_pipeline: None,
            simple_mesh_bind_group_layout: None,
            gridcube_pipeline: None,
            gridcube_bind_group_layout: None,
            pick_ranges: HashMap::new(),
            next_global_index: 1, // 0 is reserved for background
            pick_texture: None,
            pick_texture_view: None,
            pick_depth_texture: None,
            pick_depth_view: None,
            pick_staging_buffer: None,
            pick_buffer_size: (0, 0),
            point_pick_pipeline: None,
            curve_network_pick_pipeline: None,
            curve_network_tube_pick_pipeline: None,
            curve_network_tube_pick_bind_group_layout: None,
            pick_bind_group_layout: None,
            mesh_pick_pipeline: None,
            mesh_pick_bind_group_layout: None,
            gridcube_pick_pipeline: None,
            gridcube_pick_bind_group_layout: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();
        engine.create_curve_network_tube_pipelines();
        engine.create_simple_mesh_pipeline();
        engine.create_gridcube_pipeline();
        engine.create_shadow_pipeline();
        engine.init_tone_mapping();
        engine.init_ssaa_pass();
        engine.init_reflection_pass();
        engine.create_ground_stencil_pipeline();
        engine.create_reflected_mesh_pipeline();
        engine.create_reflected_point_cloud_pipeline();
        engine.create_reflected_curve_network_pipeline();
        engine.init_pick_pipeline();
        engine.init_mesh_pick_pipeline();

        Ok(engine)
    }

    /// Creates a new headless render engine.
    pub async fn new_headless(width: u32, height: u32) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RenderError::AdapterCreationFailed)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("polyscope device (headless)"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let (depth_texture, depth_view, depth_only_view) =
            Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniforms"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create slice plane buffer and bind group
        let slice_planes_data = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
        let slice_plane_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slice Plane Buffer"),
            contents: bytemuck::cast_slice(&slice_planes_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let slice_plane_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Slice Plane Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(128),
                    },
                    count: None,
                }],
            });

        let slice_plane_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Plane Bind Group"),
            layout: &slice_plane_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: slice_plane_buffer.as_entire_binding(),
            }],
        });

        // Create shadow map pass first (needed for bind group)
        let shadow_map_pass = ShadowMapPass::new(&device);

        // Ground plane bind group layout (includes shadow bindings)
        let ground_plane_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ground Plane Bind Group Layout"),
                entries: &[
                    // Camera uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(272),
                        },
                        count: None,
                    },
                    // Ground uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(96),
                        },
                        count: None,
                    },
                    // Light uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(80),
                        },
                        count: None,
                    },
                    // Shadow map texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Shadow comparison sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Ground plane shader
        let ground_plane_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ground Plane Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ground_plane.wgsl").into()),
        });

        // Ground plane pipeline layout
        let ground_plane_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ground Plane Pipeline Layout"),
                bind_group_layouts: &[&ground_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Ground plane render pipeline (with alpha blending)
        let ground_plane_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Ground Plane Pipeline"),
                layout: Some(&ground_plane_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &ground_plane_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ground_plane_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Slice plane visualization pipeline
        let slice_plane_vis_bind_group_layout =
            crate::slice_plane_render::create_slice_plane_bind_group_layout(&device);
        let slice_plane_vis_pipeline = crate::slice_plane_render::create_slice_plane_pipeline(
            &device,
            &slice_plane_vis_bind_group_layout,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Depth24PlusStencil8,
        );

        // Create matcap bind group layout and load all matcap textures
        let matcap_bind_group_layout = materials::create_matcap_bind_group_layout(&device);
        let matcap_textures =
            materials::init_matcap_textures(&device, &queue, &matcap_bind_group_layout);

        let mut engine = Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            surface_config,
            depth_texture,
            depth_view,
            depth_only_view,
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            matcap_bind_group_layout,
            matcap_textures,
            camera,
            width,
            height,
            point_pipeline: None,
            point_bind_group_layout: None,
            camera_buffer,
            slice_plane_buffer,
            slice_plane_bind_group_layout,
            slice_plane_bind_group,
            vector_pipeline: None,
            vector_bind_group_layout: None,
            mesh_pipeline: None,

            mesh_depth_normal_pipeline: None,
            mesh_bind_group_layout: None,
            curve_network_edge_pipeline: None,
            curve_network_edge_bind_group_layout: None,
            curve_network_tube_pipeline: None,
            curve_network_tube_compute_pipeline: None,
            curve_network_tube_bind_group_layout: None,
            curve_network_tube_compute_bind_group_layout: None,
            ground_plane_pipeline,
            ground_plane_bind_group_layout,
            ground_plane_render_data: None,
            slice_plane_vis_pipeline,
            slice_plane_vis_bind_group_layout,
            slice_plane_render_data: Vec::new(),
            screenshot_texture: None,
            screenshot_buffer: None,
            screenshot_hdr_texture: None,
            screenshot_hdr_view: None,
            hdr_texture: None,
            hdr_view: None,
            normal_texture: None,
            normal_view: None,
            ssao_noise_texture: None,
            ssao_noise_view: None,
            ssao_pass: None,
            ssao_output_texture: None,
            ssao_output_view: None,
            depth_peel_pass: None,
            tone_map_pass: None,
            ssaa_pass: None,
            ssaa_factor: 1,
            ssaa_intermediate_texture: None,
            ssaa_intermediate_view: None,
            shadow_map_pass: Some(shadow_map_pass),
            shadow_pipeline: None,
            shadow_bind_group_layout: None,
            reflection_pass: None,
            ground_stencil_pipeline: None,
            reflected_mesh_pipeline: None,
            reflected_mesh_bind_group_layout: None,
            reflected_point_cloud_pipeline: None,
            reflected_point_cloud_bind_group_layout: None,
            reflected_curve_network_pipeline: None,
            reflected_curve_network_bind_group_layout: None,
            simple_mesh_pipeline: None,
            simple_mesh_bind_group_layout: None,
            gridcube_pipeline: None,
            gridcube_bind_group_layout: None,
            pick_ranges: HashMap::new(),
            next_global_index: 1, // 0 is reserved for background
            pick_texture: None,
            pick_texture_view: None,
            pick_depth_texture: None,
            pick_depth_view: None,
            pick_staging_buffer: None,
            pick_buffer_size: (0, 0),
            point_pick_pipeline: None,
            curve_network_pick_pipeline: None,
            curve_network_tube_pick_pipeline: None,
            curve_network_tube_pick_bind_group_layout: None,
            pick_bind_group_layout: None,
            mesh_pick_pipeline: None,
            mesh_pick_bind_group_layout: None,
            gridcube_pick_pipeline: None,
            gridcube_pick_bind_group_layout: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();
        engine.create_curve_network_tube_pipelines();
        engine.create_simple_mesh_pipeline();
        engine.create_gridcube_pipeline();
        engine.create_shadow_pipeline();
        engine.init_tone_mapping();
        engine.init_ssaa_pass();
        engine.init_reflection_pass();
        engine.create_ground_stencil_pipeline();
        engine.create_reflected_mesh_pipeline();
        engine.create_reflected_point_cloud_pipeline();
        engine.create_reflected_curve_network_pipeline();
        engine.init_pick_pipeline();
        engine.init_mesh_pick_pipeline();

        Ok(engine)
    }

    /// Resizes the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;

        if let Some(ref surface) = self.surface {
            surface.configure(&self.device, &self.surface_config);
        }

        // Calculate SSAA-scaled dimensions
        let ssaa_width = width * self.ssaa_factor;
        let ssaa_height = height * self.ssaa_factor;

        let (depth_texture, depth_view, depth_only_view) =
            Self::create_depth_texture(&self.device, ssaa_width, ssaa_height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.depth_only_view = depth_only_view;

        // Recreate HDR texture for tone mapping (at SSAA resolution)
        self.create_hdr_texture_with_size(ssaa_width, ssaa_height);

        // Recreate normal G-buffer for SSAO (at SSAA resolution)
        self.create_normal_texture_with_size(ssaa_width, ssaa_height);

        // Resize SSAO pass and output texture (at SSAA resolution)
        if let Some(ref mut ssao_pass) = self.ssao_pass {
            ssao_pass.resize(&self.device, &self.queue, ssaa_width, ssaa_height);
        }
        self.create_ssao_output_texture_with_size(ssaa_width, ssaa_height);

        // Recreate intermediate texture for SSAA downsampling
        if self.ssaa_factor > 1 {
            self.create_ssaa_intermediate_texture();
        }

        self.camera.set_aspect_ratio(width as f32 / height as f32);
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create depth-only view for SSAO (excludes stencil aspect)
        let depth_only_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth only view"),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });

        (texture, view, depth_only_view)
    }

    /// Updates camera uniforms.
    pub fn update_camera_uniforms(&self) {
        let view = self.camera.view_matrix();
        let proj = self.camera.projection_matrix();
        let view_proj = proj * view;
        let inv_proj = proj.inverse();

        let uniforms = CameraUniforms {
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            view_proj: view_proj.to_cols_array_2d(),
            inv_proj: inv_proj.to_cols_array_2d(),
            camera_pos: self.camera.position.to_array(),
            _padding: 0.0,
        };

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Updates slice plane uniforms from the provided slice plane data.
    ///
    /// Takes an iterator of `SlicePlaneUniforms` and uploads them to the GPU buffer.
    /// Up to `MAX_SLICE_PLANES` planes are used; remaining slots are disabled.
    pub fn update_slice_plane_uniforms(&self, planes: impl Iterator<Item = SlicePlaneUniforms>) {
        let mut uniforms = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
        for (i, plane) in planes.take(MAX_SLICE_PLANES).enumerate() {
            uniforms[i] = plane;
        }

        self.queue
            .write_buffer(&self.slice_plane_buffer, 0, bytemuck::cast_slice(&uniforms));
    }

    /// Gets the camera buffer.
    pub fn camera_buffer(&self) -> &wgpu::Buffer {
        &self.camera_buffer
    }

    /// Gets the shadow map pass (if initialized).
    pub fn shadow_map_pass(&self) -> Option<&ShadowMapPass> {
        self.shadow_map_pass.as_ref()
    }

    /// Returns the depth texture view.
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Returns the HDR texture view if available.
    pub fn hdr_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.hdr_view.as_ref()
    }

    /// Returns the viewport dimensions.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the render dimensions (accounting for SSAA).
    #[must_use]
    pub fn render_dimensions(&self) -> (u32, u32) {
        (
            self.width * self.ssaa_factor,
            self.height * self.ssaa_factor,
        )
    }
}
