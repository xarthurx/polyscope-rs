//! Depth peeling transparency pass.
//!
//! Implements front-to-back depth peeling for correct transparency rendering.
//! Each peel pass renders the scene, discarding fragments at or in front of the
//! previous pass's maximum depth. Peeled layers are composited using alpha-under
//! blending into a final buffer.

/// Resources for the depth peeling transparency system.
pub struct DepthPeelPass {
    /// Pipeline for peeling passes (surface_mesh_peel.wgsl)
    peel_pipeline: wgpu::RenderPipeline,
    /// Pipeline for compositing each peeled layer into the final buffer (under)
    composite_under_pipeline: wgpu::RenderPipeline,
    /// Pipeline for compositing the final result onto the scene (over)
    composite_over_pipeline: wgpu::RenderPipeline,
    /// Pipeline for updating min-depth (copies peel depth into min-depth via Max blend)
    depth_update_pipeline: wgpu::RenderPipeline,

    /// Min-depth texture (Rgba16Float) — stores the maximum depth peeled so far.
    /// Uses Rgba16Float instead of R32Float because R32Float is not blendable
    /// in WebGPU without the float32-blendable feature, and we need Max blend.
    min_depth_texture: wgpu::Texture,
    min_depth_view: wgpu::TextureView,

    /// Per-peel-pass color output (Rgba16Float, premultiplied alpha)
    peel_color_texture: wgpu::Texture,
    peel_color_view: wgpu::TextureView,

    /// Per-peel-pass depth output as color (R32Float) — fragment depth written as color
    peel_depth_color_texture: wgpu::Texture,
    peel_depth_color_view: wgpu::TextureView,

    /// Actual depth buffer for peel pass (for standard depth testing within each layer)
    peel_depth_texture: wgpu::Texture,
    peel_depth_view: wgpu::TextureView,

    /// Final accumulated result (Rgba16Float)
    pub(crate) final_texture: wgpu::Texture,
    pub(crate) final_view: wgpu::TextureView,

    /// Bind group layout for peel shader's Group 3 (min-depth texture + sampler)
    pub(crate) peel_bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for peel shader (references min_depth_texture)
    peel_bind_group: wgpu::BindGroup,

    /// Bind group layout for composite shader (image + sampler)
    composite_bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group layout for depth update shader (depth color + sampler)
    depth_update_bind_group_layout: wgpu::BindGroupLayout,

    sampler: wgpu::Sampler,

    width: u32,
    height: u32,
}

impl DepthPeelPass {
    /// Creates a new depth peel pass with all required resources.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        mesh_bind_group_layout: &wgpu::BindGroupLayout,
        slice_plane_bind_group_layout: &wgpu::BindGroupLayout,
        matcap_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("depth peel sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create textures
        // Min-depth uses Rgba16Float (blendable) instead of R32Float (not blendable).
        // Only the R channel is used; the Max blend keeps the furthest peeled depth.
        let (min_depth_texture, min_depth_view) =
            Self::create_rgba16_texture(device, width, height, "peel min depth");
        let (peel_color_texture, peel_color_view) =
            Self::create_rgba16_texture(device, width, height, "peel layer color");
        let (peel_depth_color_texture, peel_depth_color_view) =
            Self::create_r32float_texture(device, width, height, "peel layer depth color");
        let (peel_depth_texture, peel_depth_view) =
            Self::create_depth_texture(device, width, height);
        let (final_texture, final_view) =
            Self::create_rgba16_texture(device, width, height, "peel final");

        // --- Bind group layouts ---

        // Peel shader Group 3: min-depth texture + sampler
        let peel_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("peel depth bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let peel_bind_group = Self::create_peel_bind_group(
            device,
            &peel_bind_group_layout,
            &min_depth_view,
            &sampler,
        );

        // Composite shader: image texture + sampler
        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("peel composite bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Depth update shader: only needs texture (uses textureLoad, no sampler)
        let depth_update_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("peel depth update bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // --- Pipelines ---

        let peel_pipeline = Self::create_peel_pipeline(
            device,
            mesh_bind_group_layout,
            slice_plane_bind_group_layout,
            matcap_bind_group_layout,
            &peel_bind_group_layout,
        );

        let under_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let over_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let composite_under_pipeline = Self::create_composite_pipeline(
            device,
            &composite_bind_group_layout,
            "composite peel (under)",
            under_blend,
        );

        let composite_over_pipeline = Self::create_composite_pipeline(
            device,
            &composite_bind_group_layout,
            "composite peel (over)",
            over_blend,
        );

        let depth_update_pipeline =
            Self::create_depth_update_pipeline(device, &depth_update_bind_group_layout);

        Self {
            peel_pipeline,
            composite_under_pipeline,
            composite_over_pipeline,
            depth_update_pipeline,
            min_depth_texture,
            min_depth_view,
            peel_color_texture,
            peel_color_view,
            peel_depth_color_texture,
            peel_depth_color_view,
            peel_depth_texture,
            peel_depth_view,
            final_texture,
            final_view,
            peel_bind_group_layout,
            peel_bind_group,
            composite_bind_group_layout,
            depth_update_bind_group_layout,
            sampler,
            width,
            height,
        }
    }

    /// Resizes all textures. Call when window size changes.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;

        let (t, v) = Self::create_rgba16_texture(device, width, height, "peel min depth");
        self.min_depth_texture = t;
        self.min_depth_view = v;

        let (t, v) = Self::create_rgba16_texture(device, width, height, "peel layer color");
        self.peel_color_texture = t;
        self.peel_color_view = v;

        let (t, v) = Self::create_r32float_texture(device, width, height, "peel layer depth color");
        self.peel_depth_color_texture = t;
        self.peel_depth_color_view = v;

        let (t, v) = Self::create_depth_texture(device, width, height);
        self.peel_depth_texture = t;
        self.peel_depth_view = v;

        let (t, v) = Self::create_rgba16_texture(device, width, height, "peel final");
        self.final_texture = t;
        self.final_view = v;

        // Recreate bind group for min-depth
        self.peel_bind_group = Self::create_peel_bind_group(
            device,
            &self.peel_bind_group_layout,
            &self.min_depth_view,
            &self.sampler,
        );
    }

    /// Returns the peel pipeline for external use.
    pub fn peel_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.peel_pipeline
    }

