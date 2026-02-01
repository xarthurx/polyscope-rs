//! Volume grid GPU rendering resources for gridcube and isosurface visualization.

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::surface_mesh_render::ShadowModelUniforms;

/// Colormap texture resolution (number of samples).
const COLORMAP_RESOLUTION: u32 = 256;

/// Uniforms for the simple mesh (isosurface) shader.
/// Layout must match WGSL `SimpleMeshUniforms` exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct SimpleMeshUniforms {
    /// Model transform matrix.
    pub model: [[f32; 4]; 4],
    /// Base surface color (RGBA).
    pub base_color: [f32; 4],
    /// Transparency (0.0 = opaque, 1.0 = fully transparent).
    pub transparency: f32,
    /// Slice plane clipping enable: 0 = off, 1 = on.
    pub slice_planes_enabled: u32,
    /// Backface policy: 0 = identical, 1 = different, 3 = cull.
    pub backface_policy: u32,
    /// Padding to 16-byte alignment.
    pub _pad: f32,
}

impl Default for SimpleMeshUniforms {
    fn default() -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            base_color: [0.047, 0.451, 0.690, 1.0], // default isosurface blue
            transparency: 0.0,
            slice_planes_enabled: 1,
            backface_policy: 0,
            _pad: 0.0,
        }
    }
}

/// Uniforms for the gridcube shader.
/// Layout must match WGSL `GridcubeUniforms` exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct GridcubeUniforms {
    /// Model transform matrix.
    pub model: [[f32; 4]; 4],
    /// Cube size factor (0..1, default 1.0 = full size, 0.5 = half).
    pub cube_size_factor: f32,
    /// Scalar data range minimum.
    pub data_min: f32,
    /// Scalar data range maximum.
    pub data_max: f32,
    /// Transparency (0.0 = opaque, 1.0 = fully transparent).
    pub transparency: f32,
    /// Slice plane clipping enable: 0 = off, 1 = on.
    pub slice_planes_enabled: u32,
    /// Padding to 16-byte alignment.
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for GridcubeUniforms {
    fn default() -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            cube_size_factor: 1.0,
            data_min: 0.0,
            data_max: 1.0,
            transparency: 0.0,
            slice_planes_enabled: 1,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// GPU uniforms for gridcube pick rendering.
///
/// Layout must match WGSL `GridcubePickUniforms` exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct GridcubePickUniforms {
    /// Model transform matrix.
    pub model: [[f32; 4]; 4],
    /// The starting global index for this quantity's elements.
    pub global_start: u32,
    /// Cube size factor (0..1).
    pub cube_size_factor: f32,
    /// Padding to 16-byte alignment.
    pub _pad0: f32,
    pub _pad1: f32,
}

impl Default for GridcubePickUniforms {
    fn default() -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            global_start: 0,
            cube_size_factor: 1.0,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

/// GPU resources for isosurface (simple mesh) visualization.
pub struct IsosurfaceRenderData {
    /// Position buffer (storage, vec4 per expanded triangle vertex).
    pub vertex_buffer: wgpu::Buffer,
    /// Normal buffer (storage, vec4 per expanded triangle vertex).
    pub normal_buffer: wgpu::Buffer,
    /// Uniform buffer.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group (Group 0).
    pub bind_group: wgpu::BindGroup,
    /// Number of vertices (expanded triangle vertices, for non-indexed draw).
    pub num_vertices: u32,
    /// Shadow pass bind group.
    pub shadow_bind_group: Option<wgpu::BindGroup>,
    /// Shadow model uniform buffer.
    pub shadow_model_buffer: Option<wgpu::Buffer>,
}

impl IsosurfaceRenderData {
    /// Creates new isosurface render data from marching cubes output.
    ///
    /// Vertices/normals are expanded per-triangle (non-indexed drawing with storage buffers),
    /// matching the surface mesh pattern.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        vertices: &[Vec3],
        normals: &[Vec3],
        indices: &[u32],
    ) -> Self {
        // Expand to per-triangle-vertex layout (non-indexed)
        let num_triangles = indices.len() / 3;
        let num_vertices = (num_triangles * 3) as u32;

        let mut expanded_positions: Vec<f32> = Vec::with_capacity(num_triangles * 3 * 4);
        let mut expanded_normals: Vec<f32> = Vec::with_capacity(num_triangles * 3 * 4);

        for tri_idx in 0..num_triangles {
            for v in 0..3 {
                let vi = indices[tri_idx * 3 + v] as usize;
                let p = vertices[vi];
                expanded_positions.extend_from_slice(&[p.x, p.y, p.z, 1.0]);
                let n = normals[vi];
                expanded_normals.extend_from_slice(&[n.x, n.y, n.z, 0.0]);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("isosurface vertices"),
            contents: bytemuck::cast_slice(&expanded_positions),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("isosurface normals"),
            contents: bytemuck::cast_slice(&expanded_normals),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms = SimpleMeshUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("isosurface uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("isosurface bind group"),
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
            ],
        });

        Self {
            vertex_buffer,
            normal_buffer,
            uniform_buffer,
            bind_group,
            num_vertices,
            shadow_bind_group: None,
            shadow_model_buffer: None,
        }
    }

