//! Slice plane visualization rendering.
//!
//! Renders slice planes as semi-transparent grids.

use glam::{Mat4, Vec3, Vec4};
use polyscope_core::slice_plane::SlicePlane;
use wgpu::util::DeviceExt;

/// GPU representation of slice plane visualization uniforms.
/// Matches the shader's `PlaneUniforms` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct PlaneRenderUniforms {
    /// Plane's object transform matrix.
    pub transform: [[f32; 4]; 4],
    /// Base color of the plane.
    pub color: [f32; 4],
    /// Color of the grid lines.
    pub grid_color: [f32; 4],
    /// Length scale for grid sizing.
    pub length_scale: f32,
    /// Size of the plane visualization (half-extent in each direction).
    pub plane_size: f32,
    /// Padding for alignment.
    pub _padding: [f32; 2],
}

impl Default for PlaneRenderUniforms {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY.to_cols_array_2d(),
            color: [0.5, 0.5, 0.5, 1.0],
            grid_color: [0.3, 0.3, 0.3, 1.0],
            length_scale: 1.0,
            plane_size: 0.05,
            _padding: [0.0; 2],
        }
    }
}

/// Computes the transform matrix for a slice plane.
///
/// The plane lies in X=0 in local space, with Y and Z as tangent directions.
/// The transform positions and orients the plane quad in world space.
fn compute_plane_transform(origin: Vec3, normal: Vec3) -> Mat4 {
    // Build orthonormal basis for the plane
    // The normal becomes the local X axis (plane is at X=0)
    let x_axis = normal.normalize();

    // Choose an up vector that's not parallel to normal
    let up = if x_axis.dot(Vec3::Y).abs() < 0.99 {
        Vec3::Y
    } else {
        Vec3::Z
    };

    // Y axis is the first tangent direction
    let y_axis = up.cross(x_axis).normalize();
    // Z axis is the second tangent direction
    let z_axis = x_axis.cross(y_axis).normalize();

    // Create transform: translation + rotation
    Mat4::from_cols(
        Vec4::new(x_axis.x, x_axis.y, x_axis.z, 0.0),
        Vec4::new(y_axis.x, y_axis.y, y_axis.z, 0.0),
        Vec4::new(z_axis.x, z_axis.y, z_axis.z, 0.0),
        Vec4::new(origin.x, origin.y, origin.z, 1.0),
    )
}

/// Slice plane visualization render resources.
pub struct SlicePlaneRenderData {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SlicePlaneRenderData {
    /// Creates new slice plane render data.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `bind_group_layout` - The bind group layout
    /// * `camera_buffer` - The camera uniform buffer
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let uniforms = PlaneRenderUniforms::default();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slice Plane Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Plane Bind Group"),
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

    /// Updates the uniforms for a specific slice plane.
    ///
    /// # Arguments
    /// * `queue` - The wgpu queue
    /// * `plane` - The slice plane to visualize
    /// * `length_scale` - Scene length scale for grid sizing
    pub fn update(&self, queue: &wgpu::Queue, plane: &SlicePlane, length_scale: f32) {
        let transform = compute_plane_transform(plane.origin(), plane.normal());
        let color = plane.color();

        let uniforms = PlaneRenderUniforms {
            transform: transform.to_cols_array_2d(),
            color: [color.x, color.y, color.z, 1.0],
            grid_color: [color.x * 0.6, color.y * 0.6, color.z * 0.6, 1.0],
            length_scale,
            plane_size: plane.plane_size(),
            _padding: [0.0; 2],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the bind group for rendering.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Draws the slice plane visualization.
    ///
    /// Draws 6 vertices (2 triangles) forming a quad. Both sides are rendered
    /// because the pipeline has `cull_mode`: None.
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

/// Creates the bind group layout for slice plane rendering.
#[must_use]
pub fn create_slice_plane_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Slice Plane Bind Group Layout"),
        entries: &[
            // Camera uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Plane uniforms
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Creates the render pipeline for slice plane visualization.
#[must_use]
pub fn create_slice_plane_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("shaders/slice_plane.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Slice Plane Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Slice Plane Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Slice Plane Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Draw both sides
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: true, // Write depth so scene geometry can occlude the plane
            depth_compare: wgpu::CompareFunction::LessEqual, // Respect depth so nearer planes win
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}
