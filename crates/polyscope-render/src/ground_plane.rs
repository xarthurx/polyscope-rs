//! Ground plane rendering.

use wgpu::util::DeviceExt;

/// GPU representation of ground plane uniforms.
/// Matches the shader's GroundUniforms struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundPlaneUniforms {
    /// Scene center (xyz) + padding
    pub center: [f32; 4],
    /// Forward direction on ground plane (basis X)
    pub basis_x: [f32; 4],
    /// Right direction on ground plane (basis Y)
    pub basis_y: [f32; 4],
    /// Up direction / normal to ground (basis Z)
    pub basis_z: [f32; 4],
    /// Ground plane height
    pub height: f32,
    /// Scene length scale for tiling
    pub length_scale: f32,
    /// Camera height for fade calculation
    pub camera_height: f32,
    /// +1 or -1 depending on up direction
    pub up_sign: f32,
}

impl Default for GroundPlaneUniforms {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, 0.0, 0.0],
            basis_x: [0.0, 0.0, 1.0, 0.0], // Z forward
            basis_y: [1.0, 0.0, 0.0, 0.0], // X right
            basis_z: [0.0, 1.0, 0.0, 0.0], // Y up
            height: 0.0,
            length_scale: 1.0,
            camera_height: 5.0,
            up_sign: 1.0,
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
    ///
    /// # Arguments
    /// * `queue` - The wgpu queue
    /// * `scene_center` - Center of the scene bounding box
    /// * `scene_min_y` - Minimum Y coordinate of scene bounding box
    /// * `length_scale` - Scene length scale
    /// * `camera_height` - Current camera Y position
    /// * `height_override` - Optional manual height override
    pub fn update(
        &self,
        queue: &wgpu::Queue,
        scene_center: [f32; 3],
        scene_min_y: f32,
        length_scale: f32,
        camera_height: f32,
        height_override: Option<f32>,
    ) {
        // Compute ground height
        let height = height_override.unwrap_or_else(|| {
            // Place slightly below scene (like original Polyscope)
            scene_min_y - length_scale * 0.01
        });

        let uniforms = GroundPlaneUniforms {
            center: [scene_center[0], scene_center[1], scene_center[2], 0.0],
            // Y-up coordinate system: X=right, Z=forward, Y=up
            basis_x: [0.0, 0.0, 1.0, 0.0], // Forward (Z)
            basis_y: [1.0, 0.0, 0.0, 0.0], // Right (X)
            basis_z: [0.0, 1.0, 0.0, 0.0], // Up (Y)
            height,
            length_scale,
            camera_height,
            up_sign: 1.0, // Y is up, so positive
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the bind group for rendering.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
