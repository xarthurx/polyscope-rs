//! Point cloud GPU rendering resources.

use glam::{Vec3, Vec4};
use wgpu::util::DeviceExt;

/// GPU resources for rendering a point cloud.
pub struct PointCloudRenderData {
    /// Position buffer (storage buffer).
    pub position_buffer: wgpu::Buffer,
    /// Color buffer (storage buffer).
    pub color_buffer: wgpu::Buffer,
    /// Uniform buffer for point-specific settings.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for this point cloud.
    pub bind_group: wgpu::BindGroup,
    /// Number of points.
    pub num_points: u32,
}

/// Uniforms for point cloud rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct PointUniforms {
    pub model_matrix: [[f32; 4]; 4],
    pub point_radius: f32,
    pub use_per_point_color: u32,
    pub _padding: [f32; 2],
    pub base_color: [f32; 4],
}

impl Default for PointUniforms {
    fn default() -> Self {
        Self {
            model_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            point_radius: 0.01,
            use_per_point_color: 0,
            _padding: [0.0; 2],
            base_color: [0.2, 0.5, 0.8, 1.0], // Default blue
        }
    }
}

impl PointCloudRenderData {
    /// Creates new render data from point positions.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        positions: &[Vec3],
        colors: Option<&[Vec4]>,
    ) -> Self {
        let num_points = positions.len() as u32;

        // Create position buffer
        let position_data: Vec<f32> = positions
            .iter()
            .flat_map(|p| [p.x, p.y, p.z, 0.0]) // pad to vec4 for alignment
            .collect();
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point positions"),
            contents: bytemuck::cast_slice(&position_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create color buffer (default white if not provided)
        let color_data: Vec<f32> = if let Some(colors) = colors {
            colors.iter().flat_map(|c| c.to_array()).collect()
        } else {
            vec![1.0; positions.len() * 4]
        };
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point colors"),
            contents: bytemuck::cast_slice(&color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create uniform buffer
        let uniforms = PointUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point cloud bind group"),
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
                    resource: position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: color_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            position_buffer,
            color_buffer,
            uniform_buffer,
            bind_group,
            num_points,
        }
    }

    /// Updates the color buffer.
    pub fn update_colors(&self, queue: &wgpu::Queue, colors: &[Vec4]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| c.to_array()).collect();
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&color_data));
    }

    /// Updates uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &PointUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
}
