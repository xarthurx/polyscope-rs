//! GPU picking of the window: pick resources of the structures and the pick pass.
//!
//! Captures and headless renders do not pick, so only the window uses this module.

use std::collections::HashSet;

use super::{CameraView, CurveNetwork, PointCloud, Structure, SurfaceMesh, VolumeGrid, VolumeMesh};
use polyscope_core::Context;
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::HasQuantities;
use polyscope_render::RenderEngine;
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

/// Renders the pick pass of a window frame.
///
/// One lock of the context covers the whole pass. It creates the missing pick
/// resources, gives the pick ranges of the frame, writes the pick uniforms and
/// draws. Thus another thread cannot change the registry between these steps,
/// and the pick buffer always matches the pick ranges of the frame.
pub(super) fn render_pick_pass(engine: &mut RenderEngine) {
    crate::with_context_mut(|ctx| {
        prepare_pick_owners(engine, ctx);
        draw_pick_owners(engine, ctx);
    });
}

/// Creates the missing pick resources of the visible structures, gives them
/// the pick ranges of the frame and writes their pick uniforms.
///
/// Each frame gives all ranges again, one after the other, from the current
/// element counts. Thus a count change, a removed structure or an empty
/// structure leaves no unused indices, and two ranges cannot overlap. A hidden
/// structure gets no range, so it cannot use up the indices of visible
/// structures.
fn prepare_pick_owners(engine: &mut RenderEngine, ctx: &mut Context) {
    engine.clear_pick_ranges();
    let length_scale = ctx.length_scale;
    let visible: HashSet<(&'static str, String)> = ctx
        .registry
        .iter()
        .filter(|structure| ctx.is_structure_visible(*structure))
        .map(|structure| (structure.type_name(), structure.name().to_string()))
        .collect();

    for structure in ctx.registry.iter_mut() {
        if !visible.contains(&(structure.type_name(), structure.name().to_string())) {
            continue;
        }

        // PointCloud: the pick pass draws it with render data
        if structure.type_name() == "PointCloud" {
            let structure_name = structure.name().to_string();
            if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>()
                && pc.render_data().is_some()
            {
                let num_points = pc.points().len() as u32;
                let global_start =
                    engine.assign_pick_range("PointCloud", &structure_name, num_points);
                if pc.pick_bind_group().is_some() {
                    pc.set_pick_global_start(global_start);
                } else {
                    pc.init_pick_resources(
                        &engine.device,
                        engine.pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }
                pc.update_pick_uniforms(&engine.queue);
            }
        }

        // SurfaceMesh: the pick pass draws it with pick resources
        if structure.type_name() == "SurfaceMesh"
            && let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>()
        {
            let has_pick = mesh.pick_bind_group().is_some();
            if has_pick || mesh.render_data().is_some() {
                let num_faces = mesh.num_faces() as u32;
                let global_start = engine.assign_pick_range("SurfaceMesh", mesh.name(), num_faces);
                if has_pick {
                    mesh.set_pick_global_start(global_start);
                } else {
                    mesh.init_pick_resources(
                        &engine.device,
                        engine.mesh_pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }
                mesh.update_pick_uniforms(&engine.queue);
            }
        }

        // CurveNetwork: the pick pass draws it with render data (edge and tube)
        if structure.type_name() == "CurveNetwork"
            && let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>()
            && cn.render_data().is_some()
        {
            let num_edges = cn.num_edges() as u32;
            let global_start = engine.assign_pick_range("CurveNetwork", cn.name(), num_edges);
            if cn.pick_bind_group().is_some() {
                cn.set_pick_global_start(global_start);
            } else {
                // Initialize curve network pick pipeline if not done
                if !engine.has_curve_network_pick_pipeline() {
                    engine.init_curve_network_pick_pipeline();
                }
                cn.init_pick_resources(
                    &engine.device,
                    engine.pick_bind_group_layout(),
                    engine.camera_buffer(),
                    global_start,
                );
            }

            // Initialize tube pick resources (for tube render mode)
            // This provides a larger clickable area using ray-cylinder intersection
            if !cn.has_tube_pick_resources() {
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
            // The line and tube pick radii follow the current radius.
            cn.update_pick_uniforms(&engine.queue, length_scale);
        }

        // CameraView: the curve network line pick draws its wireframe edges
        if structure.type_name() == "CameraView"
            && let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>()
            && let Some(num_edges) = cv.render_data().map(|rd| rd.num_edges)
        {
            let global_start = engine.assign_pick_range("CameraView", cv.name(), num_edges);
            if cv.pick_bind_group().is_some() {
                cv.set_pick_global_start(global_start);
                cv.update_pick_uniforms(&engine.queue);
            } else {
                if !engine.has_curve_network_pick_pipeline() {
                    engine.init_curve_network_pick_pipeline();
                }
                cv.init_pick_resources(
                    &engine.device,
                    engine.pick_bind_group_layout(),
                    engine.camera_buffer(),
                    global_start,
                );
            }
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
                    if let Some(nsq) = q.as_any().downcast_ref::<VolumeGridNodeScalarQuantity>() {
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

            // Give pick ranges and init pick resources for each enabled
            // quantity. The range belongs to the index of the quantity.
            if engine.has_gridcube_pick_pipeline() {
                let vg_name = vg.name().to_string();
                for (index, quantity) in vg.quantities_mut().iter_mut().enumerate() {
                    if !quantity.is_enabled() {
                        continue;
                    }

                    if let Some(nsq) = quantity
                        .as_any_mut()
                        .downcast_mut::<VolumeGridNodeScalarQuantity>()
                    {
                        // The pick pass draws it in gridcube mode with pick resources.
                        let has_pick = nsq.pick_bind_group().is_some();
                        if nsq.viz_mode() == VolumeGridVizMode::Gridcube
                            && (has_pick || nsq.gridcube_render_data().is_some())
                        {
                            let global_start = engine.assign_quantity_pick_range(
                                "VolumeGrid",
                                &vg_name,
                                index,
                                nsq.num_pick_elements(),
                            );
                            if has_pick {
                                nsq.set_pick_global_start(global_start);
                            } else {
                                nsq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    global_start,
                                );
                            }
                            // The model and the cube size can change in each frame.
                            nsq.update_pick_uniforms(
                                &engine.queue,
                                transform.to_cols_array_2d(),
                                cube_size_factor,
                            );
                        }
                    }

                    if let Some(csq) = quantity
                        .as_any_mut()
                        .downcast_mut::<VolumeGridCellScalarQuantity>()
                    {
                        // The pick pass draws it with pick resources.
                        let has_pick = csq.pick_bind_group().is_some();
                        if has_pick || csq.gridcube_render_data().is_some() {
                            let global_start = engine.assign_quantity_pick_range(
                                "VolumeGrid",
                                &vg_name,
                                index,
                                csq.num_pick_elements(),
                            );
                            if has_pick {
                                csq.set_pick_global_start(global_start);
                            } else {
                                csq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    global_start,
                                );
                            }
                            // The model and the cube size can change in each frame.
                            csq.update_pick_uniforms(
                                &engine.queue,
                                transform.to_cols_array_2d(),
                                cube_size_factor,
                            );
                        }
                    }
                }
            }
        }

        // VolumeMesh: the pick pass draws it with pick resources
        if structure.type_name() == "VolumeMesh"
            && let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>()
        {
            let has_pick = vm.pick_bind_group().is_some();
            if has_pick || vm.render_data().is_some() {
                let num_cells = vm.num_cells() as u32;
                let global_start = engine.assign_pick_range("VolumeMesh", vm.name(), num_cells);
                if has_pick {
                    vm.set_pick_global_start(global_start);
                } else {
                    if !engine.has_mesh_pick_pipeline() {
                        engine.init_mesh_pick_pipeline();
                    }
                    vm.init_pick_resources(
                        &engine.device,
                        engine.mesh_pick_bind_group_layout(),
                        engine.camera_buffer(),
                        global_start,
                    );
                }
                vm.update_pick_uniforms(&engine.queue);
            }
        }
    }
}

/// Returns whether an owner draws with the pick range of this frame.
///
/// `prepare_pick_owners` gives a range to each owner that the pick pass
/// draws, under the same lock. The check keeps the pass from drawing an owner
/// with an old first index if the two functions ever disagree.
fn has_frame_range(engine: &RenderEngine, type_name: &str, name: &str, start: u32) -> bool {
    engine.get_pick_range_start(type_name, name) == Some(start)
}

/// Draws the visible structures with pick resources into the pick buffer.
fn draw_pick_owners(engine: &RenderEngine, ctx: &Context) {
    let mut encoder = engine
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pick pass encoder"),
        });

    if let Some(mut pick_pass) = engine.begin_pick_pass(&mut encoder) {
        // Draw point clouds to pick buffer
        pick_pass.set_pipeline(engine.point_pick_pipeline());
        for structure in ctx.registry.iter() {
            if !ctx.is_structure_visible(structure) {
                continue;
            }
            if structure.type_name() == "PointCloud"
                && let Some(pc) = structure.as_any().downcast_ref::<PointCloud>()
                && let (Some(pick_bind_group), Some(render_data)) =
                    (pc.pick_bind_group(), pc.render_data())
                && has_frame_range(engine, "PointCloud", pc.name(), pc.pick_global_start())
            {
                pick_pass.set_bind_group(0, pick_bind_group, &[]);
                // 6 vertices per quad, num_points instances
                pick_pass.draw(0..6, 0..render_data.num_points);
            }
        }

        // Draw curve networks to pick buffer
        // Use tube picking (ray-cylinder) for all curve networks for better hit detection
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
                if !has_frame_range(engine, "CurveNetwork", cn.name(), cn.pick_global_start()) {
                    continue;
                }

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

        // Draw camera view wireframes to pick buffer (C++ camera views are pickable)
        if engine.has_curve_network_pick_pipeline() {
            pick_pass.set_pipeline(engine.curve_network_pick_pipeline());
            for structure in ctx.registry.iter() {
                if ctx.is_structure_visible(structure)
                    && let Some(cv) = structure.as_any().downcast_ref::<CameraView>()
                    && let (Some(bind_group), Some(render_data)) =
                        (cv.pick_bind_group(), cv.render_data())
                    && has_frame_range(engine, "CameraView", cv.name(), cv.pick_global_start())
                {
                    pick_pass.set_bind_group(0, bind_group, &[]);
                    pick_pass.draw(0..render_data.num_edges * 2, 0..1);
                }
            }
        }

        // Draw surface meshes and volume meshes to pick buffer
        // (both use the same mesh pick pipeline with face/cell index mapping)
        if engine.has_mesh_pick_pipeline() {
            pick_pass.set_pipeline(engine.mesh_pick_pipeline());
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }
                if structure.type_name() == "SurfaceMesh"
                    && let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                    && let Some(pick_bind_group) = mesh.pick_bind_group()
                    && has_frame_range(engine, "SurfaceMesh", mesh.name(), mesh.pick_global_start())
                {
                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                    pick_pass.draw(0..mesh.num_triangulation_vertices(), 0..1);
                }
                if structure.type_name() == "VolumeMesh"
                    && let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>()
                    && let Some(pick_bind_group) = vm.pick_bind_group()
                    && has_frame_range(engine, "VolumeMesh", vm.name(), vm.pick_global_start())
                {
                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                    pick_pass.draw(0..vm.num_render_vertices(), 0..1);
                }
            }
        }

        // --- VolumeGrid gridcube picking ---
        if engine.has_gridcube_pick_pipeline() {
            pick_pass.set_pipeline(engine.gridcube_pick_pipeline());
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }
                if structure.type_name() == "VolumeGrid"
                    && let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>()
                {
                    // A quantity draws only with the pick range of this frame.
                    let frame_start = |index: usize| {
                        engine.get_quantity_pick_range_start("VolumeGrid", vg.name(), index)
                    };
                    for (index, quantity) in vg.quantities().iter().enumerate() {
                        if !quantity.is_enabled() {
                            continue;
                        }
                        if let Some(nsq) = quantity
                            .as_any()
                            .downcast_ref::<VolumeGridNodeScalarQuantity>()
                            && nsq.viz_mode() == VolumeGridVizMode::Gridcube
                            && let Some(pick_bg) = nsq.pick_bind_group()
                            && frame_start(index) == Some(nsq.pick_global_start())
                        {
                            pick_pass.set_bind_group(0, pick_bg, &[]);
                            pick_pass.draw(0..nsq.pick_total_vertices(), 0..1);
                        }
                        if let Some(csq) = quantity
                            .as_any()
                            .downcast_ref::<VolumeGridCellScalarQuantity>()
                            && let Some(pick_bg) = csq.pick_bind_group()
                            && frame_start(index) == Some(csq.pick_global_start())
                        {
                            pick_pass.set_bind_group(0, pick_bg, &[]);
                            pick_pass.draw(0..csq.pick_total_vertices(), 0..1);
                        }
                    }
                }
            }
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
        self.render_pick_frame_with(|| {});
    }

    /// Runs `between` after the frame preparation and before the pick pass,
    /// as another thread can, and then the rest of the frame as
    /// `render_pick_frame` does.
    pub(crate) fn render_pick_frame_with(&mut self, between: impl FnOnce()) {
        self.prepare_frame(true);
        between();
        if let Some(engine) = &mut self.engine {
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
