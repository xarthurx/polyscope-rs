//! GPU picking of the window: pick resources of the structures and the pick pass.
//!
//! Captures and headless renders do not pick, so only the window uses this module.

use super::{CameraView, CurveNetwork, PointCloud, Structure, SurfaceMesh, VolumeGrid, VolumeMesh};
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::HasQuantities;
use polyscope_render::RenderEngine;
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

/// Creates the missing pick resources of all structures and updates the
/// pick uniforms of volume grid quantities.
pub(super) fn init_pick_resources(engine: &mut RenderEngine) {
    crate::with_context_mut(|ctx| {
        for structure in ctx.registry.iter_mut() {
            // PointCloud: windowed-only pick resources
            if structure.type_name() == "PointCloud" {
                let structure_name = structure.name().to_string();
                if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                    // Initialize pick resources (after render data init by shared function)
                    if pc.pick_bind_group().is_none() && pc.render_data().is_some() {
                        let num_points = pc.points().len() as u32;
                        let global_start =
                            engine.assign_pick_range("PointCloud", &structure_name, num_points);
                        pc.init_pick_resources(
                            &engine.device,
                            engine.pick_bind_group_layout(),
                            engine.camera_buffer(),
                            global_start,
                        );
                    }
                }
            }

            // SurfaceMesh: windowed-only pick resources
            if structure.type_name() == "SurfaceMesh"
                && let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>()
            {
                // Initialize pick resources (after render data init by shared function)
                if mesh.pick_bind_group().is_none() && mesh.render_data().is_some() {
                    let num_faces = mesh.num_faces() as u32;
                    let global_start =
                        engine.assign_pick_range("SurfaceMesh", mesh.name(), num_faces);
                    mesh.init_pick_resources(
                        &engine.device,
                        engine.mesh_pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }
            }

            // CurveNetwork: windowed-only pick resources (edge and tube)
            if structure.type_name() == "CurveNetwork"
                && let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>()
            {
                // Initialize pick resources (after render data init by shared function)
                if cn.pick_bind_group().is_none() && cn.render_data().is_some() {
                    // Initialize curve network pick pipeline if not done
                    if !engine.has_curve_network_pick_pipeline() {
                        engine.init_curve_network_pick_pipeline();
                    }
                    let num_edges = cn.num_edges() as u32;
                    let global_start =
                        engine.assign_pick_range("CurveNetwork", cn.name(), num_edges);
                    cn.init_pick_resources(
                        &engine.device,
                        engine.pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }

                // Initialize tube pick resources (for tube render mode)
                // This provides a larger clickable area using ray-cylinder intersection
                if !cn.has_tube_pick_resources() && cn.render_data().is_some() {
                    // Initialize tube pick pipeline if not done
                    if !engine.has_curve_network_tube_pick_pipeline() {
                        engine.init_curve_network_tube_pick_pipeline();
                    }
                    cn.init_tube_pick_resources(
                        &engine.device,
                        engine.curve_network_tube_pick_bind_group_layout(),
                        engine.camera_buffer(),
                    );
                }
            }

            // CameraView init moved to shared function
            // CameraView: windowed-only pick resources (curve network line pick)
            if structure.type_name() == "CameraView"
                && let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>()
                && cv.pick_bind_group().is_none()
                && let Some(num_edges) = cv.render_data().map(|rd| rd.num_edges)
            {
                if !engine.has_curve_network_pick_pipeline() {
                    engine.init_curve_network_pick_pipeline();
                }
                let global_start = engine.assign_pick_range("CameraView", cv.name(), num_edges);
                cv.init_pick_resources(
                    &engine.device,
                    engine.pick_bind_group_layout(),
                    engine.camera_buffer(),
                    global_start,
                );
            }

            // VolumeGrid: pick resources of enabled gridcube quantities
            if structure.type_name() == "VolumeGrid"
                && let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>()
            {
                let cube_size_factor = vg.cube_size_factor();
                let transform = vg.transform();
                // Init gridcube pick pipeline if needed
                if !engine.has_gridcube_pick_pipeline() {
                    // Only init if there are any enabled gridcube quantities
                    let has_enabled_gridcube = vg.quantities().iter().any(|q| {
                        if let Some(nsq) = q.as_any().downcast_ref::<VolumeGridNodeScalarQuantity>()
                        {
                            nsq.is_enabled()
                                && nsq.viz_mode() == VolumeGridVizMode::Gridcube
                                && nsq.gridcube_render_data().is_some()
                        } else if let Some(csq) =
                            q.as_any().downcast_ref::<VolumeGridCellScalarQuantity>()
                        {
                            csq.is_enabled() && csq.gridcube_render_data().is_some()
                        } else {
                            false
                        }
                    });
                    if has_enabled_gridcube {
                        engine.init_gridcube_pick_pipeline();
                    }
                }

                // Assign pick ranges and init pick resources for each enabled quantity
                if engine.has_gridcube_pick_pipeline() {
                    let vg_name = vg.name().to_string();
                    for quantity in vg.quantities_mut() {
                        if !quantity.is_enabled() {
                            continue;
                        }

                        if let Some(nsq) = quantity
                            .as_any_mut()
                            .downcast_mut::<VolumeGridNodeScalarQuantity>()
                        {
                            if nsq.viz_mode() == VolumeGridVizMode::Gridcube
                                && nsq.gridcube_render_data().is_some()
                                && nsq.pick_bind_group().is_none()
                            {
                                let num_elements = nsq.num_pick_elements();
                                let pick_name = format!("{}/{}", vg_name, nsq.name());
                                let global_start = engine.assign_pick_range(
                                    "VolumeGrid",
                                    &pick_name,
                                    num_elements,
                                );
                                nsq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    global_start,
                                );
                            }
                            // Update pick uniforms every frame (model may change)
                            nsq.update_pick_uniforms(
                                &engine.queue,
                                transform.to_cols_array_2d(),
                                cube_size_factor.max(0.5),
                            );
                        }

                        if let Some(csq) = quantity
                            .as_any_mut()
                            .downcast_mut::<VolumeGridCellScalarQuantity>()
                        {
                            if csq.gridcube_render_data().is_some()
                                && csq.pick_bind_group().is_none()
                            {
                                let num_elements = csq.num_pick_elements();
                                let pick_name = format!("{}/{}", vg_name, csq.name());
                                let global_start = engine.assign_pick_range(
                                    "VolumeGrid",
                                    &pick_name,
                                    num_elements,
                                );
                                csq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    global_start,
                                );
                            }
                            // Update pick uniforms every frame
                            csq.update_pick_uniforms(
                                &engine.queue,
                                transform.to_cols_array_2d(),
                                cube_size_factor.max(0.5),
                            );
                        }
                    }
                }
            }

            // VolumeMesh: windowed-only pick resources
            // (slice plane culling logic moved to shared function)
            if structure.type_name() == "VolumeMesh"
                && let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>()
            {
                // Initialize pick resources (after render data init by shared function)
                if vm.pick_bind_group().is_none() && vm.render_data().is_some() {
                    if !engine.has_mesh_pick_pipeline() {
                        engine.init_mesh_pick_pipeline();
                    }
                    let num_cells = vm.num_cells() as u32;
                    let global_start = engine.assign_pick_range("VolumeMesh", vm.name(), num_cells);
                    vm.init_pick_resources(
                        &engine.device,
                        engine.mesh_pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }
            }
        }
    });
}

