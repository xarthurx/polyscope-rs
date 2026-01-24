//! The main rendering engine.

use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::color_maps::ColorMapRegistry;
use crate::error::{RenderError, RenderResult};
use crate::ground_plane::GroundPlaneRenderData;
use crate::materials::MaterialRegistry;

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

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniforms"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Ground plane bind group layout
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
                        format: surface_config.format,
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
                    format: wgpu::TextureFormat::Depth32Float,
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
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            camera,
            width,
            height,
            point_pipeline: None,
            point_bind_group_layout: None,
            camera_buffer,
            vector_pipeline: None,
            vector_bind_group_layout: None,
            mesh_pipeline: None,
            mesh_bind_group_layout: None,
            curve_network_edge_pipeline: None,
            curve_network_edge_bind_group_layout: None,
            ground_plane_pipeline,
            ground_plane_bind_group_layout,
            ground_plane_render_data: None,
            screenshot_texture: None,
            screenshot_buffer: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();

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

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniforms"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Ground plane bind group layout
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
                        format: surface_config.format,
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
                    format: wgpu::TextureFormat::Depth32Float,
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
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            camera,
            width,
            height,
            point_pipeline: None,
            point_bind_group_layout: None,
            camera_buffer,
            vector_pipeline: None,
            vector_bind_group_layout: None,
            mesh_pipeline: None,
            mesh_bind_group_layout: None,
            curve_network_edge_pipeline: None,
            curve_network_edge_bind_group_layout: None,
            ground_plane_pipeline,
            ground_plane_bind_group_layout,
            ground_plane_render_data: None,
            screenshot_texture: None,
            screenshot_buffer: None,
        };

        engine.init_point_pipeline();
        engine.init_vector_pipeline();
        engine.create_mesh_pipeline();
        engine.create_curve_network_edge_pipeline();

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

        let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        self.camera.set_aspect_ratio(width as f32 / height as f32);
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
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
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
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
                bind_group_layouts: &[&bind_group_layout],
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
                        format: self.surface_config.format,
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
                    format: wgpu::TextureFormat::Depth32Float,
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
                bind_group_layouts: &[&bind_group_layout],
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
                        format: self.surface_config.format,
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
                    format: wgpu::TextureFormat::Depth32Float,
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
                bind_group_layouts: &[&bind_group_layout],
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
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
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
                    format: wgpu::TextureFormat::Depth32Float,
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
                bind_group_layouts: &[&bind_group_layout],
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
                        format: self.surface_config.format,
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
                    format: wgpu::TextureFormat::Depth32Float,
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
    pub fn render_ground_plane(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        enabled: bool,
        scene_center: [f32; 3],
        scene_min_y: f32,
        length_scale: f32,
        height_override: Option<f32>,
    ) {
        if !enabled {
            return;
        }

        // Initialize render data if needed
        if self.ground_plane_render_data.is_none() {
            self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
                &self.device,
                &self.ground_plane_bind_group_layout,
                &self.camera_buffer,
            ));
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

    /// Creates a screenshot texture for capturing frames.
    ///
    /// Returns a texture view that can be used as a render target.
    /// After rendering to this view, call `capture_screenshot()` to get the pixel data.
    pub fn create_screenshot_target(&mut self) -> wgpu::TextureView {
        // Calculate buffer size with proper alignment
        let bytes_per_row = Self::aligned_bytes_per_row(self.width);
        let buffer_size = (bytes_per_row * self.height) as u64;

        // Create capture texture
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

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.screenshot_texture = Some(texture);
        self.screenshot_buffer = Some(buffer);

        view
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

        Ok(result)
    }

    /// Returns the current viewport dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
