//! Ground plane rendering.

use polyscope_core::GroundPlaneConfig;
use wgpu::util::DeviceExt;

/// GPU representation of ground plane uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundPlaneUniforms {
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub height: f32,
    pub tile_size: f32,
    pub transparency: f32,
    pub _padding: f32,
}

impl Default for GroundPlaneUniforms {
    fn default() -> Self {
        Self {
            color1: [0.75, 0.75, 0.75, 1.0],
            color2: [0.55, 0.55, 0.55, 1.0],
            height: 0.0,
            tile_size: 1.0,
            transparency: 0.0,
            _padding: 0.0,
        }
    }
}

impl From<&GroundPlaneConfig> for GroundPlaneUniforms {
    fn from(config: &GroundPlaneConfig) -> Self {
        Self {
            color1: [config.color1[0], config.color1[1], config.color1[2], 1.0],
            color2: [config.color2[0], config.color2[1], config.color2[2], 1.0],
            height: config.height,
            tile_size: config.tile_size,
            transparency: config.transparency,
            _padding: 0.0,
        }
    }
}

/// Ground plane render resources.
pub struct GroundPlaneRenderData {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GroundPlaneRenderData {
    /// Creates new ground plane render data.
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let uniforms = GroundPlaneUniforms::default();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ground Plane Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ground Plane Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            uniform_buffer,
            bind_group,
        }
    }

    /// Updates the ground plane uniforms.
    pub fn update(&self, queue: &wgpu::Queue, config: &GroundPlaneConfig, scene_min_y: f32) {
        let mut uniforms = GroundPlaneUniforms::from(config);

        // If height is relative, place below scene
        if config.height_is_relative {
            uniforms.height = scene_min_y - 0.5 * config.tile_size;
        }

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the bind group for rendering.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
