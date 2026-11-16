//! Planar reflection rendering pass.

use glam::Mat4;
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

/// GPU representation of reflection uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReflectionUniforms {
    pub reflection_matrix: [[f32; 4]; 4],
    pub intensity: f32,
    /// Coordinate of the ground plane on the up axis.
    pub ground_height: f32,
    /// Index of the up axis: 0 = X, 1 = Y, 2 = Z.
    pub up_axis: u32,
    /// 1 for a positive up direction, -1 for a negative up direction.
    pub up_sign: f32,
}

impl Default for ReflectionUniforms {
    fn default() -> Self {
        Self {
            reflection_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            intensity: 0.25,
            ground_height: 0.0,
            up_axis: 1,
            up_sign: 1.0,
        }
    }
}

/// Reflection pass render resources.
///
/// This pass handles planar reflections for the ground plane by:
/// 1. Rendering the ground plane to stencil buffer (marking reflection region)
/// 2. Rendering reflected scene geometry (only where stencil is set)
/// 3. Blending the reflection with the final ground plane
pub struct ReflectionPass {
    /// Reflection uniform buffer.
    uniform_buffer: wgpu::Buffer,
    /// Bind group layout for reflection uniforms.
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for reflection uniforms.
    bind_group: wgpu::BindGroup,
}

impl ReflectionPass {
    /// Creates a new reflection pass.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        // Create reflection uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Reflection Uniform Buffer"),
            contents: bytemuck::cast_slice(&[ReflectionUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout for reflection uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Reflection Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(80),
                },
                count: None,
            }],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflection Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    /// Updates the reflection uniforms.
    ///
    /// The shaders discard reflected geometry above the ground plane at
    /// `ground_height` on the up axis of `frame`.
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        reflection_matrix: Mat4,
        intensity: f32,
        ground_height: f32,
        frame: crate::ground_plane::GroundPlaneFrame,
    ) {
        let uniforms = ReflectionUniforms {
            reflection_matrix: reflection_matrix.to_cols_array_2d(),
            intensity,
            ground_height,
            up_axis: frame.axis as u32,
            up_sign: frame.sign,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the reflection bind group.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the reflection bind group layout.
    #[must_use]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_uniforms_size() {
        // Ensure uniform is correctly aligned for GPU
        assert_eq!(
            std::mem::size_of::<ReflectionUniforms>(),
            64 + 4 + 4 + 4 + 4 // 4x4 matrix + intensity + ground_height + up_axis + up_sign
        );
    }
}
