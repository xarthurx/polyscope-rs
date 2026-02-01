use super::{PointCloud, SurfaceMesh, CurveNetwork, CameraView, VolumeGrid, VolumeMesh, Structure};
use polyscope_structures::volume_grid::{VolumeGridNodeScalarQuantity, VolumeGridCellScalarQuantity, VolumeGridVizMode};
use polyscope_core::structure::HasQuantities;
use polyscope_render::RenderEngine;

/// Draw point clouds to a wgpu render pass.
pub(super) fn draw_point_clouds<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.point_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "PointCloud" {
                if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                    if let Some(render_data) = pc.render_data() {
                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        render_pass.set_bind_group(2, engine.matcap_bind_group_for(pc.material()), &[]);
                        // 6 vertices per quad, num_points instances
                        render_pass.draw(0..6, 0..render_data.num_points);
                    }
                }
            }
        }
    });
}

/// Draw vector quantities to a wgpu render pass.
pub(super) fn draw_vector_quantities<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.vector_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
            if structure.type_name() == "PointCloud" {
                if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                    if let Some(vq) = pc.active_vector_quantity() {
                        if let Some(render_data) = vq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            // shaft sides: 8×6=48 + cone sides: 8×3=24 + cone cap: 8×3=24 + shaft cap: 8×3=24 = 120 vertices per arrow
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                }
            }
            if structure.type_name() == "SurfaceMesh" {
                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                    // Draw vertex vector quantity (e.g. vertex normals)
                    if let Some(vq) = mesh.active_vertex_vector_quantity() {
                        if let Some(render_data) = vq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                    // Draw face vector quantity (e.g. face normals)
                    if let Some(vq) = mesh.active_face_vector_quantity() {
                        if let Some(render_data) = vq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                    // Draw vertex intrinsic vector quantity (e.g. tangent field)
                    if let Some(iq) = mesh.active_vertex_intrinsic_vector_quantity() {
                        if let Some(render_data) = iq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                    // Draw face intrinsic vector quantity
                    if let Some(iq) = mesh.active_face_intrinsic_vector_quantity() {
                        if let Some(render_data) = iq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                    // Draw one-form quantity (edge flow arrows)
                    if let Some(oq) = mesh.active_one_form_quantity() {
                        if let Some(render_data) = oq.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            render_pass.draw(0..120, 0..render_data.num_vectors);
                        }
                    }
                }
            }
        }
    });
}

/// Draw curve network edges (line mode), camera views, and volume grid wireframes to a wgpu render pass.
pub(super) fn draw_curve_networks_and_lines<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.curve_network_edge_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    // Only render in line mode (0)
                    if cn.render_mode() == 0 {
                        if let Some(render_data) = cn.render_data() {
                            render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                            // 2 vertices per edge (LineList topology)
                            render_pass.draw(0..render_data.num_edges * 2, 0..1);
                        }
                    }
                }
            }
            if structure.type_name() == "CameraView" {
                if let Some(cv) = structure.as_any().downcast_ref::<CameraView>() {
                    if let Some(render_data) = cv.render_data() {
                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        // 2 vertices per edge (LineList topology)
                        render_pass.draw(0..render_data.num_edges * 2, 0..1);
                    }
                }
            }
            if structure.type_name() == "VolumeGrid" {
                if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                    if let Some(render_data) = vg.render_data() {
                        render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                        // 2 vertices per edge (LineList topology)
                        render_pass.draw(0..render_data.num_edges * 2, 0..1);
                    }
                }
            }
        }
    });
}

/// Draw curve network tubes to a wgpu render pass.
pub(super) fn draw_curve_network_tubes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.curve_network_tube_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    // Only render in tube mode (1)
                    if cn.render_mode() == 1 {
                        if let Some(render_data) = cn.render_data() {
                            if let (Some(tube_bg), Some(gen_vb)) = (
                                &render_data.tube_render_bind_group,
                                &render_data.generated_vertex_buffer,
                            ) {
                                render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                                render_pass.set_bind_group(0, tube_bg, &[]);
                                render_pass.set_vertex_buffer(0, gen_vb.slice(..));
                                // 36 vertices per edge (12 triangles for bounding box)
                                render_pass.draw(0..render_data.num_edges * 36, 0..1);
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Draw curve network node spheres (tube mode - fills gaps at joints) to a wgpu render pass.
pub(super) fn draw_curve_network_nodes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.point_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    // Only render node spheres in tube mode (1)
                    if cn.render_mode() == 1 {
                        if let Some(render_data) = cn.render_data() {
                            if let Some(node_bg) = &render_data.node_render_bind_group {
                                render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                                render_pass.set_bind_group(0, node_bg, &[]);
                                // 6 vertices per quad, num_nodes instances
                                render_pass.draw(0..6, 0..render_data.num_nodes);
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Draw surface meshes and volume meshes (simple/none transparency mode) to a wgpu render pass.
pub(super) fn draw_meshes_simple<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.mesh_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "SurfaceMesh" {
                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                    if let Some(render_data) = mesh.render_data() {
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
                        render_pass.draw_indexed(
                            0..render_data.num_indices,
                            0,
                            0..1,
                        );
                    }
                }
            }
            if structure.type_name() == "VolumeMesh" {
                if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                    // Render exterior faces (includes cell culling when slice plane is active)
                    if let Some(render_data) = vm.render_data() {
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
                        render_pass.draw_indexed(
                            0..render_data.num_indices,
                            0,
                            0..1,
                        );
                    }
                    // Note: No slice cap geometry needed - we use cell culling
                    // which shows whole cells instead of cross-section caps
                }
            }
        }
    });
}

/// Draw volume grid isosurfaces to a wgpu render pass.
pub(super) fn draw_volume_grid_isosurfaces<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.simple_mesh_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) || structure.type_name() != "VolumeGrid" {
                continue;
            }
            if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                for quantity in vg.quantities() {
                    if !quantity.is_enabled() {
                        continue;
                    }
                    if let Some(nsq) = quantity.as_any().downcast_ref::<VolumeGridNodeScalarQuantity>() {
                        if nsq.viz_mode() == VolumeGridVizMode::Isosurface {
                            if let Some(rd) = nsq.isosurface_render_data() {
                                render_pass.set_bind_group(0, &rd.bind_group, &[]);
                                render_pass.draw(0..rd.num_vertices, 0..1);
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Draw volume grid gridcubes to a wgpu render pass.
pub(super) fn draw_volume_grid_gridcubes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    let Some(pipeline) = &engine.gridcube_pipeline else { return };
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(1, &engine.slice_plane_bind_group, &[]);

    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) || structure.type_name() != "VolumeGrid" {
                continue;
            }
            if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                for quantity in vg.quantities() {
                    if !quantity.is_enabled() {
                        continue;
                    }
                    if let Some(nsq) = quantity.as_any().downcast_ref::<VolumeGridNodeScalarQuantity>() {
                        if nsq.viz_mode() == VolumeGridVizMode::Gridcube {
                            if let Some(rd) = nsq.gridcube_render_data() {
                                render_pass.set_bind_group(0, &rd.bind_group, &[]);
                                // 36 vertices per cube instance
                                render_pass.draw(0..rd.num_instances * 36, 0..1);
                            }
                        }
                    }
                    if let Some(csq) = quantity.as_any().downcast_ref::<VolumeGridCellScalarQuantity>() {
                        if let Some(rd) = csq.gridcube_render_data() {
                            render_pass.set_bind_group(0, &rd.bind_group, &[]);
                            render_pass.draw(0..rd.num_instances * 36, 0..1);
                        }
                    }
                }
            }
        }
    });
}
