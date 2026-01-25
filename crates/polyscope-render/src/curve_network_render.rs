//! Curve network GPU rendering resources.

use glam::Vec3;
use wgpu::util::DeviceExt;

/// Uniforms for curve network rendering.
/// Layout must match WGSL CurveNetworkUniforms exactly (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CurveNetworkUniforms {
    /// Base color (RGBA)
    pub color: [f32; 4],
    /// Radius for nodes and edges
    pub radius: f32,
    /// Whether radius is relative to scene scale (0 = absolute, 1 = relative)
    pub radius_is_relative: u32,
    /// Render mode: 0 = line, 1 = tube (cylinder)
    pub render_mode: u32,
    /// Padding to 16-byte alignment
    pub _padding: f32,
}

impl Default for CurveNetworkUniforms {
    fn default() -> Self {
        Self {
            color: [0.2, 0.5, 0.8, 1.0],
            radius: 0.005,
            radius_is_relative: 1,
            render_mode: 0, // lines by default
            _padding: 0.0,
        }
    }
}

/// GPU resources for rendering a curve network.
pub struct CurveNetworkRenderData {
    /// Node position buffer (storage buffer, vec4 for alignment).
    pub node_buffer: wgpu::Buffer,
    /// Node color buffer (storage buffer, vec4).
    pub node_color_buffer: wgpu::Buffer,

    /// Edge vertex buffer - contains tail and tip positions per edge.
    /// Layout: [tail0, tip0, tail1, tip1, ...] (vec4 each for alignment)
    pub edge_vertex_buffer: wgpu::Buffer,
    /// Edge color buffer (per-edge colors, vec4).
    pub edge_color_buffer: wgpu::Buffer,

    /// Uniform buffer for curve network settings.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for this curve network.
    pub bind_group: wgpu::BindGroup,

    /// Number of nodes.
    pub num_nodes: u32,
    /// Number of edges.
    pub num_edges: u32,

    // Tube rendering resources
    /// Generated vertex buffer from compute shader (36 vertices per edge).
    pub generated_vertex_buffer: Option<wgpu::Buffer>,
    /// Buffer containing num_edges as uniform.
    pub num_edges_buffer: Option<wgpu::Buffer>,
    /// Bind group for tube compute shader.
    pub compute_bind_group: Option<wgpu::BindGroup>,
    /// Bind group for tube render shader.
    pub tube_render_bind_group: Option<wgpu::BindGroup>,
}

