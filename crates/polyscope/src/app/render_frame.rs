//! The frame pipeline: frame preparation and the scene passes.
//!
//! The window, windowed screenshots and headless rendering all use this code.
//! The scene passes render into the engine HDR texture. The window then tone
//! maps that texture to the surface and draws the UI on top. A capture tone
//! maps it into the screenshot target.

use super::{
    App, CurveNetwork, GroundPlaneMode, Options, PointCloud, SurfaceMesh, VolumeMesh, render_init,
    render_peel, render_pick, render_scene, render_volume_grid,
};
use polyscope_render::RenderEngine;

impl App {
    /// Prepares the camera, the uniforms and the GPU data of all structures
    /// for a frame.
    ///
    /// An interactive frame (the window) also advances a camera flight and
    /// keeps the pick resources up to date. Captures do neither.
    pub(super) fn prepare_frame(&mut self, interactive: bool) {
        let Some(engine) = &mut self.engine else {
            return;
        };

        // Auto-fit the camera to the scene on the first frame with structures
        self.camera_fitted = render_init::auto_fit_camera(engine, self.camera_fitted);

        // Update the camera flight before the uniforms use the camera
        if interactive {
            engine.camera.update_flight();
        }

        // Fall back to the home view if the view matrix is not valid
        render_init::ensure_view_valid(engine);

        render_init::drain_material_queue(engine);
        render_init::update_uniforms(engine);
        render_init::init_structure_gpu_resources(engine);
        let isosurface_meshes =
            render_volume_grid::init_volume_grid_quantities(engine, interactive);
        // New pick resources exist before `update_gpu_buffers` writes the
        // pick uniforms with the render uniforms. The pick pass gives the
        // pick ranges after the scene passes.
        if interactive {
            render_pick::init_pick_resources(engine);
        }
        render_init::update_gpu_buffers(engine, interactive);

        // Registration takes the context lock, so it comes after the GPU updates.
        render_volume_grid::register_isosurface_meshes(isosurface_meshes);
    }

    /// Applies the SSAA factor of the options and the tone mapping values.
    pub(super) fn configure_frame(&mut self, options: &Options) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        engine.set_ssaa_factor(options.ssaa_factor);
        // SSAO is off with SSAA, because the tone mapping pass then reads the
        // downsampled image and not the SSAO texture.
        let tone = &self.tone_mapping_settings;
        engine.update_tone_mapping(
            tone.exposure,
            tone.white_level,
            tone.gamma,
            super::ssao_active(options, engine),
        );
    }
}

