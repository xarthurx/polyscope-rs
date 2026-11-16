//! Ground plane rendering.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::camera::AxisDirection;

/// Axis frame of the ground plane, derived from the up direction.
///
/// C++ Polyscope derives the ground plane from the up direction
/// (`getGroundPlaneAxisAndSign` in `ground_plane.cpp`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundPlaneFrame {
    /// Index of the up axis: 0 = X, 1 = Y, 2 = Z.
    pub axis: usize,
    /// 1 for a positive up direction, -1 for a negative up direction.
    pub sign: f32,
}

impl GroundPlaneFrame {
    /// Returns the frame for the up direction `up`.
    #[must_use]
    pub fn new(up: AxisDirection) -> Self {
        let (axis, sign) = match up {
            AxisDirection::PosX => (0, 1.0),
            AxisDirection::NegX => (0, -1.0),
            AxisDirection::PosY => (1, 1.0),
            AxisDirection::NegY => (1, -1.0),
            AxisDirection::PosZ => (2, 1.0),
            AxisDirection::NegZ => (2, -1.0),
        };
        Self { axis, sign }
    }

    /// Returns the unit vector along the up axis that points up.
    #[must_use]
    pub fn up(self) -> Vec3 {
        let mut v = Vec3::ZERO;
        v[self.axis] = self.sign;
        v
    }

    /// Returns the first tangent direction of the plane (C++ `baseForward`).
    #[must_use]
    pub fn forward(self) -> Vec3 {
        let mut v = Vec3::ZERO;
        v[(self.axis + 1) % 3] = self.sign;
        v
    }

    /// Returns the second tangent direction of the plane (C++ `baseRight`).
    #[must_use]
    pub fn right(self) -> Vec3 {
        let mut v = Vec3::ZERO;
        v[(self.axis + 2) % 3] = self.sign;
        v
    }

    /// Returns the ground height: the coordinate of the plane on the up axis.
    ///
    /// `height_override` is a coordinate on the up axis. Without it, the plane
    /// lies just below the bottom of the scene box, as seen along the up direction.
    #[must_use]
    pub fn ground_height(
        self,
        scene_bounds: (Vec3, Vec3),
        length_scale: f32,
        height_override: Option<f32>,
    ) -> f32 {
        height_override.unwrap_or_else(|| {
            let (min, max) = scene_bounds;
            let bottom = if self.sign > 0.0 {
                min[self.axis]
            } else {
                max[self.axis]
            };
            // A small offset (0.1% of the length scale) avoids z-fighting.
            bottom - self.sign * (length_scale * 0.001)
        })
    }

    /// Returns the heights of the plane and of the camera as offsets from
    /// `center` along [`Self::up`].
    ///
    /// The shader places the plane at `center + up * offset`.
    #[must_use]
    pub fn offsets_from_center(
        self,
        center: Vec3,
        ground_height: f32,
        camera_position: Vec3,
    ) -> (f32, f32) {
        let center_on_axis = center[self.axis];
        (
            (ground_height - center_on_axis) * self.sign,
            (camera_position[self.axis] - center_on_axis) * self.sign,
        )
    }

    /// Returns the matrix that reflects points across the ground plane at `height`.
    #[must_use]
    pub fn reflection_matrix(self, height: f32) -> Mat4 {
        let mut point = Vec3::ZERO;
        point[self.axis] = height;
        crate::reflection::reflection_matrix(point, self.up())
    }

    /// Returns the direction of the shadow light: mostly down, and tilted along
    /// the plane. For +Y up, this is the direction (0.5, -1.0, 0.3), normalized.
    #[must_use]
    pub fn light_direction(self) -> Vec3 {
        (0.5 * self.right() - self.up() + 0.3 * self.forward()).normalize()
    }
}