impl CurveNetworkRenderData {
    /// Creates new render data from curve network geometry.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `bind_group_layout` - The bind group layout for curve networks
    /// * `camera_buffer` - The camera uniform buffer
    /// * `node_positions` - Node positions
    /// * `edge_tail_inds` - Edge start indices
    /// * `edge_tip_inds` - Edge end indices
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        node_positions: &[Vec3],
        edge_tail_inds: &[u32],
        edge_tip_inds: &[u32],
    ) -> Self {
        let num_nodes = node_positions.len() as u32;
        let num_edges = edge_tail_inds.len() as u32;

        // Create node position buffer (vec4 for alignment)
        let node_data: Vec<f32> = node_positions
            .iter()
            .flat_map(|p| [p.x, p.y, p.z, 1.0])
            .collect();
        let node_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network node positions"),
            contents: bytemuck::cast_slice(&node_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create node color buffer (default zero - shader uses base color when zero)
        let node_color_data: Vec<f32> = vec![0.0; node_positions.len() * 4];
        let node_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network node colors"),
            contents: bytemuck::cast_slice(&node_color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create edge vertex buffer - 2 vertices per edge (tail, tip)
        let mut edge_vertex_data: Vec<f32> = Vec::with_capacity(edge_tail_inds.len() * 8);
        for i in 0..edge_tail_inds.len() {
            let tail = node_positions[edge_tail_inds[i] as usize];
            let tip = node_positions[edge_tip_inds[i] as usize];
            edge_vertex_data.extend_from_slice(&[tail.x, tail.y, tail.z, 1.0]);
            edge_vertex_data.extend_from_slice(&[tip.x, tip.y, tip.z, 1.0]);
        }
        let edge_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network edge vertices"),
            contents: bytemuck::cast_slice(&edge_vertex_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create edge color buffer (default zero - shader uses base color when zero)
        let edge_color_data: Vec<f32> = vec![0.0; edge_tail_inds.len() * 4];
        let edge_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network edge colors"),
            contents: bytemuck::cast_slice(&edge_color_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create uniform buffer
        let uniforms = CurveNetworkUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve network uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        // Bindings:
        // 0: camera uniforms (uniform)
        // 1: curve network uniforms (uniform)
        // 2: node positions (storage)
        // 3: node colors (storage)
        // 4: edge vertices (storage)
        // 5: edge colors (storage)
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("curve network bind group"),
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
                    resource: node_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: node_color_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: edge_vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: edge_color_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            node_buffer,
            node_color_buffer,
            edge_vertex_buffer,
            edge_color_buffer,
            uniform_buffer,
            bind_group,
            num_nodes,
            num_edges,
            generated_vertex_buffer: None,
            num_edges_buffer: None,
            compute_bind_group: None,
            tube_render_bind_group: None,
        }
    }

    /// Initializes tube rendering resources.
    pub fn init_tube_resources(
        &mut self,
        device: &wgpu::Device,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        render_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        // Create generated vertex buffer (36 vertices per edge, 32 bytes per vertex)
        let vertex_buffer_size = (self.num_edges as usize * 36 * 32) as u64;
        let generated_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Curve Network Generated Vertices"),
            size: vertex_buffer_size.max(32), // Minimum size
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Create num_edges uniform buffer
        let num_edges_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Curve Network Num Edges"),
            contents: bytemuck::cast_slice(&[self.num_edges]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create compute bind group
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Curve Network Tube Compute Bind Group"),
            layout: compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.edge_vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: generated_vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: num_edges_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render bind group
        let tube_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Curve Network Tube Render Bind Group"),
            layout: render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.edge_vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.edge_color_buffer.as_entire_binding(),
                },
            ],
        });

        self.generated_vertex_buffer = Some(generated_vertex_buffer);
        self.num_edges_buffer = Some(num_edges_buffer);
        self.compute_bind_group = Some(compute_bind_group);
        self.tube_render_bind_group = Some(tube_render_bind_group);
    }

    /// Returns whether tube resources are initialized.
    pub fn has_tube_resources(&self) -> bool {
        self.generated_vertex_buffer.is_some()
    }

    /// Updates the uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CurveNetworkUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Updates node colors.
    pub fn update_node_colors(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect();
        queue.write_buffer(
            &self.node_color_buffer,
            0,
            bytemuck::cast_slice(&color_data),
        );
    }

    /// Updates edge colors.
    pub fn update_edge_colors(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let color_data: Vec<f32> = colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect();
        queue.write_buffer(
            &self.edge_color_buffer,
            0,
            bytemuck::cast_slice(&color_data),
        );
    }

    /// Updates node positions.
    pub fn update_node_positions(&self, queue: &wgpu::Queue, positions: &[Vec3]) {
        let pos_data: Vec<f32> = positions
            .iter()
            .flat_map(|p| [p.x, p.y, p.z, 1.0])
            .collect();
        queue.write_buffer(&self.node_buffer, 0, bytemuck::cast_slice(&pos_data));
    }

    /// Updates edge vertices (when node positions change).
    pub fn update_edge_vertices(
        &self,
        queue: &wgpu::Queue,
        node_positions: &[Vec3],
        edge_tail_inds: &[u32],
        edge_tip_inds: &[u32],
    ) {
        let mut edge_vertex_data: Vec<f32> = Vec::with_capacity(edge_tail_inds.len() * 8);
        for i in 0..edge_tail_inds.len() {
            let tail = node_positions[edge_tail_inds[i] as usize];
            let tip = node_positions[edge_tip_inds[i] as usize];
            edge_vertex_data.extend_from_slice(&[tail.x, tail.y, tail.z, 1.0]);
            edge_vertex_data.extend_from_slice(&[tip.x, tip.y, tip.z, 1.0]);
        }
        queue.write_buffer(
            &self.edge_vertex_buffer,
            0,
            bytemuck::cast_slice(&edge_vertex_data),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_network_uniforms_default() {
        let uniforms = CurveNetworkUniforms::default();

        assert_eq!(uniforms.color, [0.2, 0.5, 0.8, 1.0]);
        assert_eq!(uniforms.radius, 0.005);
        assert_eq!(uniforms.radius_is_relative, 1);
        assert_eq!(uniforms.render_mode, 0);
    }

    #[test]
    fn test_curve_network_uniforms_size() {
        let size = std::mem::size_of::<CurveNetworkUniforms>();

        // Should be 32 bytes:
        // color: 16 bytes ([f32; 4])
        // radius: 4 bytes (f32)
        // radius_is_relative: 4 bytes (u32)
        // render_mode: 4 bytes (u32)
        // _padding: 4 bytes (f32)
        // Total: 32 bytes
        assert_eq!(size, 32, "CurveNetworkUniforms should be 32 bytes");

        // Must be 16-byte aligned for GPU uniform buffers
        assert_eq!(size % 16, 0, "CurveNetworkUniforms must be 16-byte aligned");
    }
}
