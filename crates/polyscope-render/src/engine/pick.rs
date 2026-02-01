use std::num::NonZeroU64;

use super::RenderEngine;

/// A contiguous range of global pick indices assigned to a structure.
#[derive(Debug, Clone)]
pub struct PickRange {
    /// The starting global index for this structure's elements.
    pub global_start: u32,
    /// Number of elements in this range.
    pub count: u32,
    /// Structure type name (e.g., `PointCloud`, `SurfaceMesh`).
    pub type_name: String,
    /// Structure name.
    pub name: String,
}

impl RenderEngine {
    // ========== Pick System - Range-Based ID Management ==========

    /// Assigns a contiguous range of global pick indices to a structure.
    ///
    /// Returns the `global_start` index. The structure owns indices
    /// `[global_start, global_start + num_elements)`.
    ///
    /// Index 0 is reserved as background (no hit), so all ranges start from >= 1.
    pub fn assign_pick_range(&mut self, type_name: &str, name: &str, num_elements: u32) -> u32 {
        let key = (type_name.to_string(), name.to_string());

        // If already assigned, return existing start
        if let Some(range) = self.pick_ranges.get(&key) {
            return range.global_start;
        }

        let global_start = self.next_global_index;
        self.next_global_index += num_elements;

        let range = PickRange {
            global_start,
            count: num_elements,
            type_name: type_name.to_string(),
            name: name.to_string(),
        };

        self.pick_ranges.insert(key, range);

        global_start
    }

    /// Removes a structure's pick range.
    ///
    /// The range is freed but the global index counter is not decremented
    /// (monotonic allocation — no fragmentation complexity).
    pub fn remove_pick_range(&mut self, type_name: &str, name: &str) {
        let key = (type_name.to_string(), name.to_string());
        self.pick_ranges.remove(&key);
    }

    /// Looks up which structure owns a global pick index.
    ///
    /// Returns `(type_name, name, local_element_index)` or None if no structure
    /// owns this index (background or freed range).
    pub fn lookup_global_index(&self, global_index: u32) -> Option<(&str, &str, u32)> {
        for range in self.pick_ranges.values() {
            if global_index >= range.global_start && global_index < range.global_start + range.count
            {
                let local = global_index - range.global_start;
                return Some((&range.type_name, &range.name, local));
            }
        }
        None
    }

    /// Gets the `global_start` for a structure's pick range, if assigned.
    pub fn get_pick_range_start(&self, type_name: &str, name: &str) -> Option<u32> {
        let key = (type_name.to_string(), name.to_string());
        self.pick_ranges.get(&key).map(|r| r.global_start)
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
    pub(crate) fn init_pick_pipeline(&mut self) {
        let shader_source = include_str!("../shaders/pick.wgsl");
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
                                min_binding_size: NonZeroU64::new(272),
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
                                min_binding_size: NonZeroU64::new(16),
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
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None, // No blending for pick buffer
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
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

    /// Gets the curve network pick pipeline.
    pub fn curve_network_pick_pipeline(&self) -> &wgpu::RenderPipeline {
        self.curve_network_pick_pipeline
            .as_ref()
            .expect("curve network pick pipeline not initialized")
    }

    /// Initializes the curve network pick pipeline.
    pub fn init_curve_network_pick_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CurveNetwork Pick Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/pick_curve.wgsl").into()),
            });

        // Reuse the pick bind group layout from point cloud pick
        let bind_group_layout = self
            .pick_bind_group_layout
            .as_ref()
            .expect("pick bind group layout not initialized");

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CurveNetwork Pick Pipeline Layout"),
                bind_group_layouts: &[bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CurveNetwork Pick Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None, // No blending for pick buffer
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..wgpu::PrimitiveState::default()
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

        self.curve_network_pick_pipeline = Some(pipeline);
    }

    /// Returns whether the curve network pick pipeline is initialized.
    pub fn has_curve_network_pick_pipeline(&self) -> bool {
        self.curve_network_pick_pipeline.is_some()
    }

