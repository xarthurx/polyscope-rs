//! Planar shadow rendering using projection-based approach.
//!
//! This implements shadows by projecting scene geometry onto the ground plane,
//! matching the approach used in C++ Polyscope. The shadows are rendered using
//! a modified view matrix that flattens geometry onto the ground plane, then
//! blurred and sampled using screen coordinates.

use glam::Mat4;
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

/// Uniforms for the depth-to-mask shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DepthToMaskUniforms {
    /// Unused padding for alignment
    pub _padding: [f32; 4],
}

impl Default for DepthToMaskUniforms {
    fn default() -> Self {
        Self { _padding: [0.0; 4] }
    }
}

/// Uniforms for the blur shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurUniforms {
    /// Blur direction: (1,0) for horizontal, (0,1) for vertical
    pub direction: [f32; 2],
    /// Texel size: 1.0 / texture_size
    pub texel_size: [f32; 2],
}

/// Planar shadow pass resources.
///
/// Implements shadow rendering by projecting geometry onto the ground plane.
pub struct PlanarShadowPass {
    /// Shadow depth texture (render target for projected scene)
    shadow_depth_texture: wgpu::Texture,
    shadow_depth_view: wgpu::TextureView,
    /// Shadow framebuffer (no color, depth only for rendering projected scene)
    /// We'll render to depth, then convert to color

    /// Color textures for ping-pong blur
    blur_textures: [wgpu::Texture; 2],
    blur_views: [wgpu::TextureView; 2],

    /// Depth-to-mask pipeline (converts depth buffer to shadow mask)
    depth_to_mask_pipeline: wgpu::RenderPipeline,
    depth_to_mask_bind_group_layout: wgpu::BindGroupLayout,

    /// Blur pipeline
    blur_pipeline: wgpu::RenderPipeline,
    blur_bind_group_layout: wgpu::BindGroupLayout,
    blur_uniform_buffer: wgpu::Buffer,

    /// Linear sampler for blur
    linear_sampler: wgpu::Sampler,

    /// Current texture dimensions
    width: u32,
    height: u32,

    /// Bind group layout for ground plane shader to sample the shadow
    pub shadow_sample_bind_group_layout: wgpu::BindGroupLayout,
}

impl PlanarShadowPass {
    /// Creates a new planar shadow pass.
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // Use half resolution for performance (shadows are blurry anyway)
        let shadow_width = width / 2;
        let shadow_height = height / 2;

        // Create shadow depth texture
        let shadow_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Planar Shadow Depth"),
            size: wgpu::Extent3d {
                width: shadow_width,
                height: shadow_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_depth_view =
            shadow_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create blur textures (RGBA for blurring)
        let create_blur_texture = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: shadow_width,
                    height: shadow_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm, // Single channel for shadow mask
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let blur_texture_0 = create_blur_texture("Planar Shadow Blur 0");
        let blur_texture_1 = create_blur_texture("Planar Shadow Blur 1");
        let blur_view_0 = blur_texture_0.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_view_1 = blur_texture_1.create_view(&wgpu::TextureViewDescriptor::default());

        // Linear sampler
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Planar Shadow Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Depth-to-mask bind group layout
        let depth_to_mask_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Depth to Mask Bind Group Layout"),
                entries: &[
                    // Depth texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Depth-to-mask shader
        let depth_to_mask_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Depth to Mask Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/depth_to_mask.wgsl").into()),
        });

