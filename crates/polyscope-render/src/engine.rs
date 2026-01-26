//! The main rendering engine.

use std::collections::HashMap;
use std::sync::Arc;

use wgpu::util::DeviceExt;

use polyscope_core::slice_plane::{SlicePlaneUniforms, MAX_SLICE_PLANES};

use crate::camera::Camera;
use crate::color_maps::ColorMapRegistry;
use crate::error::{RenderError, RenderResult};
use crate::ground_plane::GroundPlaneRenderData;
use crate::materials::MaterialRegistry;
use crate::shadow_map::ShadowMapPass;
use crate::tone_mapping::ToneMapPass;

/// Camera uniforms for GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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
    depth_only_view: wgpu::TextureView,
    /// Material registry.
    pub materials: MaterialRegistry,
    /// Color map registry.
    pub color_maps: ColorMapRegistry,
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
    /// Surface mesh render pipeline.
    pub mesh_pipeline: Option<wgpu::RenderPipeline>,
    /// Mesh bind group layout.
    mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network edge render pipeline (line rendering).
    pub curve_network_edge_pipeline: Option<wgpu::RenderPipeline>,
    /// Curve network edge bind group layout.
    curve_network_edge_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network tube render pipeline (cylinder impostor rendering).
    pub curve_network_tube_pipeline: Option<wgpu::RenderPipeline>,
    /// Curve network tube compute pipeline (generates bounding box geometry).
    pub curve_network_tube_compute_pipeline: Option<wgpu::ComputePipeline>,
    /// Curve network tube render bind group layout.
    curve_network_tube_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Curve network tube compute bind group layout.
    curve_network_tube_compute_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Ground plane render pipeline.
    ground_plane_pipeline: wgpu::RenderPipeline,
    /// Ground plane bind group layout.
    ground_plane_bind_group_layout: wgpu::BindGroupLayout,
    /// Ground plane render data (lazily initialized).
    ground_plane_render_data: Option<GroundPlaneRenderData>,
    /// Screenshot capture texture (lazily initialized).
    screenshot_texture: Option<wgpu::Texture>,
    /// Screenshot capture buffer (lazily initialized).
    screenshot_buffer: Option<wgpu::Buffer>,
    /// Screenshot HDR texture for rendering (lazily initialized).
    screenshot_hdr_texture: Option<wgpu::Texture>,
    /// Screenshot HDR texture view.
    screenshot_hdr_view: Option<wgpu::TextureView>,
    /// HDR intermediate texture for tone mapping.
    hdr_texture: Option<wgpu::Texture>,
    /// HDR texture view.
    hdr_view: Option<wgpu::TextureView>,
    /// Normal G-buffer texture for SSAO.
    normal_texture: Option<wgpu::Texture>,
    /// Normal G-buffer texture view.
    normal_view: Option<wgpu::TextureView>,
    /// SSAO noise texture (4x4 random rotation vectors).
    ssao_noise_texture: Option<wgpu::Texture>,
    /// SSAO noise texture view.
    ssao_noise_view: Option<wgpu::TextureView>,
    /// SSAO pass.
    ssao_pass: Option<crate::ssao_pass::SsaoPass>,
    /// SSAO output texture (blurred result).
    ssao_output_texture: Option<wgpu::Texture>,
    /// SSAO output texture view.
    ssao_output_view: Option<wgpu::TextureView>,
    /// Tone mapping post-processing pass.
    tone_map_pass: Option<ToneMapPass>,
    /// Shadow map pass for ground plane shadows.
    shadow_map_pass: Option<ShadowMapPass>,
    /// Shadow render pipeline (depth-only, renders objects from light's perspective).
    shadow_pipeline: Option<wgpu::RenderPipeline>,
    /// Shadow bind group layout for shadow pass rendering.
    shadow_bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Reflection pass for ground plane reflections.
    reflection_pass: Option<crate::reflection_pass::ReflectionPass>,
    /// Stencil pipeline for ground plane reflection mask.
    ground_stencil_pipeline: Option<wgpu::RenderPipeline>,
    /// Pipeline for rendering reflected surface meshes.
    reflected_mesh_pipeline: Option<wgpu::RenderPipeline>,
    /// Bind group layout for reflected mesh (includes reflection uniforms).
    reflected_mesh_bind_group_layout: Option<wgpu::BindGroupLayout>,

    // Pick system - structure ID management
    /// Map from (type_name, name) to structure pick ID.
    structure_id_map: HashMap<(String, String), u16>,
    /// Reverse map from structure pick ID to (type_name, name).
    structure_id_reverse: HashMap<u16, (String, String)>,
    /// Next available structure ID (0 is reserved for background).
    next_structure_id: u16,

    // Pick system - GPU resources
    /// Pick color texture for element selection.
    pick_texture: Option<wgpu::Texture>,
    /// Pick color texture view.
    pick_texture_view: Option<wgpu::TextureView>,
    /// Pick depth texture.
    pick_depth_texture: Option<wgpu::Texture>,
    /// Pick depth texture view.
    pick_depth_view: Option<wgpu::TextureView>,
    /// Staging buffer for pick pixel readback.
    pick_staging_buffer: Option<wgpu::Buffer>,
    /// Current size of pick buffers (for resize detection).
    pick_buffer_size: (u32, u32),
    /// Pick pipeline for point clouds.
    point_pick_pipeline: Option<wgpu::RenderPipeline>,
    /// Pick bind group layout (shared across pick pipelines).
    pick_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl RenderEngine {
    /// Creates a new windowed render engine.
    pub async fn new_windowed(window: Arc<winit::window::Window>) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
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
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
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
                        min_binding_size: None,
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
                            min_binding_size: None,
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
                            min_binding_size: None,
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
                            min_binding_size: None,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ground_plane.wgsl").into()),
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
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ground_plane_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
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
            tone_map_pass: None,
            shadow_map_pass: Some(shadow_map_pass),
            shadow_pipeline: None,
            shadow_bind_group_layout: None,
            reflection_pass: None,
            ground_stencil_pipeline: None,
            reflected_mesh_pipeline: None,
            reflected_mesh_bind_group_layout: None,
            structure_id_map: HashMap::new(),
            structure_id_reverse: HashMap::new(),
            next_structure_id: 1, // 0 is reserved for background
            pick_texture: None,
            pick_texture_view: None,
            pick_depth_texture: None,
            pick_depth_view: None,
            pick_staging_buffer: None,
            pick_buffer_size: (0, 0),
            point_pick_pipeline: None,
            pick_bind_group_layout: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();
        engine.create_curve_network_tube_pipelines();
        engine.create_shadow_pipeline();
        engine.init_tone_mapping();
        engine.init_reflection_pass();
        engine.create_ground_stencil_pipeline();
        engine.create_reflected_mesh_pipeline();
        engine.init_pick_pipeline();

        Ok(engine)
    }

    /// Creates a new headless render engine.
    pub async fn new_headless(width: u32, height: u32) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
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
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
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
                        min_binding_size: None,
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
                            min_binding_size: None,
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
                            min_binding_size: None,
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
                            min_binding_size: None,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ground_plane.wgsl").into()),
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
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ground_plane_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
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
            tone_map_pass: None,
            shadow_map_pass: Some(shadow_map_pass),
            shadow_pipeline: None,
            shadow_bind_group_layout: None,
            reflection_pass: None,
            ground_stencil_pipeline: None,
            reflected_mesh_pipeline: None,
            reflected_mesh_bind_group_layout: None,
            structure_id_map: HashMap::new(),
            structure_id_reverse: HashMap::new(),
            next_structure_id: 1, // 0 is reserved for background
            pick_texture: None,
            pick_texture_view: None,
            pick_depth_texture: None,
            pick_depth_view: None,
            pick_staging_buffer: None,
            pick_buffer_size: (0, 0),
            point_pick_pipeline: None,
            pick_bind_group_layout: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();
        engine.create_curve_network_tube_pipelines();
        engine.create_shadow_pipeline();
        engine.init_tone_mapping();
        engine.init_reflection_pass();
        engine.create_ground_stencil_pipeline();
        engine.create_reflected_mesh_pipeline();
        engine.init_pick_pipeline();

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

        let (depth_texture, depth_view, depth_only_view) =
            Self::create_depth_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.depth_only_view = depth_only_view;

        // Recreate HDR texture for tone mapping
        self.create_hdr_texture();

        // Recreate normal G-buffer for SSAO
        self.create_normal_texture();

        // Resize SSAO pass and output texture
        if let Some(ref mut ssao_pass) = self.ssao_pass {
            ssao_pass.resize(&self.device, &self.queue, width, height);
        }
        self.create_ssao_output_texture();

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

    /// Initializes the point cloud render pipeline.
    pub fn init_point_pipeline(&mut self) {
        let shader_source = include_str!("shaders/point_sphere.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("point sphere shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("point cloud bind group layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Point uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Position storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Color storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("point pipeline layout"),
                bind_group_layouts: &[&bind_group_layout, &self.slice_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("point sphere pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Don't cull billboards
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.point_pipeline = Some(pipeline);
        self.point_bind_group_layout = Some(bind_group_layout);
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
    pub fn update_slice_plane_uniforms<'a>(
        &self,
        planes: impl Iterator<Item = SlicePlaneUniforms>,
    ) {
        let mut uniforms = [SlicePlaneUniforms::default(); MAX_SLICE_PLANES];
        for (i, plane) in planes.take(MAX_SLICE_PLANES).enumerate() {
            uniforms[i] = plane;
        }

        self.queue
            .write_buffer(&self.slice_plane_buffer, 0, bytemuck::cast_slice(&uniforms));
    }

    /// Gets the point cloud bind group layout.
    pub fn point_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.point_bind_group_layout
            .as_ref()
            .expect("point pipeline not initialized")
    }

    /// Gets the camera buffer.
    pub fn camera_buffer(&self) -> &wgpu::Buffer {
        &self.camera_buffer
    }

    /// Gets the shadow map pass (if initialized).
    pub fn shadow_map_pass(&self) -> Option<&ShadowMapPass> {
        self.shadow_map_pass.as_ref()
    }

    /// Initializes the vector arrow render pipeline.
    pub fn init_vector_pipeline(&mut self) {
        let shader_source = include_str!("shaders/vector_arrow.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vector arrow shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("vector bind group layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Vector uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Base positions storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Vectors storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("vector pipeline layout"),
                bind_group_layouts: &[&bind_group_layout, &self.slice_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("vector arrow pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.vector_pipeline = Some(pipeline);
        self.vector_bind_group_layout = Some(bind_group_layout);
    }

    /// Gets the vector bind group layout.
    pub fn vector_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.vector_bind_group_layout
            .as_ref()
            .expect("vector pipeline not initialized")
    }

    /// Gets the mesh bind group layout.
    pub fn mesh_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.mesh_bind_group_layout
            .as_ref()
            .expect("mesh pipeline not initialized")
    }

    /// Creates the surface mesh render pipeline.
    fn create_mesh_pipeline(&mut self) {
        let shader_source = include_str!("shaders/surface_mesh.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("surface mesh shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mesh bind group layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Mesh uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Positions storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Normals storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Barycentrics storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Colors storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge is real storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("mesh pipeline layout"),
                bind_group_layouts: &[&bind_group_layout, &self.slice_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("surface mesh pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[
                        // Color output (HDR)
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // Normal output (G-buffer for SSAO)
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Culling handled in shader
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.mesh_pipeline = Some(pipeline);
        self.mesh_bind_group_layout = Some(bind_group_layout);
    }

    /// Gets the curve network edge bind group layout.
    pub fn curve_network_edge_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.curve_network_edge_bind_group_layout
            .as_ref()
            .expect("curve network edge pipeline not initialized")
    }

    /// Creates the curve network edge render pipeline (line rendering).
    fn create_curve_network_edge_pipeline(&mut self) {
        let shader_source = include_str!("shaders/curve_network_edge.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("curve network edge shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("curve network edge bind group layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Curve network uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Node positions storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Node colors storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge vertices storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge colors storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("curve network edge pipeline layout"),
                bind_group_layouts: &[&bind_group_layout, &self.slice_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("curve network edge pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // HDR format for scene rendering
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Lines have no front/back
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.curve_network_edge_pipeline = Some(pipeline);
        self.curve_network_edge_bind_group_layout = Some(bind_group_layout);
    }

    /// Creates the curve network tube pipelines (compute and render).
    fn create_curve_network_tube_pipelines(&mut self) {
        // Compute shader
        let compute_shader_source = include_str!("shaders/curve_network_tube_compute.wgsl");
        let compute_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Curve Network Tube Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(compute_shader_source.into()),
            });

        // Compute bind group layout
        let compute_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Curve Network Tube Compute Bind Group Layout"),
                    entries: &[
                        // Edge vertices (input)
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Output vertices
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Num edges
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let compute_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Curve Network Tube Compute Pipeline Layout"),
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Curve Network Tube Compute Pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &compute_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // Render shader
        let render_shader_source = include_str!("shaders/curve_network_tube.wgsl");
        let render_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Curve Network Tube Render Shader"),
                source: wgpu::ShaderSource::Wgsl(render_shader_source.into()),
            });

        // Render bind group layout
        let render_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Curve Network Tube Render Bind Group Layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Curve network uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge vertices (for raycast)
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge colors
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Curve Network Tube Render Pipeline Layout"),
                    bind_group_layouts: &[&render_bind_group_layout, &self.slice_plane_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Curve Network Tube Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[
                        // Generated vertex buffer layout
                        wgpu::VertexBufferLayout {
                            array_stride: 32, // vec4<f32> position + vec4<u32> edge_id_and_vertex_id
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[
                                wgpu::VertexAttribute {
                                    format: wgpu::VertexFormat::Float32x4,
                                    offset: 0,
                                    shader_location: 0,
                                },
                                wgpu::VertexAttribute {
                                    format: wgpu::VertexFormat::Uint32x4,
                                    offset: 16,
                                    shader_location: 1,
                                },
                            ],
                        },
                    ],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Don't cull - we need to see box from inside too
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.curve_network_tube_pipeline = Some(render_pipeline);
        self.curve_network_tube_compute_pipeline = Some(compute_pipeline);
        self.curve_network_tube_bind_group_layout = Some(render_bind_group_layout);
        self.curve_network_tube_compute_bind_group_layout = Some(compute_bind_group_layout);
    }

    /// Gets the curve network tube render bind group layout.
    pub fn curve_network_tube_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.curve_network_tube_bind_group_layout
            .as_ref()
            .expect("Tube bind group layout not initialized")
    }

    /// Gets the curve network tube compute bind group layout.
    pub fn curve_network_tube_compute_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.curve_network_tube_compute_bind_group_layout
            .as_ref()
            .expect("Tube compute bind group layout not initialized")
    }

    /// Gets the curve network tube compute pipeline.
    pub fn curve_network_tube_compute_pipeline(&self) -> &wgpu::ComputePipeline {
        self.curve_network_tube_compute_pipeline
            .as_ref()
            .expect("Tube compute pipeline not initialized")
    }

    /// Creates the shadow render pipeline (depth-only, for rendering objects from light's perspective).
    fn create_shadow_pipeline(&mut self) {
        let shader_source = include_str!("shaders/shadow_map.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shadow Map Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Bind group layout matching shadow_map.wgsl:
        // binding 0: light uniforms (view_proj, light_dir)
        // binding 1: model uniforms (model matrix)
        // binding 2: vertex positions (storage buffer)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Shadow Bind Group Layout"),
                    entries: &[
                        // Light uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Model uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Vertex positions (storage buffer)
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Shadow Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shadow Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[], // Depth-only, no color attachments
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    // Shadow pipeline uses Depth32Float to match shadow map texture
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,     // Bias to prevent shadow acne
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(), // No MSAA for shadow map
                multiview: None,
                cache: None,
            });

        self.shadow_pipeline = Some(pipeline);
        self.shadow_bind_group_layout = Some(bind_group_layout);
    }

    /// Gets the shadow render pipeline.
    pub fn shadow_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.shadow_pipeline.as_ref()
    }

    /// Gets the shadow bind group layout.
    pub fn shadow_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.shadow_bind_group_layout.as_ref()
    }

    /// Creates the ground stencil pipeline for reflection masking.
    fn create_ground_stencil_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Ground Stencil Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("shaders/ground_stencil.wgsl").into(),
                ),
            });

        // Use existing ground plane bind group layout (camera + ground uniforms)
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ground Stencil Pipeline Layout"),
                bind_group_layouts: &[&self.ground_plane_bind_group_layout],
                push_constant_ranges: &[],
            });

        self.ground_stencil_pipeline = Some(self.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Ground Stencil Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::empty(), // No color writes
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false, // Don't write depth
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState {
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Replace, // Write stencil ref
                        },
                        back: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Replace,
                        },
                        read_mask: 0xFF,
                        write_mask: 0xFF,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));
    }

    /// Creates the reflected mesh pipeline for ground reflections.
    fn create_reflected_mesh_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Reflected Mesh Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("shaders/reflected_mesh.wgsl").into(),
                ),
            });

        // Bind group 0: camera, mesh uniforms, buffers (same as surface mesh)
        // Bind group 1: reflection uniforms
        let Some(reflection_pass) = &self.reflection_pass else {
            return;
        };

        // Create bind group layout for group 0 (mesh data)
        let mesh_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Reflected Mesh Bind Group Layout 0"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Mesh uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Positions
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Normals
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Barycentrics
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Colors
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Edge is real
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Reflected Mesh Pipeline Layout"),
                bind_group_layouts: &[&mesh_bind_group_layout, reflection_pass.bind_group_layout()],
                push_constant_ranges: &[],
            });

        self.reflected_mesh_pipeline = Some(self.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Reflected Mesh Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Front), // Cull front faces (they become back after reflection)
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false, // Don't write depth for reflections
                    depth_compare: wgpu::CompareFunction::Always, // Always pass depth test, stencil does the masking
                    stencil: wgpu::StencilState {
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Equal, // Only render where stencil == ref
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Keep,
                        },
                        back: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Equal,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Keep,
                        },
                        read_mask: 0xFF,
                        write_mask: 0x00, // Don't modify stencil
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));

        self.reflected_mesh_bind_group_layout = Some(mesh_bind_group_layout);
    }

    /// Renders the ground plane.
    ///
    /// # Arguments
    /// * `encoder` - The command encoder
    /// * `view` - The render target view
    /// * `enabled` - Whether the ground plane is enabled
    /// * `scene_center` - Center of the scene bounding box
    /// * `scene_min_y` - Minimum Y coordinate of scene bounding box
    /// * `length_scale` - Scene length scale
    /// * `height_override` - Optional manual height (None = auto below scene)
    /// * `shadow_darkness` - Shadow darkness (0.0 = no shadow, 1.0 = full black)
    /// * `shadow_mode` - Shadow mode: 0=none, 1=shadow_only, 2=tile_with_shadow
    /// * `reflection_intensity` - Reflection intensity (0.0 = opaque, affects transparency)
    #[allow(clippy::too_many_arguments)]
    pub fn render_ground_plane(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        enabled: bool,
        scene_center: [f32; 3],
        scene_min_y: f32,
        length_scale: f32,
        height_override: Option<f32>,
        shadow_darkness: f32,
        shadow_mode: u32,
        reflection_intensity: f32,
    ) {
        // Check if camera is in orthographic mode
        let is_orthographic = self.camera.projection_mode == crate::camera::ProjectionMode::Orthographic;
        if !enabled {
            return;
        }

        // Always use HDR texture for ground plane rendering (pipelines use HDR format)
        let view = self.hdr_view.as_ref().unwrap_or(surface_view);

        // Initialize render data if needed
        if self.ground_plane_render_data.is_none() {
            if let Some(ref shadow_pass) = self.shadow_map_pass {
                self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
                    &self.device,
                    &self.ground_plane_bind_group_layout,
                    &self.camera_buffer,
                    shadow_pass.light_buffer(),
                    shadow_pass.depth_view(),
                    shadow_pass.comparison_sampler(),
                ));
            }
        }

        // Get camera height
        let camera_height = self.camera.position.y;

        if let Some(render_data) = &self.ground_plane_render_data {
            render_data.update(
                &self.queue,
                scene_center,
                scene_min_y,
                length_scale,
                camera_height,
                height_override,
                shadow_darkness,
                shadow_mode,
                is_orthographic,
                reflection_intensity,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ground Plane Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.ground_plane_pipeline);
            render_pass.set_bind_group(0, render_data.bind_group(), &[]);
            // 4 triangles * 3 vertices = 12 vertices for infinite plane
            render_pass.draw(0..12, 0..1);
        }
    }

    /// Renders the ground plane to the stencil buffer for reflection masking.
    ///
    /// This should be called before rendering reflected geometry.
    /// The stencil buffer will have value 1 where the ground plane is visible.
    #[allow(clippy::too_many_arguments)]
    pub fn render_stencil_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        ground_height: f32,
        scene_center: [f32; 3],
        length_scale: f32,
    ) {
        let Some(pipeline) = &self.ground_stencil_pipeline else {
            return;
        };

        // Initialize render data if needed
        if self.ground_plane_render_data.is_none() {
            if let Some(ref shadow_pass) = self.shadow_map_pass {
                self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
                    &self.device,
                    &self.ground_plane_bind_group_layout,
                    &self.camera_buffer,
                    shadow_pass.light_buffer(),
                    shadow_pass.depth_view(),
                    shadow_pass.comparison_sampler(),
                ));
            }
        }

        let Some(render_data) = &self.ground_plane_render_data else {
            return;
        };

        // Check if camera is in orthographic mode
        let is_orthographic =
            self.camera.projection_mode == crate::camera::ProjectionMode::Orthographic;
        let camera_height = self.camera.position.y;

        // Update ground uniforms for stencil pass
        render_data.update(
            &self.queue,
            scene_center,
            scene_center[1] - length_scale * 0.5, // scene_min_y estimate
            length_scale,
            camera_height,
            Some(ground_height),
            0.0, // shadow_darkness (unused in stencil)
            0,   // shadow_mode (unused in stencil)
            is_orthographic,
            0.0, // reflection_intensity (unused in stencil)
        );

        let view = self.hdr_view.as_ref().unwrap_or(color_view);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Stencil Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Don't clear color
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep existing depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0), // Clear stencil to 0
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, render_data.bind_group(), &[]);
        render_pass.set_stencil_reference(1); // Write 1 to stencil
        render_pass.draw(0..12, 0..1); // 4 triangles = 12 vertices
    }

    /// Creates a screenshot texture for capturing frames.
    ///
    /// Returns a texture view (HDR format) that can be used as a render target.
    /// The pipelines render to HDR format, so we need an HDR texture for rendering,
    /// then tone map to the final screenshot texture.
    /// After rendering to this view, call `apply_screenshot_tone_mapping()` then
    /// `capture_screenshot()` to get the pixel data.
    pub fn create_screenshot_target(&mut self) -> wgpu::TextureView {
        // Calculate buffer size with proper alignment
        let bytes_per_row = Self::aligned_bytes_per_row(self.width);
        let buffer_size = (bytes_per_row * self.height) as u64;

        // Create HDR texture for rendering (matches pipeline format)
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot HDR texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // HDR format matching pipelines
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create final capture texture (surface format for readback)
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Create staging buffer for readback
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.screenshot_hdr_texture = Some(hdr_texture);
        self.screenshot_hdr_view = Some(hdr_view);
        self.screenshot_texture = Some(texture);
        self.screenshot_buffer = Some(buffer);

        // Return the HDR view for rendering
        self.screenshot_hdr_view.as_ref().unwrap().clone()
    }

    /// Returns the screenshot texture view (for tone mapping output).
    pub fn screenshot_texture_view(&self) -> Option<wgpu::TextureView> {
        self.screenshot_texture
            .as_ref()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
    }

    /// Applies tone mapping from the screenshot HDR texture to the final screenshot texture.
    pub fn apply_screenshot_tone_mapping(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(hdr_view) = &self.screenshot_hdr_view else {
            log::error!("Screenshot HDR view not initialized");
            return;
        };

        let Some(screenshot_texture) = &self.screenshot_texture else {
            log::error!("Screenshot texture not initialized");
            return;
        };

        let screenshot_view =
            screenshot_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Use the existing tone mapping pass
        // For screenshots, we use the main SSAO output view if available
        // (Note: SSAO effect depends on the main render resolution, not screenshot resolution)
        if let Some(tone_map_pass) = &self.tone_map_pass {
            // Use SSAO output or fall back to HDR view (which is ignored when ssao_enabled=false)
            let ssao_view = self.ssao_output_view.as_ref().unwrap_or(hdr_view);
            tone_map_pass.render_to_target(
                &self.device,
                encoder,
                hdr_view,
                ssao_view,
                &screenshot_view,
            );
        }
    }

    /// Returns the screenshot depth view for rendering.
    pub fn screenshot_depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Calculates bytes per row with proper alignment for wgpu buffer copies.
    fn aligned_bytes_per_row(width: u32) -> u32 {
        let bytes_per_pixel = 4u32; // RGBA8
        let unaligned = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        (unaligned + align - 1) / align * align
    }

    /// Captures the screenshot after rendering to the screenshot target.
    ///
    /// This method copies the screenshot texture to a buffer and reads it back.
    /// Call this after rendering to the view returned by `create_screenshot_target()`.
    ///
    /// Returns the raw RGBA pixel data.
    pub fn capture_screenshot(&mut self) -> Result<Vec<u8>, crate::screenshot::ScreenshotError> {
        let texture = self
            .screenshot_texture
            .as_ref()
            .ok_or(crate::screenshot::ScreenshotError::InvalidImageData)?;
        let buffer = self
            .screenshot_buffer
            .as_ref()
            .ok_or(crate::screenshot::ScreenshotError::InvalidImageData)?;

        let bytes_per_row = Self::aligned_bytes_per_row(self.width);

        // Create encoder and copy texture to buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot copy encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read data
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| crate::screenshot::ScreenshotError::BufferMapFailed)?
            .map_err(|_| crate::screenshot::ScreenshotError::BufferMapFailed)?;

        // Copy data, removing row padding
        let data = buffer_slice.get_mapped_range();
        let mut result = Vec::with_capacity((self.width * self.height * 4) as usize);
        let row_bytes = (self.width * 4) as usize;

        for row in 0..self.height {
            let start = (row * bytes_per_row) as usize;
            let end = start + row_bytes;
            result.extend_from_slice(&data[start..end]);
        }

        drop(data);
        buffer.unmap();

        // Clean up screenshot resources
        self.screenshot_texture = None;
        self.screenshot_buffer = None;
        self.screenshot_hdr_texture = None;
        self.screenshot_hdr_view = None;

        Ok(result)
    }

    /// Returns the current viewport dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Initializes tone mapping resources.
    fn init_tone_mapping(&mut self) {
        self.tone_map_pass = Some(ToneMapPass::new(&self.device, self.surface_config.format));
        self.create_hdr_texture();
        self.create_normal_texture();
        self.create_ssao_noise_texture();
        self.init_ssao_pass();
    }

    /// Initializes SSAO pass.
    fn init_ssao_pass(&mut self) {
        let ssao_pass = crate::ssao_pass::SsaoPass::new(&self.device, self.width, self.height);
        self.ssao_pass = Some(ssao_pass);
        self.create_ssao_output_texture();
    }

    /// Creates the SSAO output texture (blurred result).
    fn create_ssao_output_texture(&mut self) {
        let ssao_output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Output Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ssao_output_view =
            ssao_output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.ssao_output_texture = Some(ssao_output_texture);
        self.ssao_output_view = Some(ssao_output_view);
    }

    /// Creates the HDR intermediate texture for tone mapping.
    fn create_hdr_texture(&mut self) {
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // HDR format
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.hdr_texture = Some(hdr_texture);
        self.hdr_view = Some(hdr_view);
    }

    /// Creates the normal G-buffer texture for SSAO.
    fn create_normal_texture(&mut self) {
        let normal_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal G-Buffer"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // View-space normals (xyz) + unused (w)
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.normal_texture = Some(normal_texture);
        self.normal_view = Some(normal_view);
    }

    /// Creates the SSAO noise texture.
    fn create_ssao_noise_texture(&mut self) {
        use rand::Rng;

        // Generate 4x4 random rotation vectors
        let mut rng = rand::thread_rng();
        let mut noise_data = Vec::with_capacity(4 * 4 * 4); // 4x4 pixels, RGBA8

        for _ in 0..16 {
            // Random rotation vector in tangent plane (z=0)
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let x = angle.cos();
            let y = angle.sin();
            // Store in [0,1] range
            noise_data.push(((x * 0.5 + 0.5) * 255.0) as u8);
            noise_data.push(((y * 0.5 + 0.5) * 255.0) as u8);
            noise_data.push(0u8); // z = 0
            noise_data.push(255u8); // w = 1
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Noise Texture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &noise_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * 4),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.ssao_noise_texture = Some(texture);
        self.ssao_noise_view = Some(view);
    }

    /// Returns the HDR texture view for rendering the scene.
    pub fn hdr_view(&self) -> Option<&wgpu::TextureView> {
        self.hdr_view.as_ref()
    }

    /// Returns the normal G-buffer view if available.
    pub fn normal_view(&self) -> Option<&wgpu::TextureView> {
        self.normal_view.as_ref()
    }

    /// Returns the SSAO noise texture view if available.
    pub fn ssao_noise_view(&self) -> Option<&wgpu::TextureView> {
        self.ssao_noise_view.as_ref()
    }

    /// Returns the SSAO output texture view if available.
    pub fn ssao_output_view(&self) -> Option<&wgpu::TextureView> {
        self.ssao_output_view.as_ref()
    }

    /// Returns the SSAO pass.
    pub fn ssao_pass(&self) -> Option<&crate::ssao_pass::SsaoPass> {
        self.ssao_pass.as_ref()
    }

    /// Renders the SSAO pass.
    /// Returns true if SSAO was rendered, false if resources are not available.
    pub fn render_ssao(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        config: &polyscope_core::SsaoConfig,
    ) -> bool {
        // Check if all required resources are available
        // Use depth_only_view for SSAO (excludes stencil aspect)
        let (ssao_pass, depth_view, normal_view, noise_view, output_view) = match (
            &self.ssao_pass,
            Some(&self.depth_only_view),
            self.normal_view.as_ref(),
            self.ssao_noise_view.as_ref(),
            self.ssao_output_view.as_ref(),
        ) {
            (Some(pass), Some(depth), Some(normal), Some(noise), Some(output)) => {
                (pass, depth, normal, noise, output)
            }
            _ => return false,
        };

        if !config.enabled {
            return false;
        }

        // Update SSAO uniforms
        let proj = self.camera.projection_matrix();
        let inv_proj = proj.inverse();
        ssao_pass.update_uniforms(
            &self.queue,
            proj,
            inv_proj,
            config.radius,
            config.bias,
            config.intensity,
            config.sample_count,
            self.width as f32,
            self.height as f32,
        );

        // Create bind groups
        let ssao_bind_group =
            ssao_pass.create_ssao_bind_group(&self.device, depth_view, normal_view, noise_view);
        let blur_bind_group = ssao_pass.create_blur_bind_group(&self.device);

        // Render SSAO pass
        ssao_pass.render_ssao(encoder, &ssao_bind_group);

        // Render blur pass to output texture
        ssao_pass.render_blur(encoder, output_view, &blur_bind_group);

        true
    }

    /// Returns the tone map pass.
    pub fn tone_map_pass(&self) -> Option<&ToneMapPass> {
        self.tone_map_pass.as_ref()
    }

    /// Updates tone mapping uniforms.
    pub fn update_tone_mapping(
        &self,
        exposure: f32,
        white_level: f32,
        gamma: f32,
        ssao_enabled: bool,
    ) {
        if let Some(tone_map) = &self.tone_map_pass {
            tone_map.update_uniforms(&self.queue, exposure, white_level, gamma, ssao_enabled);
        }
    }

    /// Renders the tone mapping pass from HDR to the output view.
    /// Uses SSAO texture if available, otherwise uses a default white texture.
    pub fn render_tone_mapping(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        if let (Some(tone_map), Some(hdr_view)) = (&self.tone_map_pass, &self.hdr_view) {
            // Use SSAO output view if available, otherwise create a dummy white texture view
            let ssao_view = self.ssao_output_view.as_ref().unwrap_or_else(|| {
                // This should not happen in practice since we always create SSAO output
                // But as a fallback, we'll use the HDR view (which will be ignored anyway
                // when ssao_enabled is false)
                hdr_view
            });
            let bind_group = tone_map.create_bind_group(&self.device, hdr_view, ssao_view);
            tone_map.render(encoder, output_view, &bind_group);
        }
    }

    /// Initializes reflection pass resources.
    fn init_reflection_pass(&mut self) {
        self.reflection_pass = Some(crate::reflection_pass::ReflectionPass::new(&self.device));
    }

    /// Returns the reflection pass.
    pub fn reflection_pass(&self) -> Option<&crate::reflection_pass::ReflectionPass> {
        self.reflection_pass.as_ref()
    }

    /// Updates reflection uniforms.
    pub fn update_reflection(
        &self,
        reflection_matrix: glam::Mat4,
        intensity: f32,
        ground_height: f32,
    ) {
        if let Some(reflection) = &self.reflection_pass {
            reflection.update_uniforms(&self.queue, reflection_matrix, intensity, ground_height);
        }
    }

    /// Creates a bind group for reflected mesh rendering.
    pub fn create_reflected_mesh_bind_group(
        &self,
        mesh_render_data: &crate::surface_mesh_render::SurfaceMeshRenderData,
    ) -> Option<wgpu::BindGroup> {
        let layout = self.reflected_mesh_bind_group_layout.as_ref()?;

        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflected Mesh Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mesh_render_data.uniform_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mesh_render_data.position_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mesh_render_data.normal_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mesh_render_data.barycentric_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mesh_render_data.color_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: mesh_render_data.edge_is_real_buffer().as_entire_binding(),
                },
            ],
        }))
    }

    /// Renders a single reflected mesh.
    ///
    /// Call this for each visible surface mesh after render_stencil_pass.
    pub fn render_reflected_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh_bind_group: &wgpu::BindGroup,
        vertex_count: u32,
    ) {
        let Some(pipeline) = &self.reflected_mesh_pipeline else {
            return;
        };
        let Some(reflection) = &self.reflection_pass else {
            return;
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, mesh_bind_group, &[]);
        render_pass.set_bind_group(1, reflection.bind_group(), &[]);
        render_pass.set_stencil_reference(1); // Test against stencil value 1
        render_pass.draw(0..vertex_count, 0..1);
    }

    /// Returns the depth texture view.
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Returns the HDR texture view if available.
    pub fn hdr_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.hdr_view.as_ref()
    }

    // ========== Pick System - Structure ID Management ==========

    /// Assigns a unique pick ID to a structure. Returns the assigned ID.
    pub fn assign_structure_id(&mut self, type_name: &str, name: &str) -> u16 {
        let key = (type_name.to_string(), name.to_string());
        if let Some(&id) = self.structure_id_map.get(&key) {
            return id;
        }
        let id = self.next_structure_id;
        self.next_structure_id += 1;
        self.structure_id_map.insert(key.clone(), id);
        self.structure_id_reverse.insert(id, key);
        id
    }

    /// Removes a structure's pick ID.
    pub fn remove_structure_id(&mut self, type_name: &str, name: &str) {
        let key = (type_name.to_string(), name.to_string());
        if let Some(id) = self.structure_id_map.remove(&key) {
            self.structure_id_reverse.remove(&id);
        }
    }

    /// Looks up structure info from a pick ID.
    pub fn lookup_structure_id(&self, id: u16) -> Option<(&str, &str)> {
        self.structure_id_reverse
            .get(&id)
            .map(|(t, n)| (t.as_str(), n.as_str()))
    }

    /// Gets the pick ID for a structure, if assigned.
    pub fn get_structure_id(&self, type_name: &str, name: &str) -> Option<u16> {
        let key = (type_name.to_string(), name.to_string());
        self.structure_id_map.get(&key).copied()
    }

    // ========== Pick System - GPU Resources ==========

    /// Creates or recreates pick buffer textures to match viewport size.
    pub fn init_pick_buffers(&mut self, width: u32, height: u32) {
        // Skip if size unchanged
        if self.pick_buffer_size == (width, height) && self.pick_texture.is_some() {
            return;
        }

        let device = &self.device;

        // Create pick color texture (Rgba8Unorm for exact values)
        let pick_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pick Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pick_texture_view = pick_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create pick depth texture
        let pick_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pick Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let pick_depth_view =
            pick_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create staging buffer for single pixel readback (4 bytes RGBA)
        // Buffer size must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let pick_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pick Staging Buffer"),
            size: 256, // Minimum aligned size, we only read 4 bytes
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.pick_texture = Some(pick_texture);
        self.pick_texture_view = Some(pick_texture_view);
        self.pick_depth_texture = Some(pick_depth_texture);
        self.pick_depth_view = Some(pick_depth_view);
        self.pick_staging_buffer = Some(pick_staging_buffer);
        self.pick_buffer_size = (width, height);
    }

    /// Initializes the pick pipeline for point clouds.
    fn init_pick_pipeline(&mut self) {
        let shader_source = include_str!("shaders/pick.wgsl");
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pick Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Pick bind group layout: camera, pick uniforms, positions
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Pick Bind Group Layout"),
                    entries: &[
                        // Camera uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Pick uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Position storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pick Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("PointCloud Pick Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None, // No blending for pick buffer
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.point_pick_pipeline = Some(pipeline);
        self.pick_bind_group_layout = Some(bind_group_layout);
    }

    /// Gets the pick bind group layout.
    pub fn pick_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.pick_bind_group_layout
            .as_ref()
            .expect("pick pipeline not initialized")
    }

    /// Gets the point cloud pick pipeline.
    pub fn point_pick_pipeline(&self) -> &wgpu::RenderPipeline {
        self.point_pick_pipeline
            .as_ref()
            .expect("pick pipeline not initialized")
    }

    /// Reads the pick buffer at (x, y) and returns the decoded structure/element.
    ///
    /// Returns None if picking system not initialized or coordinates out of bounds.
    /// Returns Some((0, 0)) for background clicks.
    pub fn pick_at(&self, x: u32, y: u32) -> Option<(u16, u16)> {
        let pick_texture = self.pick_texture.as_ref()?;
        let staging_buffer = self.pick_staging_buffer.as_ref()?;

        // Bounds check
        let (width, height) = self.pick_buffer_size;
        if x >= width || y >= height {
            return None;
        }

        // Create encoder for copy operation
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pick Readback Encoder"),
            });

        // Copy single pixel from pick texture to staging buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: pick_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256), // Aligned
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read pixel
        let buffer_slice = staging_buffer.slice(..4);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().ok()?;

        let data = buffer_slice.get_mapped_range();
        let pixel: [u8; 4] = [data[0], data[1], data[2], data[3]];
        drop(data);
        staging_buffer.unmap();

        let (struct_id, elem_id) = crate::pick::decode_pick_id(pixel[0], pixel[1], pixel[2]);
        Some((struct_id, elem_id))
    }

    /// Returns the pick texture view for external rendering.
    pub fn pick_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.pick_texture_view.as_ref()
    }

    /// Returns the pick depth texture view for external rendering.
    pub fn pick_depth_view(&self) -> Option<&wgpu::TextureView> {
        self.pick_depth_view.as_ref()
    }

    /// Begins a pick render pass. Returns the render pass encoder.
    ///
    /// The caller is responsible for rendering structures to this pass
    /// and then dropping the encoder to finish the pass.
    pub fn begin_pick_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<wgpu::RenderPass<'a>> {
        let pick_view = self.pick_texture_view.as_ref()?;
        let pick_depth = self.pick_depth_view.as_ref()?;

        Some(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pick Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: pick_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // Background = (0,0,0)
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: pick_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }))
    }
}
