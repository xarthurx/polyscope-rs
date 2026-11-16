//! Depth-peeling transparency (`TransparencyMode::Pretty`) of the scene passes.

use super::{SurfaceMesh, VolumeMesh};
use polyscope_render::RenderEngine;

/// Writes the depth and normals of surface meshes before the main pass, so
/// that later geometry is depth-tested against them.
pub(super) fn render_mesh_depth_prepass(encoder: &mut wgpu::CommandEncoder, engine: &RenderEngine) {
    let (Some(depth_pipeline), Some(color_view)) = (
        engine.mesh_depth_normal_pipeline.as_ref(),
        engine.hdr_view(),
    ) else {
        return;
    };
    let normal_view = engine
        .normal_view()
        .expect("Normal view should be available");

    let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Surface Mesh Depth Prepass"),
        color_attachments: &[
            Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            }),
        ],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &engine.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });

    prepass.set_pipeline(depth_pipeline);
    prepass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "SurfaceMesh"
                && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                && let Some(render_data) = mesh.render_data()
            {
                prepass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                prepass.set_bind_group(0, &render_data.bind_group, &[]);
                prepass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                prepass.draw_indexed(0..render_data.num_indices, 0, 0..1);
            }
        }
    });
}

/// Draws meshes in the MRT mesh pass of depth-peeling mode.
///
/// Surface meshes write depth and normals only, because the peel passes add
/// their color. Volume meshes draw in full (they are not peeled).
pub(super) fn draw_meshes_for_depth_peel<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(mesh_pipeline) = &engine.mesh_pipeline else {
        return;
    };
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);
    render_pass.set_pipeline(
        engine
            .mesh_depth_normal_pipeline
            .as_ref()
            .unwrap_or(mesh_pipeline),
    );

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "SurfaceMesh"
                && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                && let Some(render_data) = mesh.render_data()
            {
                render_pass.set_bind_group(
                    2,
                    engine.matcap_bind_group_for(structure.material()),
                    &[],
                );
                render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                render_pass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
            }
        }
    });

    render_pass.set_pipeline(mesh_pipeline);
    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "VolumeMesh"
                && let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>()
                && let Some(render_data) = vm.render_data()
            {
                // Exterior faces (includes cell culling when a slice plane is active)
                render_pass.set_bind_group(
                    2,
                    engine.matcap_bind_group_for(structure.material()),
                    &[],
                );
                render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                render_pass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
            }
        }
    });
}

/// Peels the surface meshes layer by layer and composites the result onto
/// the engine HDR texture.
pub(super) fn render_depth_peel(
    encoder: &mut wgpu::CommandEncoder,
    engine: &mut RenderEngine,
    num_passes: u32,
) {
    let has_surface_meshes = crate::with_context(|ctx| {
        ctx.registry
            .iter()
            .any(|s| s.is_enabled() && s.type_name() == "SurfaceMesh")
    });
    if !has_surface_meshes {
        return;
    }

    engine.ensure_depth_peel_pass();
    let engine: &RenderEngine = engine;
    let peel = engine.depth_peel_pass().unwrap();

    // Clear final buffer to transparent black
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Peel: clear final"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: peel.final_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });

    // Clear min-depth buffer to 0.0 (no depth peeled yet)
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Peel: clear min-depth"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: peel.min_depth_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });

    for _i_pass in 0..num_passes {
        // Peel pass: render all surface meshes, discarding already-peeled fragments
        {
            let mut peel_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Peel: geometry pass"),
                color_attachments: &[
                    // Color output (premultiplied alpha)
                    Some(wgpu::RenderPassColorAttachment {
                        view: peel.peel_color_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    // Depth-as-color output
                    Some(wgpu::RenderPassColorAttachment {
                        view: peel.peel_depth_color_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: peel.peel_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            peel_pass.set_pipeline(peel.peel_pipeline());
            peel_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);
            peel_pass.set_bind_group(3, peel.peel_bind_group(), &[]);

            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !ctx.is_structure_visible(structure) {
                        continue;
                    }
                    if structure.type_name() == "SurfaceMesh"
                        && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                        && let Some(render_data) = mesh.render_data()
                    {
                        peel_pass.set_bind_group(
                            2,
                            engine.matcap_bind_group_for(structure.material()),
                            &[],
                        );
                        peel_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        peel_pass.set_index_buffer(
                            render_data.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        peel_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                    }
                }
            });
        }

        // Composite this peel layer into the final buffer (alpha-under)
        peel.composite_layer(encoder, &engine.device);

        // Update min-depth from this peel's depth output (Max blend)
        peel.update_min_depth(encoder, &engine.device);
    }

    // Composite final peeled result onto the scene
    let target = engine.hdr_view().expect("HDR view should be available");
    peel.composite_final_to_scene(encoder, &engine.device, target);
}