    /// Updates the uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &SimpleMeshUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Initializes shadow rendering resources.
    pub fn init_shadow_resources(
        &mut self,
        device: &wgpu::Device,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
        light_buffer: &wgpu::Buffer,
    ) {
        let shadow_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("isosurface shadow model buffer"),
            contents: bytemuck::cast_slice(&[ShadowModelUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("isosurface shadow bind group"),
            layout: shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: shadow_model_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.vertex_buffer.as_entire_binding(),
                },
            ],
        });

        self.shadow_model_buffer = Some(shadow_model_buffer);
        self.shadow_bind_group = Some(shadow_bind_group);
    }

    /// Returns whether shadow resources have been initialized.
    #[must_use]
    pub fn has_shadow_resources(&self) -> bool {
        self.shadow_bind_group.is_some()
    }

    /// Updates the shadow model uniform buffer with the current transform.
    pub fn update_shadow_model(&self, queue: &wgpu::Queue, model_matrix: [[f32; 4]; 4]) {
        if let Some(buffer) = &self.shadow_model_buffer {
            let uniforms = ShadowModelUniforms {
                model: model_matrix,
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }
}

/// GPU resources for gridcube visualization.
pub struct GridcubeRenderData {
    /// Combined buffer: first 36 entries are unit cube template vertices (vec4),
    /// followed by per-instance data (vec4: xyz=center, w=`half_size`).
    pub position_buffer: wgpu::Buffer,
    /// Cube template normals (36 entries, vec4).
    pub normal_buffer: wgpu::Buffer,
    /// Per-instance scalar values.
    pub scalar_buffer: wgpu::Buffer,
    /// Uniform buffer.
    pub uniform_buffer: wgpu::Buffer,
    /// Colormap texture (1D, RGBA).
    pub colormap_texture: wgpu::Texture,
    /// Colormap texture view.
    pub colormap_view: wgpu::TextureView,
    /// Colormap sampler.
    pub colormap_sampler: wgpu::Sampler,
    /// Bind group (Group 0).
    pub bind_group: wgpu::BindGroup,
    /// Number of instances (grid nodes/cells).
    pub num_instances: u32,
    /// Shadow pass bind group.
    pub shadow_bind_group: Option<wgpu::BindGroup>,
    /// Shadow model uniform buffer.
    pub shadow_model_buffer: Option<wgpu::Buffer>,
}

/// Generates the 36 vertices and 36 normals for a unit cube ([-0.5, 0.5]^3).
/// Returns (positions, normals) as vec4 arrays.
fn generate_unit_cube() -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    // 6 faces, 2 triangles each, 3 vertices each = 36 vertices
    // Face order: +X, -X, +Y, -Y, +Z, -Z
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        // +X face (normal = +X)
        ([1.0, 0.0, 0.0], [
            [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5],
        ]),
        // -X face
        ([-1.0, 0.0, 0.0], [
            [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5],
        ]),
        // +Y face
        ([0.0, 1.0, 0.0], [
            [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5],
        ]),
        // -Y face
        ([0.0, -1.0, 0.0], [
            [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5],
        ]),
        // +Z face
        ([0.0, 0.0, 1.0], [
            [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
        ]),
        // -Z face
        ([0.0, 0.0, -1.0], [
            [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5],
        ]),
    ];

    let mut positions = Vec::with_capacity(36);
    let mut normals = Vec::with_capacity(36);

    for (normal, verts) in &faces {
        // Two triangles per face: 0-1-2 and 0-2-3
        let tri_indices = [[0, 1, 2], [0, 2, 3]];
        for tri in &tri_indices {
            for &vi in tri {
                let v = verts[vi];
                positions.push([v[0], v[1], v[2], 1.0]);
                normals.push([normal[0], normal[1], normal[2], 0.0]);
            }
        }
    }

    (positions, normals)
}