/// Encodes the scene passes of a frame into the engine HDR texture.
///
/// The passes are: curve network tube geometry, shadow map, slice planes,
/// ground plane (with stencil and reflections), depth prepass, main pass,
/// mesh pass, depth peeling and SSAO. Tone mapping comes after this function.
///
/// With `transparent_background`, the background gets alpha 0 and the scene
/// keeps its alpha. The ground plane is part of the scene, as in C++
/// Polyscope: it stays in the image with the alpha of its shader.
pub(super) fn encode_scene(
    engine: &mut RenderEngine,
    encoder: &mut wgpu::CommandEncoder,
    options: &Options,
    transparent_background: bool,
) {
    let ground_plane = &options.ground_plane;

    // Store ground plane settings for later use
    let gp_enabled = ground_plane.mode != GroundPlaneMode::None;
    let gp_height_override = if ground_plane.height_is_relative {
        None
    } else {
        Some(ground_plane.height)
    };
    // Shadow mode: 0=none (disabled), 1=shadow_only, 2=tile_with_shadow
    let gp_shadow_mode = match ground_plane.mode {
        GroundPlaneMode::None => 0u32,
        GroundPlaneMode::ShadowOnly => 1u32,
        // TileReflection also uses tile mode with shadows
        GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
    };
    // The ground plane, its shadows and its reflection follow the up direction
    let ground_frame = polyscope_render::GroundPlaneFrame::new(engine.camera.up_direction);

    // Compute pass for curve network tubes
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Curve Network Tube Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(engine.curve_network_tube_compute_pipeline());

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }
                if structure.type_name() == "CurveNetwork"
                    && let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>()
                    && let Some(render_data) = cn.render_data()
                    && let Some(compute_bg) = &render_data.compute_bind_group
                {
                    compute_pass.set_bind_group(0, compute_bg, &[]);
                    let num_workgroups = render_data.num_edges.div_ceil(64);
                    compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                }
            }
        });
    }

    // Shadow pass - render scene objects from light's perspective to shadow map
    if let (Some(shadow_pipeline), Some(shadow_map_pass)) =
        (engine.shadow_pipeline(), engine.shadow_map_pass())
    {
        // Compute light matrix from scene bounds
        let (scene_center, scene_radius) =
            crate::with_context(|ctx| (ctx.center(), ctx.length_scale * 5.0));
        let light_dir = ground_frame.light_direction();
        let light_matrix = polyscope_render::ShadowMapPass::compute_light_matrix(
            scene_center,
            scene_radius,
            light_dir,
        );

        // Update light uniforms
        shadow_map_pass.update_light(&engine.queue, light_matrix, light_dir);

        // Begin shadow pass
        {
            let mut shadow_pass = shadow_map_pass.begin_shadow_pass(encoder);
            shadow_pass.set_pipeline(shadow_pipeline);

            // Render shadow-casting structures (SurfaceMesh only for now)
            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !ctx.is_structure_visible(structure) {
                        continue;
                    }
                    if structure.type_name() == "SurfaceMesh"
                        && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                        && let Some(shadow_bg) = mesh.shadow_bind_group()
                    {
                        shadow_pass.set_bind_group(0, shadow_bg, &[]);
                        if let Some(rd) = mesh.render_data() {
                            shadow_pass.draw(0..rd.num_vertices(), 0..1);
                        }
                    }
                }
            });
        }
    }

    // Render slice plane visualizations FIRST (before scene geometry)
    // This allows scene geometry to properly occlude the slice planes
    let (slice_planes, length_scale_for_planes) = crate::with_context(|ctx| {
        (
            ctx.slice_planes().cloned().collect::<Vec<_>>(),
            ctx.length_scale,
        )
    });
    // A transparent background has alpha 0. Scene pixels keep their alpha.
    let background = options.background_color;
    let background_alpha = if transparent_background { 0.0 } else { 1.0 };
    engine.render_slice_planes_with_clear(
        encoder,
        &slice_planes,
        length_scale_for_planes,
        [background.x, background.y, background.z, background_alpha],
    );

    let use_depth_peel = options.transparency_mode == polyscope_core::TransparencyMode::Pretty;

    // Render ground plane BEFORE surface mesh passes so transparent objects
    // composite correctly over the ground. Without this, either:
    // - Simple mode: meshes with no depth write get overwritten by later ground plane
    // - Pretty mode: mesh depth prepass blocks ground from rendering, making peeled
    //   transparent meshes appear gray (no ground behind them to show through)
    let (scene_center, scene_bounds, length_scale) = crate::with_context(|ctx| {
        let center = ctx.center();
        (
            [center.x, center.y, center.z],
            ctx.bounding_box,
            ctx.length_scale,
        )
    });

    if ground_plane.mode == GroundPlaneMode::TileReflection {
        // Compute ground height (a coordinate on the up axis)
        let ground_height =
            ground_frame.ground_height(scene_bounds, length_scale, gp_height_override);

        // Update reflection uniforms
        let reflection_matrix = ground_frame.reflection_matrix(ground_height);
        engine.update_reflection(
            reflection_matrix,
            ground_plane.reflection_intensity,
            ground_height,
        );

        // 1. Render stencil pass (mark ground plane region)
        engine.render_stencil_pass(encoder, ground_height, scene_center, length_scale);

        // 2. Render ground plane FIRST (opaque base)
        engine.render_ground_plane(
            encoder,
            true, // enabled
            scene_center,
            scene_bounds,
            length_scale,
            gp_height_override,
            ground_plane.shadow_darkness,
            gp_shadow_mode,
            0.0, // No transparency - fully opaque ground
        );

        // 3. Render reflected meshes ON TOP of ground
        {
            let hdr_view = engine.hdr_view().expect("HDR view should be available");
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Reflected Geometry Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Keep stencil from previous pass
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                ..Default::default()
            });

            // Render each visible structure reflected
            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !ctx.is_structure_visible(structure) {
                        continue;
                    }
                    // SurfaceMesh
                    if structure.type_name() == "SurfaceMesh"
                        && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                        && let Some(mesh_data) = mesh.render_data()
                        && let Some(bind_group) = engine.create_reflected_mesh_bind_group(mesh_data)
                    {
                        engine.render_reflected_mesh(
                            &mut render_pass,
                            &bind_group,
                            mesh_data.vertex_count(),
                            structure.material(),
                        );
                    }
                    // VolumeMesh (uses SurfaceMeshRenderData)
                    if structure.type_name() == "VolumeMesh"
                        && let Some(vol_mesh) = structure.as_any().downcast_ref::<VolumeMesh>()
                        && let Some(mesh_data) = vol_mesh.render_data()
                        && let Some(bind_group) = engine.create_reflected_mesh_bind_group(mesh_data)
                    {
                        engine.render_reflected_mesh(
                            &mut render_pass,
                            &bind_group,
                            mesh_data.vertex_count(),
                            structure.material(),
                        );
                    }
                    // PointCloud
                    if structure.type_name() == "PointCloud"
                        && let Some(pc) = structure.as_any().downcast_ref::<PointCloud>()
                        && let Some(pc_data) = pc.render_data()
                        && let Some(bind_group) =
                            engine.create_reflected_point_cloud_bind_group(pc_data)
                    {
                        engine.render_reflected_point_cloud(
                            &mut render_pass,
                            &bind_group,
                            pc_data.num_points,
                            structure.material(),
                        );
                    }
                    // CurveNetwork
                    if structure.type_name() == "CurveNetwork"
                        && let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>()
                        && let Some(cn_data) = cn.render_data()
                        && let Some(bind_group) =
                            engine.create_reflected_curve_network_bind_group(cn_data)
                    {
                        engine.render_reflected_curve_network(
                            &mut render_pass,
                            &bind_group,
                            cn_data,
                            structure.material(),
                        );
                    }
                }
            });
        }
    } else {
        // Non-reflection ground plane modes
        engine.render_ground_plane(
            encoder,
            gp_enabled,
            scene_center,
            scene_bounds,
            length_scale,
            gp_height_override,
            ground_plane.shadow_darkness,
            gp_shadow_mode,
            0.0,
        );
    }

    // Surface mesh depth prepass for Pretty mode (opaque meshes only)
    if use_depth_peel {
        render_peel::render_mesh_depth_prepass(encoder, engine);
    }

    // Main render pass - always render scene to HDR texture
    // Get fresh reference to hdr_view after slice plane rendering
    let hdr_view = engine
        .hdr_view()
        .expect("HDR texture should always be available");
    {
        // All scene content renders to HDR texture for consistent format
        let scene_view = hdr_view;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: scene_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Load slice plane content
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &engine.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Load slice plane depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        // Draw point clouds
        render_scene::draw_point_clouds(&mut render_pass, engine);

        // Draw vector quantities
        render_scene::draw_vector_quantities(&mut render_pass, engine);

        // Note: Surface meshes and volume meshes are rendered in a separate pass
        // with MRT (multiple render targets) for SSAO normal output

        // Draw curve network edges (line mode), camera views, and volume grids
        render_scene::draw_curve_networks_and_lines(&mut render_pass, engine);

        // Draw curve network tubes (tube mode)
        render_scene::draw_curve_network_tubes(&mut render_pass, engine);

        // Draw curve network node spheres (tube mode - fills gaps at joints)
        render_scene::draw_curve_network_nodes(&mut render_pass, engine);
    } // End of main render pass scope

    // Surface mesh render pass with MRT (HDR color + normal G-buffer for SSAO)
    if engine.mesh_pipeline.is_some() {
        let hdr_view = engine.hdr_view().expect("HDR view should be available");
        let normal_view = engine
            .normal_view()
            .expect("Normal view should be available");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Surface Mesh Pass"),
            color_attachments: &[
                // Color output (HDR)
                Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                // Normal output (G-buffer for SSAO)
                // Alpha=0 marks "no geometry" so SSAO skips ground plane/background
                Some(wgpu::RenderPassColorAttachment {
                    view: normal_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.5,
                            b: 1.0,
                            a: 0.0, // a=0 means no valid geometry
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &engine.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve depth from main pass
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

        if use_depth_peel {
            // Surface meshes: depth/normal only (color handled by depth peeling).
            // Volume meshes: full color/normal pass (not peeled).
            render_peel::draw_meshes_for_depth_peel(&mut render_pass, engine);
        } else {
            // Simple/None mode: render all surface meshes through the normal
            // pipeline (alpha blending with depth write). The ground plane has
            // already been rendered before this pass, so alpha blending correctly
            // composites transparent meshes over the ground. Depth write ensures
            // proper occlusion between meshes and prevents later passes from
            // overwriting mesh pixels. When alpha=1.0, this produces the same
            // visual result as fully opaque rendering.
            render_scene::draw_meshes_simple(&mut render_pass, engine);
        }

        // Draw volume grid isosurfaces (simple mesh pipeline, same MRT pass)
        render_scene::draw_volume_grid_isosurfaces(&mut render_pass, engine);

        // Draw volume grid gridcubes (gridcube pipeline, same MRT pass)
        render_scene::draw_volume_grid_gridcubes(&mut render_pass, engine);
    }

    // Note: Ground plane is rendered earlier (before depth prepass and surface mesh pass)
    // so that transparent meshes can correctly composite over it.

    // Depth peeling transparency pass for surface meshes
    // All surface meshes go through depth peeling to handle overlapping geometry correctly
    if use_depth_peel {
        render_peel::render_depth_peel(encoder, engine, options.transparency_render_passes);
    }

    // Render SSAO if enabled
    if super::ssao_active(options, engine) {
        engine.render_ssao(encoder, &options.ssao);
    }
}
