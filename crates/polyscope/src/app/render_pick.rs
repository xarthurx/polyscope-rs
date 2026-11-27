//! GPU picking of the window: pick resources of the structures and the pick pass.
//!
//! Captures and headless renders do not pick, so only the window uses this module.
//!
//! The frame preparation creates the pick resources. It writes the pick
//! uniforms (model, radius and cube size) in the same lock and with the same
//! values as the render uniforms. The pick pass runs after the scene passes.
//! It gives the pick ranges of the frame and writes only the first pick index
//! of each owner, so it reads no transform. Thus the pick buffer shows each
//! structure where the window shows it, also if another thread moves a
//! structure during the frame.

use std::collections::{HashMap, HashSet};

use super::{CameraView, CurveNetwork, PointCloud, Structure, SurfaceMesh, VolumeGrid, VolumeMesh};
use polyscope_core::Context;
use polyscope_core::structure::HasQuantities;
use polyscope_render::{PickElementType, RenderEngine};
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

/// Creates the missing pick resources of the structures with render data.
///
/// The frame preparation calls this function before `update_gpu_buffers`,
/// which writes the pick uniforms together with the render uniforms. Thus new
/// pick resources get the model and the radius of the render uniforms of the
/// same frame. Volume grid quantities create their pick resources in
/// `init_volume_grid_quantities`. New pick resources keep the first pick index
/// of their structure until the pick pass gives the range of the frame.
pub(super) fn init_pick_resources(engine: &mut RenderEngine) {
    crate::with_context_mut(|ctx| {
        for structure in ctx.registry.iter_mut() {
            if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                if pc.pick_bind_group().is_none() && pc.render_data().is_some() {
                    let start = pc.pick_global_start();
                    pc.init_pick_resources(
                        &engine.device,
                        engine.pick_bind_group_layout(),
                        engine.camera_buffer(),
                        start,
                    );
                }
            } else if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
                if mesh.pick_bind_group().is_none() && mesh.render_data().is_some() {
                    let start = mesh.pick_global_start();
                    mesh.init_pick_resources(
                        &engine.device,
                        engine.mesh_pick_bind_group_layout(),
                        engine.camera_buffer(),
                        start,
                    );
                }
            } else if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
                if cn.render_data().is_none() {
                    continue;
                }
                if cn.pick_bind_group().is_none() {
                    if !engine.has_curve_network_pick_pipeline() {
                        engine.init_curve_network_pick_pipeline();
                    }
                    let start = cn.pick_global_start();
                    cn.init_pick_resources(
                        &engine.device,
                        engine.pick_bind_group_layout(),
                        engine.camera_buffer(),
                        start,
                    );
                }
                // The tube pick (ray-cylinder intersection) gives a larger
                // clickable area than the line pick.
                if !cn.has_tube_pick_resources() {
                    if !engine.has_curve_network_tube_pick_pipeline() {
                        engine.init_curve_network_tube_pick_pipeline();
                    }
                    cn.init_tube_pick_resources(
                        &engine.device,
                        engine.curve_network_tube_pick_bind_group_layout(),
                        engine.camera_buffer(),
                    );
                }
            } else if let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>() {
                // The curve network line pick draws the wireframe edges.
                if cv.pick_bind_group().is_none() && cv.render_data().is_some() {
                    if !engine.has_curve_network_pick_pipeline() {
                        engine.init_curve_network_pick_pipeline();
                    }
                    let start = cv.pick_global_start();
                    cv.init_pick_resources(
                        &engine.device,
                        engine.pick_bind_group_layout(),
                        engine.camera_buffer(),
                        start,
                    );
                }
            } else if let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>()
                && vm.pick_bind_group().is_none()
                && vm.render_data().is_some()
            {
                if !engine.has_mesh_pick_pipeline() {
                    engine.init_mesh_pick_pipeline();
                }
                let start = vm.pick_global_start();
                vm.init_pick_resources(
                    &engine.device,
                    engine.mesh_pick_bind_group_layout(),
                    engine.camera_buffer(),
                    start,
                );
            }
        }
    });
}

/// The element type of each volume grid quantity range of a pick pass, by
/// grid name and quantity index: a cell for a cell quantity, and a node
/// (shown as a vertex) for a node quantity.
pub(super) type GridPickElementTypes = HashMap<(String, usize), PickElementType>;

/// Renders the pick pass of a window frame.
///
/// The window runs it after the scene passes of the frame, so it uses the
/// uniforms and the curve network tube geometry of that frame. One lock of the
/// context covers the whole pass: it gives the pick ranges of the frame,
/// writes the first pick index of each owner and draws. Thus another thread
/// cannot change the registry between these steps, and the pick buffer always
/// matches the pick ranges of the frame.
///
/// Returns the element type of each gridcube range, so that a click reports
/// the type of the frame that it sees, also if the quantities change later.
pub(super) fn render_pick_pass(engine: &mut RenderEngine) -> GridPickElementTypes {
    crate::with_context_mut(|ctx| {
        let grid_types = assign_pick_ranges(engine, ctx);
        draw_pick_owners(engine, ctx);
        grid_types
    })
}

