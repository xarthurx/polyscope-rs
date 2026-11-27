//! GPU data of volume grid scalar quantities: gridcubes and isosurfaces.
//!
//! The window and every capture build this data in the same way.

use super::{Structure, Vec3, VolumeGrid};
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::HasQuantities;
use polyscope_render::{
    GridcubeRenderData, GridcubeUniforms, IsosurfaceRenderData, RenderEngine, SimpleMeshUniforms,
};
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

/// An isosurface that the UI asked to register as a surface mesh:
/// name, vertices and triangles.
pub(super) type IsosurfaceMesh = (String, Vec<Vec3>, Vec<[u32; 3]>);

/// Returns whether a structure is a volume grid with an enabled quantity that
/// shows gridcubes.
fn has_enabled_gridcubes(structure: &dyn Structure) -> bool {
    structure
        .as_any()
        .downcast_ref::<VolumeGrid>()
        .is_some_and(|vg| {
            vg.quantities().iter().any(|q| {
                q.is_enabled()
                    && (q.as_any().is::<VolumeGridCellScalarQuantity>()
                        || q.as_any()
                            .downcast_ref::<VolumeGridNodeScalarQuantity>()
                            .is_some_and(|nsq| nsq.viz_mode() == VolumeGridVizMode::Gridcube))
            })
        })
}