/// GPU representation of ground plane uniforms.
/// Matches the shader's `GroundUniforms` struct.
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
    /// Camera height above the scene center along `basis_z`, for the fade
    pub camera_height: f32,
    /// Sign factor of the heights in the fade calculation
    pub up_sign: f32,
    /// Shadow darkness (0.0 = no shadow, 1.0 = full black)
    pub shadow_darkness: f32,
    /// Shadow mode: 0=none, `1=shadow_only`, `2=tile_with_shadow`
    pub shadow_mode: u32,
    /// Whether camera is in orthographic mode (0=perspective, 1=orthographic)
    pub is_orthographic: u32,
    /// Reflection intensity (0.0 = no reflection/opaque ground, 1.0 = mirror)
    pub reflection_intensity: f32,
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
            shadow_darkness: 0.4,
            shadow_mode: 0, // No shadows by default
            is_orthographic: 0,
            reflection_intensity: 0.0, // No reflection by default
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
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `bind_group_layout` - The bind group layout
    /// * `camera_buffer` - The camera uniform buffer
    /// * `light_buffer` - The light uniform buffer (from `ShadowMapPass`)
    /// * `shadow_depth_view` - The shadow map depth texture view
    /// * `shadow_sampler` - The shadow comparison sampler
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        light_buffer: &wgpu::Buffer,
        shadow_depth_view: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(shadow_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(shadow_sampler),
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
    /// * `frame` - Axis frame of the ground plane (from the up direction)
    /// * `ground_height` - Coordinate of the plane on the up axis
    /// * `length_scale` - Scene length scale
    /// * `camera_position` - Current camera position
    /// * `shadow_darkness` - Shadow darkness (0.0 = no shadow, 1.0 = full black)
    /// * `shadow_mode` - Shadow mode: 0=none, `1=shadow_only`, `2=tile_with_shadow`
    /// * `is_orthographic` - Whether camera is in orthographic mode
    /// * `reflection_intensity` - Reflection intensity (0.0 = opaque, 1.0 = mirror)
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        queue: &wgpu::Queue,
        scene_center: [f32; 3],
        frame: GroundPlaneFrame,
        ground_height: f32,
        length_scale: f32,
        camera_position: Vec3,
        shadow_darkness: f32,
        shadow_mode: u32,
        is_orthographic: bool,
        reflection_intensity: f32,
    ) {
        let (height, camera_height) =
            frame.offsets_from_center(Vec3::from(scene_center), ground_height, camera_position);

        let uniforms = GroundPlaneUniforms {
            center: [scene_center[0], scene_center[1], scene_center[2], 0.0],
            basis_x: frame.forward().extend(0.0).to_array(),
            basis_y: frame.right().extend(0.0).to_array(),
            basis_z: frame.up().extend(0.0).to_array(),
            height,
            length_scale,
            camera_height,
            // basis_z points up, so the sign is already in the heights.
            up_sign: 1.0,
            shadow_darkness,
            shadow_mode,
            is_orthographic: u32::from(is_orthographic),
            reflection_intensity,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Returns the bind group for rendering.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_UP: [AxisDirection; 6] = [
        AxisDirection::PosX,
        AxisDirection::NegX,
        AxisDirection::PosY,
        AxisDirection::NegY,
        AxisDirection::PosZ,
        AxisDirection::NegZ,
    ];

    /// For +Y up, the frame equals the basis that was hard-coded before.
    #[test]
    fn y_up_frame_matches_previous_basis() {
        let frame = GroundPlaneFrame::new(AxisDirection::PosY);
        assert_eq!(frame.forward(), Vec3::Z);
        assert_eq!(frame.right(), Vec3::X);
        assert_eq!(frame.up(), Vec3::Y);
        assert_eq!(
            frame.light_direction(),
            Vec3::new(0.5, -1.0, 0.3).normalize()
        );
        let reflection = frame.reflection_matrix(-0.25);
        assert_eq!(
            reflection,
            crate::reflection::ground_reflection_matrix(-0.25)
        );
    }

    /// For every up direction, the plane lies under the scene box and is
    /// perpendicular to the up direction (C++ `ground_plane.cpp`).
    #[test]
    fn ground_plane_lies_under_the_scene_for_every_up_direction() {
        let bounds = (Vec3::new(-1.0, 2.0, 0.0), Vec3::new(1.0, 3.0, 4.0));
        let center = (bounds.0 + bounds.1) * 0.5;
        let length_scale = (bounds.1 - bounds.0).length();
        for up_dir in ALL_UP {
            let frame = GroundPlaneFrame::new(up_dir);
            let up = up_dir.to_vec3();
            assert_eq!(frame.up(), up, "{}", up_dir.name());
            // The plane tangents are unit vectors perpendicular to up and to each other.
            for v in [frame.forward(), frame.right()] {
                assert!((v.length() - 1.0).abs() < 1e-6);
                assert!(v.dot(up).abs() < 1e-6);
            }
            assert!(frame.forward().dot(frame.right()).abs() < 1e-6);

            // The shader puts the plane origin at center + up * offset.
            let ground_height = frame.ground_height(bounds, length_scale, None);
            let camera = center + up * 3.0;
            let (offset, camera_offset) = frame.offsets_from_center(center, ground_height, camera);
            let origin = center + up * offset;
            // Under every corner of the box, by a small gap.
            let lowest = [bounds.0, bounds.1]
                .iter()
                .map(|p| p.dot(up))
                .fold(f32::INFINITY, f32::min);
            let gap = lowest - origin.dot(up);
            assert!(
                gap > 0.0 && gap < 0.01 * length_scale,
                "{}: gap {gap}",
                up_dir.name()
            );
            // The origin sits below the center, and the camera is above the plane.
            assert!((origin - center).cross(up).length() < 1e-6);
            assert!(camera_offset - offset > 0.0);

            // The shadow light points down; the reflection keeps plane points.
            assert!(frame.light_direction().dot(up) < -0.5);
            let reflection = frame.reflection_matrix(ground_height);
            let on_plane = origin + frame.forward() * 2.0 - frame.right();
            assert!(reflection.transform_point3(on_plane).distance(on_plane) < 1e-5);
            let above = on_plane + up;
            assert!(reflection.transform_point3(above).distance(on_plane - up) < 1e-5);
        }
    }

    /// A manual height is a coordinate on the up axis.
    #[test]
    fn manual_height_is_a_coordinate_on_the_up_axis() {
        let bounds = (Vec3::splat(-1.0), Vec3::splat(1.0));
        let frame = GroundPlaneFrame::new(AxisDirection::NegZ);
        let height = frame.ground_height(bounds, 1.0, Some(0.5));
        assert_eq!(height, 0.5);
        let center = Vec3::new(3.0, 4.0, 1.0);
        let (offset, _) = frame.offsets_from_center(center, height, Vec3::ZERO);
        assert!(((center + frame.up() * offset).z - 0.5).abs() < 1e-6);
    }
}