    /// Initializes the curve network tube pick pipeline (uses ray-cylinder intersection).
    pub fn init_curve_network_tube_pick_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("CurveNetwork Tube Pick Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../shaders/pick_curve_tube.wgsl").into(),
                ),
            });

        // Create bind group layout for tube picking
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CurveNetwork Tube Pick Bind Group Layout"),
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
                        // Pick uniforms (global_start, radius, min_pick_radius)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(16),
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
                    ],
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CurveNetwork Tube Pick Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("CurveNetwork Tube Pick Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[
                        // Generated vertex buffer layout (same as tube render)
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
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None, // No blending for pick buffer
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
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

        self.curve_network_tube_pick_pipeline = Some(pipeline);
        self.curve_network_tube_pick_bind_group_layout = Some(bind_group_layout);
    }

    /// Returns whether the curve network tube pick pipeline is initialized.
    pub fn has_curve_network_tube_pick_pipeline(&self) -> bool {
        self.curve_network_tube_pick_pipeline.is_some()
    }

    /// Gets the curve network tube pick pipeline.
    pub fn curve_network_tube_pick_pipeline(&self) -> &wgpu::RenderPipeline {
        self.curve_network_tube_pick_pipeline
            .as_ref()
            .expect("curve network tube pick pipeline not initialized")
    }

    /// Gets the curve network tube pick bind group layout.
    pub fn curve_network_tube_pick_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.curve_network_tube_pick_bind_group_layout
            .as_ref()
            .expect("curve network tube pick bind group layout not initialized")
    }

    /// Initializes the mesh pick pipeline for surface meshes.
    pub fn init_mesh_pick_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Mesh Pick Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/pick_mesh.wgsl").into()),
            });

        // Mesh pick bind group layout: camera, mesh pick uniforms, positions, face_indices
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mesh Pick Bind Group Layout"),
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
                        // Mesh pick uniforms (global_start + model matrix = 80 bytes)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(80),
                            },
                            count: None,
                        },
                        // Position storage buffer (expanded per-triangle-vertex)
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
                        // Face index mapping buffer (tri_index -> face_index)
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                label: Some("Mesh Pick Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("SurfaceMesh Pick Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
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

        self.mesh_pick_pipeline = Some(pipeline);
        self.mesh_pick_bind_group_layout = Some(bind_group_layout);
    }

    /// Returns whether the mesh pick pipeline is initialized.
    pub fn has_mesh_pick_pipeline(&self) -> bool {
        self.mesh_pick_pipeline.is_some()
    }

    /// Gets the mesh pick pipeline.
    pub fn mesh_pick_pipeline(&self) -> &wgpu::RenderPipeline {
        self.mesh_pick_pipeline
            .as_ref()
            .expect("mesh pick pipeline not initialized")
    }

    /// Gets the mesh pick bind group layout.
    pub fn mesh_pick_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.mesh_pick_bind_group_layout
            .as_ref()
            .expect("mesh pick bind group layout not initialized")
    }

    /// Initializes the gridcube pick pipeline for volume grid instances.
    pub fn init_gridcube_pick_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Gridcube Pick Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../shaders/pick_gridcube.wgsl").into(),
                ),
            });

        // Gridcube pick bind group layout: camera, GridcubePickUniforms, positions
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Gridcube Pick Bind Group Layout"),
                    entries: &[
                        // Camera uniforms (binding 0)
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
                        // GridcubePickUniforms (model + global_start + cube_size_factor = 80 bytes) (binding 1)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(80),
                            },
                            count: None,
                        },
                        // Cube positions storage buffer (template + per-instance) (binding 2)
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
                label: Some("Gridcube Pick Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Gridcube Pick Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..wgpu::PrimitiveState::default()
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

        self.gridcube_pick_pipeline = Some(pipeline);
        self.gridcube_pick_bind_group_layout = Some(bind_group_layout);
    }

    /// Returns whether the gridcube pick pipeline is initialized.
    pub fn has_gridcube_pick_pipeline(&self) -> bool {
        self.gridcube_pick_pipeline.is_some()
    }

    /// Gets the gridcube pick pipeline.
    pub fn gridcube_pick_pipeline(&self) -> &wgpu::RenderPipeline {
        self.gridcube_pick_pipeline
            .as_ref()
            .expect("gridcube pick pipeline not initialized")
    }

    /// Gets the gridcube pick bind group layout.
    pub fn gridcube_pick_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.gridcube_pick_bind_group_layout
            .as_ref()
            .expect("gridcube pick bind group layout not initialized")
    }

    /// Reads the pick buffer at (x, y) and returns the decoded global index.
    ///
    /// Returns None if picking system not initialized or coordinates out of bounds.
    /// Returns Some(0) for background clicks.
    pub fn pick_at(&self, x: u32, y: u32) -> Option<u32> {
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

        // Decode flat 24-bit global index from RGB
        let global_index = crate::pick::color_to_index(pixel[0], pixel[1], pixel[2]);
        Some(global_index)
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // Background = (0,0,0) = index 0
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
