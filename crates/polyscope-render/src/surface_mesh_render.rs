//! Surface mesh GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// Uniforms for surface mesh rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    /// Shading style: 0 = smooth, 1 = flat, 2 = tri-flat
    pub shade_style: u32,
    /// Show edges: 0 = off, 1 = on
    pub show_edges: u32,
    /// Edge width in pixels
    pub edge_width: f32,
    /// Surface transparency (0.0 = opaque, 1.0 = fully transparent)
    pub transparency: f32,
    /// Surface color (RGBA)
    pub surface_color: [f32; 4],
    /// Edge color (RGBA)
    pub edge_color: [f32; 4],
    /// Backface policy: 0 = identical, 1 = different, 2 = custom, 3 = cull
    pub backface_policy: u32,
    /// Padding for 16-byte alignment
    pub _padding: [f32; 3],
    /// Backface color (RGBA), used when backface_policy is custom
    pub backface_color: [f32; 4],
}

impl Default for MeshUniforms {
    fn default() -> Self {
        Self {
            shade_style: 0,                      // smooth shading
            show_edges: 0,                       // edges off
            edge_width: 1.0,                     // 1 pixel edge
            transparency: 0.0,                   // fully opaque
            surface_color: [0.5, 0.5, 0.5, 1.0], // gray
            edge_color: [0.0, 0.0, 0.0, 1.0],    // black edges
            backface_policy: 0,                  // identical to front
            _padding: [0.0; 3],
            backface_color: [0.3, 0.3, 0.3, 1.0], // darker gray
        }
    }
}

/// GPU resources for rendering a surface mesh.
pub struct SurfaceMeshRenderData {
    /// Position buffer (storage buffer, vec4 for alignment).
    pub vertex_buffer: wgpu::Buffer,
    /// Index buffer (triangle indices).
    pub index_buffer: wgpu::Buffer,
    /// Normal buffer (vertex normals, vec4 for alignment).
    pub normal_buffer: wgpu::Buffer,
    /// Barycentric coordinate buffer for wireframe rendering.
    /// Each triangle vertex gets [1,0,0], [0,1,0], [0,0,1].
    pub barycentric_buffer: wgpu::Buffer,
    /// Color buffer (per-vertex colors for quantities, vec4).
    pub color_buffer: wgpu::Buffer,
    /// Uniform buffer for mesh-specific settings.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for this surface mesh.
    pub bind_group: wgpu::BindGroup,
    /// Number of triangles.
    pub num_triangles: u32,
    /// Number of indices (num_triangles * 3).
    pub num_indices: u32,
}

impl SurfaceMeshRenderData {
    /// Creates new render data from mesh geometry.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `bind_group_layout` - The bind group layout for surface meshes
    /// * `camera_buffer` - The camera uniform buffer
    /// * `vertices` - Vertex positions
    /// * `triangles` - Triangle indices (each [u32; 3] is one triangle)
    /// * `vertex_normals` - Per-vertex normals
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        vertex_normals: &[Vec3],
    ) -> Self {
        let num_triangles = triangles.len() as u32;
        let num_indices = num_triangles * 3;

        // Create vertex position buffer (vec4 for alignment)
        let vertex_data: Vec<f32> = vertices.iter().flat_map(|v| [v.x, v.y, v.z, 1.0]).collect();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create index buffer
        let index_data: Vec<u32> = triangles.iter().flat_map(|t| [t[0], t[1], t[2]]).collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // Create normal buffer (vec4 for alignment)
        let normal_data: Vec<f32> = vertex_normals
            .iter()
            .flat_map(|n| [n.x, n.y, n.z, 0.0])
            .collect();
        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh normals"),
            contents: bytemuck::cast_slice(&normal_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create barycentric coordinate buffer
        // Each triangle vertex gets [1,0,0], [0,1,0], [0,0,1] for wireframe rendering
        let barycentric_data: Vec<f32> = (0..triangles.len())
            .flat_map(|_| {
                [
                    1.0, 0.0, 0.0, 0.0, // vertex 0: [1,0,0] + padding
                    0.0, 1.0, 0.0, 0.0, // vertex 1: [0,1,0] + padding
                    0.0, 0.0, 1.0, 0.0, // vertex 2: [0,0,1] + padding
                ]
            })
            .collect();
        let barycentric_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh barycentrics"),
            contents: bytemuck::cast_slice(&barycentric_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create color buffer (default white)
        let color_data: Vec<f32> = vec![1.0; vertices.len() * 4];
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh colors"),
            contents: bytemuck::cast_slice(&color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create uniform buffer
        let uniforms = MeshUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        // Bindings:
        // 0: camera uniforms (uniform)
        // 1: mesh uniforms (uniform)
        // 2: positions (storage)
        // 3: normals (storage)
        // 4: barycentrics (storage)
        // 5: colors (storage)
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface mesh bind group"),
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
            ],
        });

        Self {
            vertex_buffer,
            index_buffer,
            normal_buffer,
            barycentric_buffer,
            color_buffer,
            uniform_buffer,
            bind_group,
            num_triangles,
            num_indices,
        }
    }

    /// Updates the mesh uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &MeshUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Updates the per-vertex color buffer.
    pub fn update_colors(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect();
        queue.write_buffer(&self.color_buffer, 0, bytemuck::cast_slice(&color_data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_uniforms_default() {
        let uniforms = MeshUniforms::default();

        // Verify default values
        assert_eq!(
            uniforms.shade_style, 0,
            "default shade_style should be smooth (0)"
        );
        assert_eq!(
            uniforms.show_edges, 0,
            "default show_edges should be off (0)"
        );
        assert_eq!(uniforms.edge_width, 1.0, "default edge_width should be 1.0");
        assert_eq!(
            uniforms.transparency, 0.0,
            "default transparency should be 0.0 (opaque)"
        );
        assert_eq!(
            uniforms.surface_color,
            [0.5, 0.5, 0.5, 1.0],
            "default surface_color should be gray"
        );
        assert_eq!(
            uniforms.edge_color,
            [0.0, 0.0, 0.0, 1.0],
            "default edge_color should be black"
        );
        assert_eq!(
            uniforms.backface_policy, 0,
            "default backface_policy should be identical (0)"
        );
        assert_eq!(
            uniforms.backface_color,
            [0.3, 0.3, 0.3, 1.0],
            "default backface_color should be darker gray"
        );
    }

    #[test]
    fn test_mesh_uniforms_size() {
        let size = std::mem::size_of::<MeshUniforms>();

        // Verify size is 16-byte aligned for GPU uniform buffers
        assert_eq!(
            size % 16,
            0,
            "MeshUniforms size ({} bytes) must be 16-byte aligned",
            size
        );

        // Expected size breakdown:
        // shade_style: 4 bytes (u32)
        // show_edges: 4 bytes (u32)
        // edge_width: 4 bytes (f32)
        // transparency: 4 bytes (f32)
        // surface_color: 16 bytes ([f32; 4])
        // edge_color: 16 bytes ([f32; 4])
        // backface_policy: 4 bytes (u32)
        // _padding: 12 bytes ([f32; 3])
        // backface_color: 16 bytes ([f32; 4])
        // Total: 80 bytes
        assert_eq!(
            size, 80,
            "MeshUniforms should be 80 bytes, got {} bytes",
            size
        );
    }
}
