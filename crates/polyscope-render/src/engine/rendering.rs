use super::RenderEngine;
use crate::ground_plane::GroundPlaneRenderData;
use crate::slice_plane_render::SlicePlaneRenderData;

impl RenderEngine {
    /// Renders the ground plane.
    ///
    /// # Arguments
    /// * `encoder` - The command encoder
    /// * `view` - The render target view
    /// * `enabled` - Whether the ground plane is enabled
    /// * `scene_center` - Center of the scene bounding box
    /// * `scene_min_y` - Minimum Y coordinate of scene bounding box
    /// * `length_scale` - Scene length scale
    /// * `height_override` - Optional manual height (None = auto below scene)
    /// * `shadow_darkness` - Shadow darkness (0.0 = no shadow, 1.0 = full black)
    /// * `shadow_mode` - Shadow mode: 0=none, `1=shadow_only`, `2=tile_with_shadow`
    /// * `reflection_intensity` - Reflection intensity (0.0 = opaque, affects transparency)
    #[allow(clippy::too_many_arguments)]
    pub fn render_ground_plane(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        enabled: bool,
        scene_center: [f32; 3],
        scene_min_y: f32,
        length_scale: f32,
        height_override: Option<f32>,
        shadow_darkness: f32,
        shadow_mode: u32,
        reflection_intensity: f32,
    ) {
        // Check if camera is in orthographic mode
        let is_orthographic =
            self.camera.projection_mode == crate::camera::ProjectionMode::Orthographic;
        if !enabled {
            return;
        }

        // Always use HDR texture for ground plane rendering (pipelines use HDR format)
        let view = self.hdr_view.as_ref().unwrap_or(surface_view);

        // Initialize render data if needed
        if self.ground_plane_render_data.is_none() {
            if let Some(ref shadow_pass) = self.shadow_map_pass {
                self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
                    &self.device,
                    &self.ground_plane_bind_group_layout,
                    &self.camera_buffer,
                    shadow_pass.light_buffer(),
                    shadow_pass.depth_view(),
                    shadow_pass.comparison_sampler(),
                ));
            }
        }

        // Get camera height
        let camera_height = self.camera.position.y;

        if let Some(render_data) = &self.ground_plane_render_data {
            render_data.update(
                &self.queue,
                scene_center,
                scene_min_y,
                length_scale,
                camera_height,
                height_override,
                shadow_darkness,
                shadow_mode,
                is_orthographic,
                reflection_intensity,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ground Plane Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.ground_plane_pipeline);
            render_pass.set_bind_group(0, render_data.bind_group(), &[]);
            // 4 triangles * 3 vertices = 12 vertices for infinite plane
            render_pass.draw(0..12, 0..1);
        }
    }

    /// Renders slice plane visualizations.
    ///
    /// Renders enabled slice planes as semi-transparent grids.
    /// Should be called after rendering structures, before tone mapping.
    pub fn render_slice_planes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        planes: &[polyscope_core::slice_plane::SlicePlane],
        length_scale: f32,
    ) {
        // Use HDR texture if available
        let Some(view) = &self.hdr_view else {
            return;
        };

        // Ensure we have enough render data for all planes
        while self.slice_plane_render_data.len() < planes.len() {
            let data = SlicePlaneRenderData::new(
                &self.device,
                &self.slice_plane_vis_bind_group_layout,
                &self.camera_buffer,
            );
            self.slice_plane_render_data.push(data);
        }

        // Render each enabled plane that should be drawn
        for (i, plane) in planes.iter().enumerate() {
            if !plane.is_enabled() || !plane.draw_plane() {
                continue;
            }

            // Update uniforms for this plane
            self.slice_plane_render_data[i].update(&self.queue, plane, length_scale);

            // Begin render pass for this plane
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slice Plane Visualization Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.slice_plane_vis_pipeline);
            self.slice_plane_render_data[i].draw(&mut render_pass);
        }
    }

    /// Renders slice plane visualizations with clearing.
    ///
    /// Clears the HDR texture and depth buffer first, then renders slice planes.
    /// This should be called BEFORE rendering scene geometry so that geometry
    /// can properly occlude the slice planes.
    pub fn render_slice_planes_with_clear(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        planes: &[polyscope_core::slice_plane::SlicePlane],
        length_scale: f32,
        clear_color: [f32; 3],
    ) {
        // Use HDR texture if available
        let Some(view) = &self.hdr_view else {
            return;
        };

        // Ensure we have enough render data for all planes
        while self.slice_plane_render_data.len() < planes.len() {
            let data = SlicePlaneRenderData::new(
                &self.device,
                &self.slice_plane_vis_bind_group_layout,
                &self.camera_buffer,
            );
            self.slice_plane_render_data.push(data);
        }

        // Check if any planes need to be rendered
        let has_visible_planes = planes.iter().any(|p| p.is_enabled() && p.draw_plane());

        // First pass: clear the buffers
        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slice Plane Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear_color[0]),
                            g: f64::from(clear_color[1]),
                            b: f64::from(clear_color[2]),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            // Pass ends here, clearing is done
        }

        // Only render planes if there are visible ones
        if !has_visible_planes {
            return;
        }

        // Render each enabled plane that should be drawn
        for (i, plane) in planes.iter().enumerate() {
            if !plane.is_enabled() || !plane.draw_plane() {
                continue;
            }

            // Update uniforms for this plane
            self.slice_plane_render_data[i].update(&self.queue, plane, length_scale);

            // Begin render pass for this plane (loads existing content)
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slice Plane Visualization Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.slice_plane_vis_pipeline);
            self.slice_plane_render_data[i].draw(&mut render_pass);
        }
    }

    /// Renders the ground plane to the stencil buffer for reflection masking.
    ///
    /// This should be called before rendering reflected geometry.
    /// The stencil buffer will have value 1 where the ground plane is visible.
    #[allow(clippy::too_many_arguments)]
    pub fn render_stencil_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        ground_height: f32,
        scene_center: [f32; 3],
        length_scale: f32,
    ) {
        let Some(pipeline) = &self.ground_stencil_pipeline else {
            return;
        };

        // Initialize render data if needed
        if self.ground_plane_render_data.is_none() {
            if let Some(ref shadow_pass) = self.shadow_map_pass {
                self.ground_plane_render_data = Some(GroundPlaneRenderData::new(
                    &self.device,
                    &self.ground_plane_bind_group_layout,
                    &self.camera_buffer,
                    shadow_pass.light_buffer(),
                    shadow_pass.depth_view(),
                    shadow_pass.comparison_sampler(),
                ));
            }
        }

        let Some(render_data) = &self.ground_plane_render_data else {
            return;
        };

        // Check if camera is in orthographic mode
        let is_orthographic =
            self.camera.projection_mode == crate::camera::ProjectionMode::Orthographic;
        let camera_height = self.camera.position.y;

        // Update ground uniforms for stencil pass
        render_data.update(
            &self.queue,
            scene_center,
            scene_center[1] - length_scale * 0.5, // scene_min_y estimate
            length_scale,
            camera_height,
            Some(ground_height),
            0.0, // shadow_darkness (unused in stencil)
            0,   // shadow_mode (unused in stencil)
            is_orthographic,
            0.0, // reflection_intensity (unused in stencil)
        );

        let view = self.hdr_view.as_ref().unwrap_or(color_view);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Stencil Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Don't clear color
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep existing depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0), // Clear stencil to 0
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, render_data.bind_group(), &[]);
        render_pass.set_stencil_reference(1); // Write 1 to stencil
        render_pass.draw(0..12, 0..1); // 4 triangles = 12 vertices
    }

    /// Initializes reflection pass resources.
    pub(crate) fn init_reflection_pass(&mut self) {
        self.reflection_pass = Some(crate::reflection_pass::ReflectionPass::new(&self.device));
    }

    /// Returns the reflection pass.
    pub fn reflection_pass(&self) -> Option<&crate::reflection_pass::ReflectionPass> {
        self.reflection_pass.as_ref()
    }

    /// Updates reflection uniforms.
    pub fn update_reflection(
        &self,
        reflection_matrix: glam::Mat4,
        intensity: f32,
        ground_height: f32,
    ) {
        if let Some(reflection) = &self.reflection_pass {
            reflection.update_uniforms(&self.queue, reflection_matrix, intensity, ground_height);
        }
    }

    /// Creates a bind group for reflected mesh rendering.
    pub fn create_reflected_mesh_bind_group(
        &self,
        mesh_render_data: &crate::surface_mesh_render::SurfaceMeshRenderData,
    ) -> Option<wgpu::BindGroup> {
        let layout = self.reflected_mesh_bind_group_layout.as_ref()?;

        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflected Mesh Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mesh_render_data.uniform_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mesh_render_data.position_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mesh_render_data.normal_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mesh_render_data.barycentric_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mesh_render_data.color_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: mesh_render_data.edge_is_real_buffer().as_entire_binding(),
                },
            ],
        }))
    }

    /// Renders a single reflected mesh.
    ///
    /// Call this for each visible surface mesh after `render_stencil_pass`.
    pub fn render_reflected_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh_bind_group: &wgpu::BindGroup,
        vertex_count: u32,
    ) {
        let Some(pipeline) = &self.reflected_mesh_pipeline else {
            return;
        };
        let Some(reflection) = &self.reflection_pass else {
            return;
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, mesh_bind_group, &[]);
        render_pass.set_bind_group(1, reflection.bind_group(), &[]);
        render_pass.set_stencil_reference(1); // Test against stencil value 1
        render_pass.draw(0..vertex_count, 0..1);
    }

    /// Creates a bind group for reflected point cloud rendering.
    pub fn create_reflected_point_cloud_bind_group(
        &self,
        point_render_data: &crate::point_cloud_render::PointCloudRenderData,
    ) -> Option<wgpu::BindGroup> {
        let layout = self.reflected_point_cloud_bind_group_layout.as_ref()?;

        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflected Point Cloud Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: point_render_data.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: point_render_data.position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: point_render_data.color_buffer.as_entire_binding(),
                },
            ],
        }))
    }

    /// Renders a single reflected point cloud.
    pub fn render_reflected_point_cloud(
        &self,
        render_pass: &mut wgpu::RenderPass,
        point_bind_group: &wgpu::BindGroup,
        point_count: u32,
    ) {
        let Some(pipeline) = &self.reflected_point_cloud_pipeline else {
            return;
        };
        let Some(reflection) = &self.reflection_pass else {
            return;
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, point_bind_group, &[]);
        render_pass.set_bind_group(1, reflection.bind_group(), &[]);
        render_pass.set_stencil_reference(1);
        // 6 vertices per point (billboard quad as 2 triangles)
        render_pass.draw(0..6, 0..point_count);
    }

    /// Creates a bind group for reflected curve network rendering.
    pub fn create_reflected_curve_network_bind_group(
        &self,
        curve_render_data: &crate::curve_network_render::CurveNetworkRenderData,
    ) -> Option<wgpu::BindGroup> {
        let layout = self.reflected_curve_network_bind_group_layout.as_ref()?;

        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflected Curve Network Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: curve_render_data.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: curve_render_data.edge_vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: curve_render_data.edge_color_buffer.as_entire_binding(),
                },
            ],
        }))
    }

    /// Renders a single reflected curve network (tube mode).
    pub fn render_reflected_curve_network(
        &self,
        render_pass: &mut wgpu::RenderPass,
        curve_bind_group: &wgpu::BindGroup,
        curve_render_data: &crate::curve_network_render::CurveNetworkRenderData,
    ) {
        let Some(pipeline) = &self.reflected_curve_network_pipeline else {
            return;
        };
        let Some(reflection) = &self.reflection_pass else {
            return;
        };
        let Some(tube_vertex_buffer) = &curve_render_data.generated_vertex_buffer else {
            return;
        };

        // 36 vertices per edge (tube geometry)
        let tube_vertex_count = curve_render_data.num_edges * 36;

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, curve_bind_group, &[]);
        render_pass.set_bind_group(1, reflection.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, tube_vertex_buffer.slice(..));
        render_pass.set_stencil_reference(1);
        render_pass.draw(0..tube_vertex_count, 0..1);
    }
}
