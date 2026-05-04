//! Curve network GPU rendering resources.

use glam::{Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::point_cloud_render::PointUniforms;

/// Uniforms for curve network rendering.
/// Layout must match WGSL `CurveNetworkUniforms` exactly (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
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
    /// Buffer containing `num_edges` as uniform.
    pub num_edges_buffer: Option<wgpu::Buffer>,
    /// Bind group for tube compute shader.
    pub compute_bind_group: Option<wgpu::BindGroup>,
    /// Bind group for tube render shader.
    pub tube_render_bind_group: Option<wgpu::BindGroup>,

    // Node sphere rendering resources (for tube mode joint filling)
    /// Uniform buffer for node sphere rendering (matches `PointUniforms`).
    pub node_uniform_buffer: Option<wgpu::Buffer>,
    /// Bind group for node sphere rendering (uses point pipeline).
    pub node_render_bind_group: Option<wgpu::BindGroup>,
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
    #[must_use]
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
            node_uniform_buffer: None,
            node_render_bind_group: None,
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
    #[must_use]
    pub fn has_tube_resources(&self) -> bool {
        self.generated_vertex_buffer.is_some()
    }

    /// Initializes node sphere rendering resources for tube mode.
    /// Uses the point pipeline to render spheres at each node to fill gaps at joints.
    pub fn init_node_render_resources(
        &mut self,
        device: &wgpu::Device,
        point_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        // Create uniform buffer matching PointUniforms structure
        let uniforms = PointUniforms::default();
        let node_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Curve Network Node Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group matching point pipeline layout
        let node_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Curve Network Node Render Bind Group"),
            layout: point_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: node_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.node_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.node_color_buffer.as_entire_binding(),
                },
            ],
        });

        self.node_uniform_buffer = Some(node_uniform_buffer);
        self.node_render_bind_group = Some(node_render_bind_group);
    }

    /// Returns whether node render resources are initialized.
    #[must_use]
    pub fn has_node_render_resources(&self) -> bool {
        self.node_render_bind_group.is_some()
    }

    /// Updates node sphere uniforms (radius, color, etc.).
    pub fn update_node_uniforms(&self, queue: &wgpu::Queue, uniforms: &PointUniforms) {
        if let Some(buffer) = &self.node_uniform_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[*uniforms]));
        }
    }

    /// Updates the uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CurveNetworkUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Updates node colors.
    pub fn update_node_colors(&self, queue: &wgpu::Queue, colors: &[Vec4]) {
        let color_data: Vec<f32> = colors.iter().flat_map(glam::Vec4::to_array).collect();
        queue.write_buffer(
            &self.node_color_buffer,
            0,
            bytemuck::cast_slice(&color_data),
        );
    }

    /// Updates edge colors.
    pub fn update_edge_colors(&self, queue: &wgpu::Queue, colors: &[Vec4]) {
        let color_data: Vec<f32> = colors.iter().flat_map(glam::Vec4::to_array).collect();
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

    // ========================================================================
    // Shader-math validation: parallel-ray cylinder intersection.
    //
    // The three tube shaders (`curve_network_tube.wgsl`,
    // `reflected_curve_network_tube.wgsl`, `pick_curve_tube.wgsl`) include a
    // parallel-ray branch needed for ortho viewing straight down a tube. This
    // module mirrors that branch in Rust so its math can be unit-tested
    // (no GPU required). If the WGSL diverges from this Rust port, the
    // ortho head-on tube case will silently regress.
    // ========================================================================

    use glam::Vec3;

    /// Mirrors the parallel-ray branch in the three tube shaders.
    /// Returns Some((t, hit_point)) on hit, None on miss.
    /// Ignores normal direction since pick variant doesn't compute one.
    fn ray_cylinder_parallel_intersect(
        ray_origin: Vec3,
        ray_dir: Vec3,
        cyl_start: Vec3,
        cyl_end: Vec3,
        cyl_radius: f32,
    ) -> Option<(f32, Vec3)> {
        let cyl_axis = cyl_end - cyl_start;
        let cyl_dir = cyl_axis.normalize();
        let delta = ray_origin - cyl_start;
        let delta_perp = delta - cyl_dir.dot(delta) * cyl_dir;

        if delta_perp.length_squared() > cyl_radius * cyl_radius {
            return None;
        }
        let ray_dot_cyl = ray_dir.dot(cyl_dir);
        if ray_dot_cyl.abs() < 1e-8 {
            return None;
        }
        let t_start = (cyl_start - ray_origin).dot(cyl_dir) / ray_dot_cyl;
        let t_end = (cyl_end - ray_origin).dot(cyl_dir) / ray_dot_cyl;
        let mut t_cap = t_start.min(t_end);
        if t_cap < 0.001 {
            t_cap = t_start.max(t_end);
            if t_cap < 0.001 {
                return None;
            }
        }
        Some((t_cap, ray_origin + t_cap * ray_dir))
    }

    /// Camera looks straight down a Z-aligned tube in ortho mode. Ray origin
    /// is pushed back so t > 0. Should hit the front end cap (closer to camera).
    #[test]
    fn parallel_ray_through_axis_hits_front_cap() {
        let cyl_start = Vec3::new(0.0, 0.0, 0.0);
        let cyl_end = Vec3::new(0.0, 0.0, 5.0);
        let radius = 0.1_f32;
        // Camera at +Z looking down -Z (toward origin)
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        let world_position = Vec3::new(0.0, 0.0, 5.5); // on bbox front (toward camera)
        let extent = (cyl_end - cyl_start).length() + 2.0 * radius;
        let ray_origin = world_position - extent * ray_dir;

        let hit = ray_cylinder_parallel_intersect(ray_origin, ray_dir, cyl_start, cyl_end, radius);
        let (t, p) = hit.expect("parallel ray through axis should hit cylinder cap");
        assert!(t > 0.001, "t must be positive, got {t}");
        // Front cap is cyl_end (z=5.0); hit point z must equal cyl_end.z
        assert!(
            (p.z - cyl_end.z).abs() < 1e-4,
            "expected hit at z={}, got {p:?}",
            cyl_end.z
        );
    }

    /// Same setup, ray offset within radius — must still hit the cap (the
    /// disk is filled, not just the rim).
    #[test]
    fn parallel_ray_offset_within_radius_hits() {
        let cyl_start = Vec3::ZERO;
        let cyl_end = Vec3::new(0.0, 0.0, 5.0);
        let radius = 0.1_f32;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        // Offset 0.05 from axis (within radius=0.1)
        let world_position = Vec3::new(0.05, 0.0, 5.5);
        let extent = (cyl_end - cyl_start).length() + 2.0 * radius;
        let ray_origin = world_position - extent * ray_dir;

        let hit = ray_cylinder_parallel_intersect(ray_origin, ray_dir, cyl_start, cyl_end, radius);
        assert!(hit.is_some(), "ray within radius should hit cap");
    }

    /// Ray offset beyond radius — must miss.
    #[test]
    fn parallel_ray_offset_beyond_radius_misses() {
        let cyl_start = Vec3::ZERO;
        let cyl_end = Vec3::new(0.0, 0.0, 5.0);
        let radius = 0.1_f32;
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        let world_position = Vec3::new(0.5, 0.0, 5.5); // 5x radius from axis
        let ray_origin = world_position - 10.0 * ray_dir;

        let hit = ray_cylinder_parallel_intersect(ray_origin, ray_dir, cyl_start, cyl_end, radius);
        assert!(hit.is_none(), "ray outside radius must miss");
    }

    /// Reverse-direction ray (looking from -Z toward +Z) must hit the OTHER
    /// cap (cyl_start side, since that's now nearer to the camera).
    #[test]
    fn parallel_ray_reverse_direction_hits_other_cap() {
        let cyl_start = Vec3::new(0.0, 0.0, 0.0);
        let cyl_end = Vec3::new(0.0, 0.0, 5.0);
        let radius = 0.1_f32;
        let ray_dir = Vec3::new(0.0, 0.0, 1.0); // looking down +Z now
        let world_position = Vec3::new(0.0, 0.0, -0.5);
        let extent = (cyl_end - cyl_start).length() + 2.0 * radius;
        let ray_origin = world_position - extent * ray_dir;

        let (t, p) =
            ray_cylinder_parallel_intersect(ray_origin, ray_dir, cyl_start, cyl_end, radius)
                .expect("reverse-direction parallel ray should hit");
        assert!(t > 0.001);
        assert!(
            (p.z - cyl_start.z).abs() < 1e-4,
            "expected hit at z={}, got {p:?}",
            cyl_start.z
        );
    }
}