/// Creates a 1D colormap texture from color samples.
fn create_colormap_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    colors: &[Vec3],
) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
    // Sample the colormap at COLORMAP_RESOLUTION points
    let mut pixel_data: Vec<u8> = Vec::with_capacity(COLORMAP_RESOLUTION as usize * 4);
    let n = colors.len();

    for i in 0..COLORMAP_RESOLUTION {
        let t = i as f32 / (COLORMAP_RESOLUTION - 1) as f32;
        let t_clamped = t.clamp(0.0, 1.0);

        // Linear interpolation (matches ColorMap::sample)
        let color = if n <= 1 {
            colors.first().copied().unwrap_or(Vec3::ZERO)
        } else {
            let segments = n - 1;
            let idx = (t_clamped * segments as f32).floor() as usize;
            let idx = idx.min(segments - 1);
            let frac = t_clamped * segments as f32 - idx as f32;
            colors[idx].lerp(colors[idx + 1], frac)
        };

        pixel_data.push((color.x * 255.0) as u8);
        pixel_data.push((color.y * 255.0) as u8);
        pixel_data.push((color.z * 255.0) as u8);
        pixel_data.push(255); // alpha
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colormap texture"),
        size: wgpu::Extent3d {
            width: COLORMAP_RESOLUTION,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixel_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(COLORMAP_RESOLUTION * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: COLORMAP_RESOLUTION,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D1),
        ..Default::default()
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("colormap sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    (texture, view, sampler)
}

impl GridcubeRenderData {
    /// Creates new gridcube render data.
    ///
    /// # Arguments
    /// * `centers` - Per-instance cube center positions
    /// * `half_size` - Half the cube side length (grid spacing / 2)
    /// * `scalars` - Per-instance scalar values
    /// * `colormap_colors` - Color samples for the colormap
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        centers: &[Vec3],
        half_size: f32,
        scalars: &[f32],
        colormap_colors: &[Vec3],
    ) -> Self {
        let num_instances = centers.len() as u32;
        let (cube_positions, cube_normals) = generate_unit_cube();

        // Build combined position buffer: 36 cube template verts + N instance data entries
        let mut position_data: Vec<[f32; 4]> = cube_positions;
        for center in centers {
            position_data.push([center.x, center.y, center.z, half_size]);
        }

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube positions"),
            contents: bytemuck::cast_slice(&position_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube normals"),
            contents: bytemuck::cast_slice(&cube_normals),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let scalar_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube scalars"),
            contents: bytemuck::cast_slice(scalars),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms = GridcubeUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (colormap_texture, colormap_view, colormap_sampler) =
            create_colormap_texture(device, queue, colormap_colors);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gridcube bind group"),
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
                    resource: normal_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: scalar_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&colormap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&colormap_sampler),
                },
            ],
        });

        Self {
            position_buffer,
            normal_buffer,
            scalar_buffer,
            uniform_buffer,
            colormap_texture,
            colormap_view,
            colormap_sampler,
            bind_group,
            num_instances,
            shadow_bind_group: None,
            shadow_model_buffer: None,
        }
    }

    /// Updates the uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &GridcubeUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Updates the colormap texture with new colors.
    pub fn update_colormap(&self, queue: &wgpu::Queue, colors: &[Vec3]) {
        let mut pixel_data: Vec<u8> = Vec::with_capacity(COLORMAP_RESOLUTION as usize * 4);
        let n = colors.len();

        for i in 0..COLORMAP_RESOLUTION {
            let t = i as f32 / (COLORMAP_RESOLUTION - 1) as f32;
            let t_clamped = t.clamp(0.0, 1.0);
            let color = if n <= 1 {
                colors.first().copied().unwrap_or(Vec3::ZERO)
            } else {
                let segments = n - 1;
                let idx = (t_clamped * segments as f32).floor() as usize;
                let idx = idx.min(segments - 1);
                let frac = t_clamped * segments as f32 - idx as f32;
                colors[idx].lerp(colors[idx + 1], frac)
            };

            pixel_data.push((color.x * 255.0) as u8);
            pixel_data.push((color.y * 255.0) as u8);
            pixel_data.push((color.z * 255.0) as u8);
            pixel_data.push(255);
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.colormap_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixel_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(COLORMAP_RESOLUTION * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: COLORMAP_RESOLUTION,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Returns the total number of vertices to draw (36 per instance).
    #[must_use]
    pub fn total_vertices(&self) -> u32 {
        36 * self.num_instances
    }

    /// Initializes shadow rendering resources.
    pub fn init_shadow_resources(
        &mut self,
        device: &wgpu::Device,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
        light_buffer: &wgpu::Buffer,
    ) {
        let shadow_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gridcube shadow model buffer"),
            contents: bytemuck::cast_slice(&[ShadowModelUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gridcube shadow bind group"),
            layout: shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: shadow_model_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.position_buffer.as_entire_binding(),
                },
            ],
        });

        self.shadow_model_buffer = Some(shadow_model_buffer);
        self.shadow_bind_group = Some(shadow_bind_group);
    }

    /// Returns whether shadow resources have been initialized.
    #[must_use]
    pub fn has_shadow_resources(&self) -> bool {
        self.shadow_bind_group.is_some()
    }

    /// Updates the shadow model uniform buffer.
    pub fn update_shadow_model(&self, queue: &wgpu::Queue, model_matrix: [[f32; 4]; 4]) {
        if let Some(buffer) = &self.shadow_model_buffer {
            let uniforms = ShadowModelUniforms {
                model: model_matrix,
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_mesh_uniforms_size() {
        let size = std::mem::size_of::<SimpleMeshUniforms>();
        assert_eq!(size % 16, 0, "SimpleMeshUniforms size ({size} bytes) must be 16-byte aligned");
        // model(64) + base_color(16) + transparency(4) + slice_planes_enabled(4) + backface_policy(4) + pad(4) = 96
        assert_eq!(size, 96, "SimpleMeshUniforms should be 96 bytes, got {size}");
    }

    #[test]
    fn test_gridcube_pick_uniforms_size() {
        let size = std::mem::size_of::<GridcubePickUniforms>();
        assert_eq!(size % 16, 0, "GridcubePickUniforms size ({size} bytes) must be 16-byte aligned");
        // model(64) + global_start(4) + cube_size_factor(4) + pad0(4) + pad1(4) = 80
        assert_eq!(size, 80, "GridcubePickUniforms should be 80 bytes, got {size}");
    }

    #[test]
    fn test_gridcube_uniforms_size() {
        let size = std::mem::size_of::<GridcubeUniforms>();
        assert_eq!(size % 16, 0, "GridcubeUniforms size ({size} bytes) must be 16-byte aligned");
        // model(64) + cube_size_factor(4) + data_min(4) + data_max(4) + transparency(4)
        // + slice_planes_enabled(4) + pad0(4) + pad1(4) + pad2(4) = 96
        assert_eq!(size, 96, "GridcubeUniforms should be 96 bytes, got {size}");
    }

    #[test]
    fn test_unit_cube_generation() {
        let (positions, normals) = generate_unit_cube();
        assert_eq!(positions.len(), 36);
        assert_eq!(normals.len(), 36);

        // All positions should be within [-0.5, 0.5]
        for p in &positions {
            assert!(p[0].abs() <= 0.5 + f32::EPSILON);
            assert!(p[1].abs() <= 0.5 + f32::EPSILON);
            assert!(p[2].abs() <= 0.5 + f32::EPSILON);
            assert!((p[3] - 1.0).abs() < f32::EPSILON); // w = 1.0
        }

        // All normals should be unit length axis-aligned
        for n in &normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 0.01);
            assert!((n[3]).abs() < f32::EPSILON); // w = 0.0
        }
    }
}
