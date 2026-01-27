//! Slice mesh rendering for volume mesh cross-section capping.
//!
//! Renders the triangulated slice geometry using the surface mesh shader.

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::surface_mesh_render::MeshUniforms;

/// GPU resources for rendering a slice mesh (volume cross-section cap).
///
/// Uses the same shader as surface meshes but with simplified geometry.
pub struct SliceMeshRenderData {
    /// Position buffer (storage buffer, vec4 for alignment).
    vertex_buffer: wgpu::Buffer,
    /// Index buffer (triangle indices).
    index_buffer: wgpu::Buffer,
    /// Normal buffer (vertex normals, vec4 for alignment).
    normal_buffer: wgpu::Buffer,
    /// Barycentric coordinate buffer for wireframe rendering (kept alive for `bind_group`).
    _barycentric_buffer: wgpu::Buffer,
    /// Color buffer (per-vertex colors).
    color_buffer: wgpu::Buffer,
    /// Edge is real buffer (kept alive for `bind_group`).
    _edge_is_real_buffer: wgpu::Buffer,
    /// Uniform buffer for mesh-specific settings.
    uniform_buffer: wgpu::Buffer,
    /// Bind group for this slice mesh.
    bind_group: wgpu::BindGroup,
    /// Number of indices (`num_triangles` * 3).
    num_indices: u32,
}

impl SliceMeshRenderData {
    /// Creates new render data from slice mesh geometry.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `bind_group_layout` - The bind group layout for surface meshes
    /// * `camera_buffer` - The camera uniform buffer
    /// * `vertices` - Vertex positions (3 per triangle)
    /// * `normals` - Vertex normals (3 per triangle)
    /// * `colors` - Vertex colors (3 per triangle)
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        normals: &[Vec3],
        colors: &[Vec3],
    ) -> Self {
        let num_vertices = vertices.len() as u32;
        let num_indices = num_vertices;

        // Expand vertex data to vec4 format
        let mut position_data: Vec<f32> = Vec::with_capacity(vertices.len() * 4);
        let mut normal_data: Vec<f32> = Vec::with_capacity(normals.len() * 4);
        let mut color_data: Vec<f32> = Vec::with_capacity(colors.len() * 4);
        let mut barycentric_data: Vec<f32> = Vec::with_capacity(vertices.len() * 4);
        let mut edge_is_real_data: Vec<f32> = Vec::with_capacity(vertices.len() * 4);

        for (i, v) in vertices.iter().enumerate() {
            position_data.extend_from_slice(&[v.x, v.y, v.z, 1.0]);

            let n = normals[i];
            normal_data.extend_from_slice(&[n.x, n.y, n.z, 0.0]);

            let c = colors[i];
            color_data.extend_from_slice(&[c.x, c.y, c.z, 1.0]);

            // Barycentric coordinates for wireframe (cycle through triangle vertices)
            let bary = match i % 3 {
                0 => [1.0, 0.0, 0.0],
                1 => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
            barycentric_data.extend_from_slice(&[bary[0], bary[1], bary[2], 0.0]);

            // All edges are "real" for slice mesh
            edge_is_real_data.extend_from_slice(&[1.0, 1.0, 1.0, 0.0]);
        }

        // Create buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh vertices"),
            contents: bytemuck::cast_slice(&position_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let index_data: Vec<u32> = (0..num_indices).collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh normals"),
            contents: bytemuck::cast_slice(&normal_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let barycentric_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh barycentrics"),
            contents: bytemuck::cast_slice(&barycentric_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh colors"),
            contents: bytemuck::cast_slice(&color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let edge_is_real_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh edge_is_real"),
            contents: bytemuck::cast_slice(&edge_is_real_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create uniform buffer with interior color styling
        let mut uniforms = MeshUniforms::default();
        uniforms.shade_style = 1; // Flat shading for slice cap
        uniforms.show_edges = 0; // No edges by default
        uniforms.backface_policy = 0; // Identical front/back

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slice mesh uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slice mesh bind group"),
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
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: normal_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: barycentric_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: color_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: edge_is_real_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            vertex_buffer,
            index_buffer,
            normal_buffer,
            _barycentric_buffer: barycentric_buffer,
            color_buffer,
            _edge_is_real_buffer: edge_is_real_buffer,
            uniform_buffer,
            bind_group,
            num_indices,
        }
    }

    /// Updates the slice mesh geometry.
    ///
    /// This recreates the buffers if the geometry size has changed.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        normals: &[Vec3],
        colors: &[Vec3],
    ) {
        // For simplicity, recreate if size differs (slice geometry can change significantly)
        let new_num_indices = vertices.len() as u32;
        if new_num_indices != self.num_indices {
            *self = Self::new(
                device,
                bind_group_layout,
                camera_buffer,
                vertices,
                normals,
                colors,
            );
            return;
        }

        // Update buffers in place
        let mut position_data: Vec<f32> = Vec::with_capacity(vertices.len() * 4);
        let mut normal_data: Vec<f32> = Vec::with_capacity(normals.len() * 4);
        let mut color_data: Vec<f32> = Vec::with_capacity(colors.len() * 4);

        for (i, v) in vertices.iter().enumerate() {
            position_data.extend_from_slice(&[v.x, v.y, v.z, 1.0]);

            let n = normals[i];
            normal_data.extend_from_slice(&[n.x, n.y, n.z, 0.0]);

            let c = colors[i];
            color_data.extend_from_slice(&[c.x, c.y, c.z, 1.0]);
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&position_data));
        queue.write_buffer(&self.normal_buffer, 0, bytemuck::cast_slice(&normal_data));
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&color_data));
    }

    /// Updates the uniform buffer with new settings.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, color: Vec3) {
        let mut uniforms = MeshUniforms::default();
        uniforms.shade_style = 1; // Flat shading
        uniforms.show_edges = 0;
        uniforms.surface_color = [color.x, color.y, color.z, 1.0];
        uniforms.backface_policy = 0;

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the bind group for rendering.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the index buffer for rendering.
    #[must_use]
    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    /// Returns the number of indices to draw.
    #[must_use]
    pub fn num_indices(&self) -> u32 {
        self.num_indices
    }

    /// Returns true if the slice mesh is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.num_indices == 0
    }
}