/// Gives the pick ranges of the frame to the visible owners with pick
/// resources and writes the first pick index of each owner.
///
/// Each frame gives all ranges again, one after the other, from the current
/// element counts. Thus a count change, a removed structure or an empty
/// structure leaves no unused indices, and two ranges cannot overlap. A hidden
/// structure gets no range, so it cannot use up the indices of visible
/// structures. Only the first pick index is written: the other pick uniforms
/// come from the frame preparation.
fn assign_pick_ranges(engine: &mut RenderEngine, ctx: &mut Context) -> GridPickElementTypes {
    engine.clear_pick_ranges();
    let mut grid_types = GridPickElementTypes::new();
    let visible: HashSet<(&'static str, String)> = ctx
        .registry
        .iter()
        .filter(|structure| ctx.is_structure_visible(*structure))
        .map(|structure| (structure.type_name(), structure.name().to_string()))
        .collect();

    for structure in ctx.registry.iter_mut() {
        let key = (structure.type_name(), structure.name().to_string());
        if !visible.contains(&key) {
            continue;
        }
        let name = key.1;

        if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
            if pc.pick_bind_group().is_some()
                && let Some(count) = pc.render_data().map(|rd| rd.num_points)
            {
                let start = engine.assign_pick_range("PointCloud", &name, count);
                pc.write_pick_global_start(&engine.queue, start);
            }
        } else if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
            if mesh.pick_bind_group().is_some() {
                let count = mesh.num_faces() as u32;
                let start = engine.assign_pick_range("SurfaceMesh", &name, count);
                mesh.write_pick_global_start(&engine.queue, start);
            }
        } else if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
            // The line pick and the tube pick share the range.
            if let Some(count) = cn.render_data().map(|rd| rd.num_edges) {
                let start = engine.assign_pick_range("CurveNetwork", &name, count);
                cn.write_pick_global_start(&engine.queue, start);
            }
        } else if let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>() {
            if cv.pick_bind_group().is_some()
                && let Some(count) = cv.render_data().map(|rd| rd.num_edges)
            {
                let start = engine.assign_pick_range("CameraView", &name, count);
                cv.write_pick_global_start(&engine.queue, start);
            }
        } else if let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>() {
            // Each gridcube quantity owns a range. The range belongs to the
            // index of the quantity.
            for (index, quantity) in vg.quantities_mut().iter_mut().enumerate() {
                if !quantity.is_enabled() {
                    continue;
                }
                if let Some(nsq) = quantity
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    if nsq.viz_mode() == VolumeGridVizMode::Gridcube
                        && nsq.pick_bind_group().is_some()
                    {
                        let count = nsq.num_pick_elements();
                        let start =
                            engine.assign_quantity_pick_range("VolumeGrid", &name, index, count);
                        nsq.write_pick_global_start(&engine.queue, start);
                        grid_types.insert((name.clone(), index), PickElementType::Vertex);
                    }
                } else if let Some(csq) = quantity
                    .as_any_mut()
                    .downcast_mut::<VolumeGridCellScalarQuantity>()
                    && csq.pick_bind_group().is_some()
                {
                    let count = csq.num_pick_elements();
                    let start =
                        engine.assign_quantity_pick_range("VolumeGrid", &name, index, count);
                    csq.write_pick_global_start(&engine.queue, start);
                    grid_types.insert((name.clone(), index), PickElementType::Cell);
                }
            }
        } else if let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>()
            && vm.pick_bind_group().is_some()
        {
            let count = vm.num_cells() as u32;
            let start = engine.assign_pick_range("VolumeMesh", &name, count);
            vm.write_pick_global_start(&engine.queue, start);
        }
    }
    grid_types
}

/// Returns whether an owner draws with the pick range of this frame.
///
/// `assign_pick_ranges` gives a range to each owner that the pick pass
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
    /// Runs the passes of a window frame without the surface and the UI, in
    /// the order of the window: frame preparation, scene passes and pick
    /// pass. It works with a headless engine.
    pub(crate) fn render_pick_frame(&mut self) {
        self.render_pick_frame_with(|| {});
    }

    /// Runs `between` after the frame preparation, where the window builds
    /// the UI and where another thread can change the registry. Then it runs
    /// the scene passes and the pick pass as `render_pick_frame` does.
    pub(crate) fn render_pick_frame_with(&mut self, between: impl FnOnce()) {
        self.prepare_frame(true);
        between();
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
            self.grid_pick_element_types = render_pick_pass(engine);
            self.pick_view = self.prepared_pick_view.take();
        }
    }
}