    /// Returns the peel bind group (Group 3 for min-depth texture).
    pub fn peel_bind_group(&self) -> &wgpu::BindGroup {
        &self.peel_bind_group
    }

    /// Returns the peel color view (render target for each peel pass).
    pub fn peel_color_view(&self) -> &wgpu::TextureView {
        &self.peel_color_view
    }

    /// Returns the peel depth color view (depth-as-color output).
    pub fn peel_depth_color_view(&self) -> &wgpu::TextureView {
        &self.peel_depth_color_view
    }

    /// Returns the peel depth view (actual depth buffer).
    pub fn peel_depth_view(&self) -> &wgpu::TextureView {
        &self.peel_depth_view
    }

    /// Returns the min-depth view.
    pub fn min_depth_view(&self) -> &wgpu::TextureView {
        &self.min_depth_view
    }

    /// Returns the final accumulated view.
    pub fn final_view(&self) -> &wgpu::TextureView {
        &self.final_view
    }

    /// Composites the current peel layer into the final buffer using alpha-under blending.
    pub fn composite_layer(&self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("peel composite bind group"),
            layout: &self.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.peel_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("peel composite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.final_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep previously accumulated layers
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.composite_under_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Updates the min-depth texture from the current peel's depth-as-color output.
    /// Uses Max blend to keep the furthest depth seen so far.
    pub fn update_min_depth(&self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("peel depth update bind group"),
            layout: &self.depth_update_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.peel_depth_color_view),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("peel min-depth update pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.min_depth_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep previous min-depth
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.depth_update_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Composites the final peeled result onto the HDR scene buffer.
    pub fn composite_final_to_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("peel final composite bind group"),
            layout: &self.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.final_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("peel final-to-scene composite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep opaque scene content
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.composite_over_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    // --- Private helpers ---

    fn create_r32float_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_rgba16_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("peel depth buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_peel_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        min_depth_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("peel depth bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(min_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    fn create_peel_pipeline(
        device: &wgpu::Device,
        mesh_bind_group_layout: &wgpu::BindGroupLayout,
        slice_plane_bind_group_layout: &wgpu::BindGroupLayout,
        matcap_bind_group_layout: &wgpu::BindGroupLayout,
        peel_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader_source = include_str!("shaders/surface_mesh_peel.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface mesh peel shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh peel pipeline layout"),
            bind_group_layouts: &[
                mesh_bind_group_layout,         // Group 0
                slice_plane_bind_group_layout,  // Group 1
                matcap_bind_group_layout,       // Group 2
                peel_bind_group_layout,         // Group 3
            ],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("surface mesh peel pipeline"),
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
                targets: &[
                    // Color output (Rgba16Float, premultiplied alpha)
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None, // No blending — single layer per pass
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // Depth-as-color output (R32Float) for min-depth tracking
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
                depth_write_enabled: true, // Write depth for this layer
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn create_composite_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        label: &str,
        blend: wgpu::BlendState,
    ) -> wgpu::RenderPipeline {
        let shader_source = include_str!("shaders/composite_peel.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite peel shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite peel pipeline layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
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
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn create_depth_update_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        // Dedicated shader using textureLoad (no sampler needed for R32Float).
        // Uses Max blend to keep the furthest depth.
        let shader_source = include_str!("shaders/depth_update_peel.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depth update shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depth update pipeline layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth update pipeline"),
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
                    format: wgpu::TextureFormat::Rgba16Float,
                    // Max blend: keeps the maximum of src and dst.
                    // Uses Rgba16Float because R32Float is not blendable in WebGPU
                    // without the float32-blendable feature.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }
}
