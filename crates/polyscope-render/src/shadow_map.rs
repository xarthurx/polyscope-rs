//! Shadow map generation and blur passes.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Shadow map resolution.
pub const SHADOW_MAP_SIZE: u32 = 2048;

/// GPU representation of light uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
}

impl Default for LightUniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.5, -1.0, 0.3, 0.0],
        }
    }
}

/// GPU representation of blur uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurUniforms {
    pub direction: [f32; 2],
    pub texel_size: [f32; 2],
}

impl Default for BlurUniforms {
    fn default() -> Self {
        let texel_size = 1.0 / SHADOW_MAP_SIZE as f32;
        Self {
            direction: [1.0, 0.0],
            texel_size: [texel_size, texel_size],
        }
    }
}

/// Shadow map render resources.
#[allow(dead_code)]
pub struct ShadowMapPass {
    /// Shadow map depth texture.
    depth_texture: wgpu::Texture,
    /// Shadow map depth view.
    depth_view: wgpu::TextureView,
    /// Light uniform buffer.
    light_buffer: wgpu::Buffer,
    /// Comparison sampler for shadow sampling.
    comparison_sampler: wgpu::Sampler,
    /// Bind group layout for consumers (ground plane shader).
    pub shadow_bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for shadow sampling.
    shadow_bind_group: wgpu::BindGroup,
}

impl ShadowMapPass {
    /// Creates a new shadow map pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        // Create depth texture for shadow map
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Depth"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Light uniform buffer
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: bytemuck::cast_slice(&[LightUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Comparison sampler for shadow mapping
        let comparison_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        // Bind group layout for shadow sampling (used by ground plane shader)
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Bind Group Layout"),
                entries: &[
                    // Light uniforms
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
                    // Shadow map texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Create bind group for shadow sampling
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&comparison_sampler),
                },
            ],
        });

        Self {
            depth_texture,
            depth_view,
            light_buffer,
            comparison_sampler,
            shadow_bind_group_layout,
            shadow_bind_group,
        }
    }

    /// Computes the light view-projection matrix for shadow mapping.
    ///
    /// Creates an orthographic projection from the light's perspective that
    /// encompasses the scene.
    #[must_use]
    pub fn compute_light_matrix(scene_center: Vec3, scene_radius: f32, light_dir: Vec3) -> Mat4 {
        let light_dir = light_dir.normalize();
        let light_pos = scene_center - light_dir * scene_radius * 2.0;

        // Find a stable up vector that's not parallel to light direction
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let view = Mat4::look_at_rh(light_pos, scene_center, up);
        let proj = Mat4::orthographic_rh(
            -scene_radius,
            scene_radius,
            -scene_radius,
            scene_radius,
            0.1,
            scene_radius * 4.0,
        );
        proj * view
    }

    /// Updates the light uniforms.
    pub fn update_light(&self, queue: &wgpu::Queue, view_proj: Mat4, light_dir: Vec3) {
        let uniforms = LightUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
        };
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Clears the shadow map depth buffer.
    ///
    /// Call this before rendering scene objects to the shadow map.
    pub fn begin_shadow_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shadow Map Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }

    /// Returns the shadow map depth view.
    #[must_use]
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Returns the light uniform buffer.
    #[must_use]
    pub fn light_buffer(&self) -> &wgpu::Buffer {
        &self.light_buffer
    }

    /// Returns the comparison sampler.
    #[must_use]
    pub fn comparison_sampler(&self) -> &wgpu::Sampler {
        &self.comparison_sampler
    }

    /// Returns the bind group for shadow sampling.
    #[must_use]
    pub fn shadow_bind_group(&self) -> &wgpu::BindGroup {
        &self.shadow_bind_group
    }

    /// Returns the bind group layout for shadow sampling.
    #[must_use]
    pub fn shadow_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.shadow_bind_group_layout
    }
}
