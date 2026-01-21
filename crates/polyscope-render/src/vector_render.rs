//! Vector arrow GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// GPU resources for rendering vectors.
pub struct VectorRenderData {
    pub base_buffer: wgpu::Buffer,
    pub vector_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub num_vectors: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VectorUniforms {
    pub length_scale: f32,
    pub radius: f32,
    pub _padding: [f32; 2],
    pub color: [f32; 4],
}

impl Default for VectorUniforms {
    fn default() -> Self {
        Self {
            length_scale: 1.0,
            radius: 0.005,
            _padding: [0.0; 2],
            color: [0.8, 0.2, 0.2, 1.0], // Red
        }
    }
}

impl VectorRenderData {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        bases: &[Vec3],
        vectors: &[Vec3],
    ) -> Self {
        let num_vectors = bases.len().min(vectors.len()) as u32;

        let base_data: Vec<f32> = bases.iter().flat_map(|p| [p.x, p.y, p.z, 0.0]).collect();
        let base_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector bases"),
            contents: bytemuck::cast_slice(&base_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let vector_data: Vec<f32> = vectors.iter().flat_map(|v| [v.x, v.y, v.z, 0.0]).collect();
        let vector_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vectors"),
            contents: bytemuck::cast_slice(&vector_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let uniforms = VectorUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vector uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vector bind group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vector_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            base_buffer,
            vector_buffer,
            uniform_buffer,
            bind_group,
            num_vectors,
        }
    }

    /// Updates vector uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &VectorUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}