        let depth_to_mask_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Depth to Mask Pipeline Layout"),
                bind_group_layouts: &[&depth_to_mask_bind_group_layout],
                push_constant_ranges: &[],
            });

        let depth_to_mask_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Depth to Mask Pipeline"),
                layout: Some(&depth_to_mask_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &depth_to_mask_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &depth_to_mask_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: None,
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
            });

        // Blur bind group layout
        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Planar Shadow Blur Bind Group Layout"),
                entries: &[
                    // Input texture
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
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(16),
                        },
                        count: None,
                    },
                ],
            });

        // Blur shader
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Planar Shadow Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shadow_blur.wgsl").into()),
        });

        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Planar Shadow Blur Pipeline Layout"),
            bind_group_layouts: &[&blur_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Planar Shadow Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
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
        });

        // Blur uniform buffer
        let blur_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Planar Shadow Blur Uniform Buffer"),
            contents: bytemuck::cast_slice(&[BlurUniforms {
                direction: [1.0, 0.0],
                texel_size: [1.0 / shadow_width as f32, 1.0 / shadow_height as f32],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layout for ground plane shader to sample the final shadow
        let shadow_sample_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Planar Shadow Sample Bind Group Layout"),
                entries: &[
                    // Shadow texture
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
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        Self {
            shadow_depth_texture,
            shadow_depth_view,
            blur_textures: [blur_texture_0, blur_texture_1],
            blur_views: [blur_view_0, blur_view_1],
            depth_to_mask_pipeline,
            depth_to_mask_bind_group_layout,
            blur_pipeline,
            blur_bind_group_layout,
            blur_uniform_buffer,
            linear_sampler,
            width: shadow_width,
            height: shadow_height,
            shadow_sample_bind_group_layout,
        }
    }

    /// Resizes the shadow textures if needed.
    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        let shadow_width = width / 2;
        let shadow_height = height / 2;

        if shadow_width == self.width && shadow_height == self.height {
            return;
        }

        self.width = shadow_width;
        self.height = shadow_height;

        // Recreate depth texture
        self.shadow_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Planar Shadow Depth"),
            size: wgpu::Extent3d {
                width: shadow_width,
                height: shadow_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.shadow_depth_view = self
            .shadow_depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate blur textures
        for (i, label) in ["Planar Shadow Blur 0", "Planar Shadow Blur 1"]
            .iter()
            .enumerate()
        {
            self.blur_textures[i] = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: shadow_width,
                    height: shadow_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.blur_views[i] = self.blur_textures[i]
                .create_view(&wgpu::TextureViewDescriptor::default());
        }

        // Update blur uniforms
        queue.write_buffer(
            &self.blur_uniform_buffer,
            0,
            bytemuck::cast_slice(&[BlurUniforms {
                direction: [1.0, 0.0],
                texel_size: [1.0 / shadow_width as f32, 1.0 / shadow_height as f32],
            }]),
        );
    }

    /// Computes the planar projection matrix that flattens geometry onto the ground plane.
    ///
    /// This modifies the view matrix to project all geometry onto the ground at `ground_height`.
    /// For Y-up coordinate systems, this zeroes out the Y component and translates to ground height.
    #[must_use]
    pub fn compute_planar_projection_matrix(ground_height: f32) -> Mat4 {
        // Projection matrix that flattens Y to ground_height
        // This is: translate(-groundHeight) * flatten_Y * translate(groundHeight)
        // Which simplifies to: set Y column to 0, set Y translation to groundHeight
        Mat4::from_cols_array(&[
            1.0, 0.0, 0.0, 0.0, // X column unchanged
            0.0, 0.0, 0.0, 0.0, // Y column zeroed (flatten)
            0.0, 0.0, 1.0, 0.0, // Z column unchanged
            0.0, ground_height, 0.0, 1.0, // Translation: move to ground height
        ])
    }

    /// Returns the depth view for shadow rendering.
    #[must_use]
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.shadow_depth_view
    }

    /// Begins the shadow depth pass.
    ///
    /// Returns a render pass configured to render to the shadow depth texture.
    pub fn begin_shadow_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Planar Shadow Depth Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }

    /// Runs the depth-to-mask and blur passes.
    ///
    /// This converts the shadow depth buffer to a blurred shadow mask texture.
    pub fn process_shadow(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        blur_iterations: u32,
    ) {
        // Step 1: Convert depth to mask
        let depth_to_mask_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth to Mask Bind Group"),
            layout: &self.depth_to_mask_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.shadow_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth to Mask Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.blur_views[0],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.depth_to_mask_pipeline);
            pass.set_bind_group(0, &depth_to_mask_bind_group, &[]);
            pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // Step 2: Blur passes (ping-pong between textures)
        for i in 0..blur_iterations {
            let src_idx = (i % 2) as usize;
            let dst_idx = ((i + 1) % 2) as usize;

            // Determine blur direction
            let direction = if i % 2 == 0 {
                [1.0, 0.0] // Horizontal
            } else {
                [0.0, 1.0] // Vertical
            };

            // Update blur uniforms
            queue.write_buffer(
                &self.blur_uniform_buffer,
                0,
                bytemuck::cast_slice(&[BlurUniforms {
                    direction,
                    texel_size: [1.0 / self.width as f32, 1.0 / self.height as f32],
                }]),
            );

            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Planar Shadow Blur Bind Group"),
                layout: &self.blur_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.blur_views[src_idx]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blur_uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Planar Shadow Blur Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.blur_views[dst_idx],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.blur_pipeline);
                pass.set_bind_group(0, &blur_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }

    /// Returns the final shadow texture view (after blur).
    ///
    /// The result depends on the number of blur iterations:
    /// - Even iterations: result in blur_views[0]
    /// - Odd iterations: result in blur_views[1]
    #[must_use]
    pub fn shadow_texture_view(&self, blur_iterations: u32) -> &wgpu::TextureView {
        // After N blur iterations, result is in blur_views[N % 2]
        &self.blur_views[(blur_iterations % 2) as usize]
    }

    /// Creates a bind group for the ground plane shader to sample the shadow.
    #[must_use]
    pub fn create_shadow_sample_bind_group(
        &self,
        device: &wgpu::Device,
        blur_iterations: u32,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Planar Shadow Sample Bind Group"),
            layout: &self.shadow_sample_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        self.shadow_texture_view(blur_iterations),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        })
    }

    /// Returns the sampler.
    #[must_use]
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.linear_sampler
    }
}