/// Builds the GPU data of the enabled volume grid scalar quantities and
/// updates their uniforms.
///
/// An interactive frame (the window) also creates the pick resources of the
/// gridcubes. It writes their pick uniforms in the same lock and with the same
/// model and cube size as their render uniforms, so the pick pass draws the
/// gridcubes where the scene draws them.
///
/// Returns the isosurfaces that the UI asked to register as surface meshes.
/// Register them with [`register_isosurface_meshes`] after the context lock
/// is released.
pub(super) fn init_volume_grid_quantities(
    engine: &mut RenderEngine,
    interactive: bool,
) -> Vec<IsosurfaceMesh> {
    let mut meshes_to_register: Vec<IsosurfaceMesh> = Vec::new();
    crate::with_context_mut(|ctx| {
        // The gridcube pick pipeline exists before the first gridcube pick resources.
        if interactive
            && !engine.has_gridcube_pick_pipeline()
            && ctx.registry.iter().any(has_enabled_gridcubes)
        {
            engine.init_gridcube_pick_pipeline();
        }
        let engine = &*engine;

        for structure in ctx.registry.iter_mut() {
            if structure.type_name() == "VolumeGrid"
                && let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>()
            {
                let grid_spacing = vg.grid_spacing();
                let cube_size_factor = vg.cube_size_factor();
                let transform = vg.transform();
                let node_dim = vg.node_dim();
                let bound_min = vg.bound_min();
                let bound_max = vg.bound_max();

                for quantity in vg.quantities_mut() {
                    if !quantity.is_enabled() {
                        continue;
                    }

                    // Node scalar quantities: gridcube + isosurface
                    if let Some(nsq) = quantity
                        .as_any_mut()
                        .downcast_mut::<VolumeGridNodeScalarQuantity>()
                    {
                        match nsq.viz_mode() {
                            VolumeGridVizMode::Gridcube => {
                                if nsq.gridcube_render_data().is_none() || nsq.gridcube_dirty() {
                                    // Generate node center positions
                                    let mut centers = Vec::new();
                                    let cell_dim_f = Vec3::new(
                                        (node_dim.x - 1).max(1) as f32,
                                        (node_dim.y - 1).max(1) as f32,
                                        (node_dim.z - 1).max(1) as f32,
                                    );
                                    for k in 0..node_dim.z {
                                        for j in 0..node_dim.y {
                                            for i in 0..node_dim.x {
                                                let t = Vec3::new(i as f32, j as f32, k as f32)
                                                    / cell_dim_f;
                                                centers
                                                    .push(bound_min + t * (bound_max - bound_min));
                                            }
                                        }
                                    }
                                    // Half the cell size. The shader applies the cube
                                    // size factor.
                                    let half_size = grid_spacing.min_element() * 0.5;

                                    // Sample colormap
                                    let colormap_colors: Vec<Vec3> =
                                        if let Some(cm) = engine.color_maps.get(nsq.color_map()) {
                                            cm.colors.clone()
                                        } else {
                                            vec![Vec3::ZERO, Vec3::ONE]
                                        };

                                    let data = GridcubeRenderData::new(
                                        &engine.device,
                                        &engine.queue,
                                        engine.gridcube_bind_group_layout(),
                                        engine.camera_buffer(),
                                        &centers,
                                        half_size,
                                        nsq.values(),
                                        &colormap_colors,
                                    );
                                    nsq.set_gridcube_render_data(data);
                                }
                            }
                            VolumeGridVizMode::Isosurface => {
                                if nsq.isosurface_render_data().is_none() || nsq.isosurface_dirty()
                                {
                                    let mesh = nsq.extract_isosurface();
                                    if mesh.vertices.is_empty() {
                                        // Isovalue outside data range — clear old surface
                                        nsq.clear_isosurface_render_data();
                                    } else {
                                        let vertices = mesh.vertices.clone();
                                        let normals = mesh.normals.clone();
                                        let indices = mesh.indices.clone();
                                        let data = IsosurfaceRenderData::new(
                                            &engine.device,
                                            engine.simple_mesh_bind_group_layout(),
                                            engine.camera_buffer(),
                                            &vertices,
                                            &normals,
                                            &indices,
                                        );
                                        nsq.set_isosurface_render_data(data);
                                    }
                                }
                            }
                        }

                        // Update uniforms every frame (model matrix may change)
                        if let Some(rd) = nsq.gridcube_render_data() {
                            let (data_min, data_max) = nsq.data_range();
                            let uniforms = GridcubeUniforms {
                                model: transform.to_cols_array_2d(),
                                cube_size_factor,
                                data_min,
                                data_max,
                                transparency: 0.0,
                                slice_planes_enabled: 0,
                                ..Default::default()
                            };
                            rd.update_uniforms(&engine.queue, &uniforms);
                        }
                        // The window picks the gridcubes of gridcube mode.
                        if interactive
                            && nsq.viz_mode() == VolumeGridVizMode::Gridcube
                            && nsq.gridcube_render_data().is_some()
                        {
                            if nsq.pick_bind_group().is_none() {
                                let start = nsq.pick_global_start();
                                nsq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    start,
                                );
                            }
                            nsq.update_pick_uniforms(
                                &engine.queue,
                                transform.to_cols_array_2d(),
                                cube_size_factor,
                            );
                        }
                        if let Some(rd) = nsq.isosurface_render_data() {
                            let color = nsq.isosurface_color();
                            let uniforms = SimpleMeshUniforms {
                                model: transform.to_cols_array_2d(),
                                base_color: [color.x, color.y, color.z, 1.0],
                                transparency: 0.0,
                                slice_planes_enabled: 0,
                                ..Default::default()
                            };
                            rd.update_uniforms(&engine.queue, &uniforms);
                        }
                    }

                    // Check for "Register as Surface Mesh" request
                    if let Some(nsq) = quantity
                        .as_any_mut()
                        .downcast_mut::<VolumeGridNodeScalarQuantity>()
                        && nsq.register_as_mesh_requested()
                    {
                        if let Some(mesh) = nsq.isosurface_mesh() {
                            let verts = mesh.vertices.clone();
                            let tris: Vec<[u32; 3]> =
                                mesh.indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
                            let name = format!("{} isosurface", nsq.name());
                            meshes_to_register.push((name, verts, tris));
                        }
                        nsq.clear_register_as_mesh_request();
                    }

                    // Cell scalar quantities: gridcube only
                    if let Some(csq) = quantity
                        .as_any_mut()
                        .downcast_mut::<VolumeGridCellScalarQuantity>()
                    {
                        if csq.gridcube_render_data().is_none() || csq.gridcube_dirty() {
                            let cell_dim = node_dim.saturating_sub(glam::UVec3::ONE);
                            let cell_spacing = (bound_max - bound_min)
                                / Vec3::new(
                                    cell_dim.x.max(1) as f32,
                                    cell_dim.y.max(1) as f32,
                                    cell_dim.z.max(1) as f32,
                                );
                            let half_cell_spacing = cell_spacing * 0.5;

                            // Generate cell center positions
                            let mut centers = Vec::new();
                            for k in 0..cell_dim.z {
                                for j in 0..cell_dim.y {
                                    for i in 0..cell_dim.x {
                                        let node_pos = bound_min
                                            + Vec3::new(i as f32, j as f32, k as f32)
                                                * cell_spacing;
                                        centers.push(node_pos + half_cell_spacing);
                                    }
                                }
                            }
                            // Half the cell size. The shader applies the cube size factor.
                            let half_size = cell_spacing.min_element() * 0.5;

                            let colormap_colors: Vec<Vec3> =
                                if let Some(cm) = engine.color_maps.get(csq.color_map()) {
                                    cm.colors.clone()
                                } else {
                                    vec![Vec3::ZERO, Vec3::ONE]
                                };

                            let data = GridcubeRenderData::new(
                                &engine.device,
                                &engine.queue,
                                engine.gridcube_bind_group_layout(),
                                engine.camera_buffer(),
                                &centers,
                                half_size,
                                csq.values(),
                                &colormap_colors,
                            );
                            csq.set_gridcube_render_data(data);
                        }

                        // Update uniforms every frame
                        if let Some(rd) = csq.gridcube_render_data() {
                            let (data_min, data_max) = csq.data_range();
                            let uniforms = GridcubeUniforms {
                                model: transform.to_cols_array_2d(),
                                cube_size_factor,
                                data_min,
                                data_max,
                                transparency: 0.0,
                                slice_planes_enabled: 0,
                                ..Default::default()
                            };
                            rd.update_uniforms(&engine.queue, &uniforms);
                        }
                        if interactive && csq.gridcube_render_data().is_some() {
                            if csq.pick_bind_group().is_none() {
                                let start = csq.pick_global_start();
                                csq.init_pick_resources(
                                    &engine.device,
                                    engine.gridcube_pick_bind_group_layout(),
                                    engine.camera_buffer(),
                                    start,
                                );
                            }
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
    });
    meshes_to_register
}

/// Registers the requested isosurfaces as surface meshes.
///
/// A timestamp in the name prevents name conflicts.
pub(super) fn register_isosurface_meshes(meshes: Vec<IsosurfaceMesh>) {
    for (name, vertices, triangles) in meshes {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let unique_name = format!("{name}_{timestamp}");
        let faces: Vec<Vec<u32>> = triangles.iter().map(|t| vec![t[0], t[1], t[2]]).collect();
        crate::register_surface_mesh(unique_name, vertices, faces);
    }
}