/// Draws all visible structures into the pick buffer.
pub(super) fn render_pick_pass(engine: &RenderEngine) {
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick pass encoder"),
        });

    if let Some(mut pick_pass) = engine.begin_pick_pass(&mut encoder) {
        // Draw point clouds to pick buffer
        pick_pass.set_pipeline(engine.point_pick_pipeline());

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }
                if structure.type_name() == "PointCloud"
                    && let Some(pc) = structure.as_any().downcast_ref::<PointCloud>()
                    && let (Some(pick_bind_group), Some(render_data)) =
                        (pc.pick_bind_group(), pc.render_data())
                {
                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                    // 6 vertices per quad, num_points instances
                    pick_pass.draw(0..6, 0..render_data.num_points);
                }
            }
        });

        // Draw curve networks to pick buffer
        // Use tube picking (ray-cylinder) for all curve networks for better hit detection
        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }
                if structure.type_name() == "CurveNetwork"
                    && let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>()
                {
                    let Some(render_data) = cn.render_data() else {
                        continue;
                    };

                    // Use tube picking when available - provides larger clickable area
                    if engine.has_curve_network_tube_pick_pipeline() {
                        if let (Some(tube_bind_group), Some(vertex_buffer)) = (
                            cn.tube_pick_bind_group(),
                            render_data.generated_vertex_buffer.as_ref(),
                        ) {
                            // Use tube-based picking (ray-cylinder intersection)
                            pick_pass.set_pipeline(engine.curve_network_tube_pick_pipeline());
                            pick_pass.set_bind_group(0, tube_bind_group, &[]);
                            pick_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            // 36 vertices per edge (bounding box triangles)
                            pick_pass.draw(0..render_data.num_edges * 36, 0..1);
                        }
                    } else if engine.has_curve_network_pick_pipeline() {
                        // Fallback to line-based picking
                        if let Some(pick_bind_group) = cn.pick_bind_group() {
                            pick_pass.set_pipeline(engine.curve_network_pick_pipeline());
                            pick_pass.set_bind_group(0, pick_bind_group, &[]);
                            // 2 vertices per edge (LineList topology)
                            pick_pass.draw(0..render_data.num_edges * 2, 0..1);
                        }
                    }
                }
            }
        });

        // Draw camera view wireframes to pick buffer (C++ camera views are pickable)
        if engine.has_curve_network_pick_pipeline() {
            pick_pass.set_pipeline(engine.curve_network_pick_pipeline());
            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if ctx.is_structure_visible(structure)
                        && let Some(cv) = structure.as_any().downcast_ref::<CameraView>()
                        && let (Some(bind_group), Some(render_data)) =
                            (cv.pick_bind_group(), cv.render_data())
                    {
                        pick_pass.set_bind_group(0, bind_group, &[]);
                        pick_pass.draw(0..render_data.num_edges * 2, 0..1);
                    }
                }
            });
        }

        // Draw surface meshes and volume meshes to pick buffer
        // (both use the same mesh pick pipeline with face/cell index mapping)
        if engine.has_mesh_pick_pipeline() {
            pick_pass.set_pipeline(engine.mesh_pick_pipeline());
            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !ctx.is_structure_visible(structure) {
                        continue;
                    }
                    if structure.type_name() == "SurfaceMesh"
                        && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                        && let Some(pick_bind_group) = mesh.pick_bind_group()
                    {
                        pick_pass.set_bind_group(0, pick_bind_group, &[]);
                        pick_pass.draw(0..mesh.num_triangulation_vertices(), 0..1);
                    }
                    if structure.type_name() == "VolumeMesh"
                        && let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>()
                        && let Some(pick_bind_group) = vm.pick_bind_group()
                    {
                        pick_pass.set_bind_group(0, pick_bind_group, &[]);
                        pick_pass.draw(0..vm.num_render_vertices(), 0..1);
                    }
                }
            });
        }

        // --- VolumeGrid gridcube picking ---
        if engine.has_gridcube_pick_pipeline() {
            pick_pass.set_pipeline(engine.gridcube_pick_pipeline());
            crate::with_context(|ctx| {
                for structure in ctx.registry.iter() {
                    if !ctx.is_structure_visible(structure) {
                        continue;
                    }
                    if structure.type_name() == "VolumeGrid"
                        && let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>()
                    {
                        for quantity in vg.quantities() {
                            if !quantity.is_enabled() {
                                continue;
                            }
                            if let Some(nsq) = quantity
                                .as_any()
                                .downcast_ref::<VolumeGridNodeScalarQuantity>()
                                && nsq.viz_mode() == VolumeGridVizMode::Gridcube
                                && let Some(pick_bg) = nsq.pick_bind_group()
                            {
                                pick_pass.set_bind_group(0, pick_bg, &[]);
                                pick_pass.draw(0..nsq.pick_total_vertices(), 0..1);
                            }
                            if let Some(csq) = quantity
                                .as_any()
                                .downcast_ref::<VolumeGridCellScalarQuantity>()
                                && let Some(pick_bg) = csq.pick_bind_group()
                            {
                                pick_pass.set_bind_group(0, pick_bg, &[]);
                                pick_pass.draw(0..csq.pick_total_vertices(), 0..1);
                            }
                        }
                    }
                }
            });
        }
    }

    engine.queue.submit(std::iter::once(encoder.finish()));
}

#[cfg(test)]
impl super::App {
    /// Runs the passes of a window frame without the surface and the UI:
    /// frame preparation, pick pass and scene passes. It works with a
    /// headless engine.
    pub(crate) fn render_pick_frame(&mut self) {
        self.prepare_frame(true);
        if let Some(engine) = &self.engine {
            render_pick_pass(engine);
        }
        let options = crate::with_context(|ctx| ctx.options.clone());
        self.configure_frame(&options);
        if let Some(engine) = &mut self.engine {
            let mut encoder =
                engine
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("test frame encoder"),
                    });
            super::render_frame::encode_scene(engine, &mut encoder, &options, false);
            engine.queue.submit(std::iter::once(encoder.finish()));
        }
    }
}
