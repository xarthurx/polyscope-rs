// Shared GPU initialization code for windowed and headless rendering

use glam::Vec3;
use polyscope_core::{MaterialLoadRequest, slice_plane::SlicePlaneUniforms};
use polyscope_render::RenderEngine;
use polyscope_structures::{
    CameraView, CurveNetwork, PointCloud, SurfaceMesh, VolumeGrid, VolumeMesh,
};

/// Auto-fit camera to scene on first render with structures.
/// Returns the new `camera_fitted` value.
pub fn auto_fit_camera(engine: &mut RenderEngine, camera_fitted: bool) -> bool {
    if !camera_fitted {
        let (has_structures, bbox) = crate::with_context(|ctx| {
            let has_structures = !ctx.registry.is_empty();
            (has_structures, ctx.bounding_box)
        });

        if has_structures {
            let (min, max) = bbox;
            // Only fit if bounding box is valid (not default zeros or infinities)
            if min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0 {
                engine.camera.look_at_box(min, max);
                return true;
            }
        }
    }
    camera_fitted
}

/// Drain deferred material load queue and load materials into the engine.
pub fn drain_material_queue(engine: &mut RenderEngine) {
    let pending_materials: Vec<MaterialLoadRequest> =
        crate::with_context_mut(|ctx| std::mem::take(&mut ctx.material_load_queue));

    for req in pending_materials {
        match req {
            MaterialLoadRequest::Static { name, path } => {
                if let Err(e) = engine.load_static_material(&name, &path) {
                    eprintln!("Failed to load static material '{name}': {e}");
                }
            }
            MaterialLoadRequest::Blendable { name, filenames } => {
                let refs: [&str; 4] = [&filenames[0], &filenames[1], &filenames[2], &filenames[3]];
                if let Err(e) = engine.load_blendable_material(&name, refs) {
                    eprintln!("Failed to load blendable material '{name}': {e}");
                }
            }
        }
    }
}

/// Update camera and slice plane uniforms.
pub fn update_uniforms(engine: &mut RenderEngine) {
    engine.update_camera_uniforms();

    crate::with_context(|ctx| {
        engine.update_slice_plane_uniforms(ctx.slice_planes().map(SlicePlaneUniforms::from));
    });
}

/// Initialize GPU resources for all structures (shared subset between windowed and headless).
/// This includes:
/// - PointCloud: init_gpu_resources + vector quantity init
/// - SurfaceMesh: init_gpu_resources + shadow resources + ALL vector/intrinsic/one-form quantity init
/// - CurveNetwork: init_gpu_resources + tube resources + node resources
/// - CameraView: init_render_data
/// - VolumeGrid: init_render_data (base wireframe only, NOT quantity init)
/// - VolumeMesh: init_render_data + slice plane culling
///
/// Windowed-only extras (pick resources, VolumeGrid quantity init) are kept in render() inline.
pub fn init_structure_gpu_resources(engine: &mut RenderEngine) {
    crate::with_context_mut(|ctx| {
        // Collect slice plane data before the loop to avoid borrow conflicts
        let slice_planes: Vec<_> = ctx.slice_planes().cloned().collect();

        for structure in ctx.registry.iter_mut() {
            if structure.type_name() == "PointCloud" {
                if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                    // Initialize point cloud render data
                    if pc.render_data().is_none() {
                        pc.init_gpu_resources(
                            &engine.device,
                            engine.point_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }

                    // Initialize vector quantity render data if enabled
                    let points = pc.points().to_vec();
                    if let Some(vq) = pc.active_vector_quantity_mut() {
                        if vq.render_data().is_none() {
                            vq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &points,
                            );
                        }
                    }
                }
            }

            if structure.type_name() == "SurfaceMesh" {
                if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
                    if mesh.render_data().is_none() {
                        mesh.init_gpu_resources(
                            &engine.device,
                            engine.mesh_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }

                    // Initialize shadow resources if render data exists but shadow doesn't
                    if mesh.render_data().is_some() && !mesh.has_shadow_resources() {
                        if let (Some(shadow_layout), Some(shadow_pass)) =
                            (engine.shadow_bind_group_layout(), engine.shadow_map_pass())
                        {
                            mesh.init_shadow_resources(
                                &engine.device,
                                shadow_layout,
                                shadow_pass.light_buffer(),
                            );
                        }
                    }

                    // Initialize vertex vector quantity render data if enabled
                    let vertices = mesh.vertices().to_vec();
                    if let Some(vq) = mesh.active_vertex_vector_quantity_mut() {
                        if vq.render_data().is_none() {
                            vq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &vertices,
                            );
                        }
                    }

                    // Initialize face vector quantity render data if enabled
                    let centroids = mesh.face_centroids();
                    if let Some(vq) = mesh.active_face_vector_quantity_mut() {
                        if vq.render_data().is_none() {
                            vq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &centroids,
                            );
                        }
                    }

                    // Initialize vertex intrinsic vector quantity render data if enabled
                    let vertices = mesh.vertices().to_vec();
                    if let Some(iq) = mesh.active_vertex_intrinsic_vector_quantity_mut() {
                        if iq.render_data().is_none() {
                            iq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &vertices,
                            );
                        }
                    }

                    // Initialize face intrinsic vector quantity render data if enabled
                    let centroids = mesh.face_centroids();
                    if let Some(iq) = mesh.active_face_intrinsic_vector_quantity_mut() {
                        if iq.render_data().is_none() {
                            iq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &centroids,
                            );
                        }
                    }

                    // Initialize one-form quantity render data if enabled
                    let vertices = mesh.vertices().to_vec();
                    let edges = mesh.edges().to_vec();
                    if let Some(oq) = mesh.active_one_form_quantity_mut() {
                        if oq.render_data().is_none() {
                            oq.init_gpu_resources(
                                &engine.device,
                                engine.vector_bind_group_layout(),
                                engine.camera_buffer(),
                                &vertices,
                                &edges,
                            );
                        }
                    }
                }
            }

            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
                    if cn.render_data().is_none() {
                        cn.init_gpu_resources(
                            &engine.device,
                            engine.curve_network_edge_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }
                    // Check what needs initialization
                    let needs_tube = cn.render_data().is_some_and(|rd| !rd.has_tube_resources());
                    let needs_node = cn
                        .render_data()
                        .is_some_and(|rd| !rd.has_node_render_resources());

                    // Initialize tube resources if not already done
                    if needs_tube {
                        cn.init_tube_resources(
                            &engine.device,
                            engine.curve_network_tube_compute_bind_group_layout(),
                            engine.curve_network_tube_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }
                    // Initialize node render resources for sphere joints
                    if needs_node {
                        cn.init_node_render_resources(
                            &engine.device,
                            engine.point_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }
                }
            }

            if structure.type_name() == "CameraView" {
                if let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>() {
                    if cv.needs_reinit(ctx.length_scale) {
                        cv.init_render_data(
                            &engine.device,
                            engine.curve_network_edge_bind_group_layout(),
                            engine.camera_buffer(),
                            &engine.queue,
                            ctx.length_scale,
                        );
                    }
                }
            }

            if structure.type_name() == "VolumeGrid" {
                if let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>() {
                    if vg.render_data().is_none() {
                        vg.init_render_data(
                            &engine.device,
                            engine.curve_network_edge_bind_group_layout(),
                            engine.camera_buffer(),
                            &engine.queue,
                        );
                    }
                    // Note: VolumeGrid quantity init (gridcube/isosurface) is windowed-only
                }
            }

            if structure.type_name() == "VolumeMesh" {
                if let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>() {
                    let mut enabled_planes: Vec<(String, Vec3, Vec3)> = slice_planes
                        .iter()
                        .filter(|p| p.is_enabled())
                        .map(|p| (p.name().to_string(), p.origin(), p.normal()))
                        .collect();
                    enabled_planes.sort_by(|a, b| a.0.cmp(&b.0));
                    let plane_params: Vec<(Vec3, Vec3)> = enabled_planes
                        .iter()
                        .map(|(_, origin, normal)| (*origin, *normal))
                        .collect();

                    if !plane_params.is_empty() {
                        vm.update_render_data_with_culling(
                            &engine.device,
                            engine.mesh_bind_group_layout(),
                            engine.camera_buffer(),
                            &plane_params,
                        );
                    } else if vm.is_culled() {
                        vm.reset_render_data(
                            &engine.device,
                            engine.mesh_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    } else if vm.render_data().is_none() {
                        vm.init_render_data(
                            &engine.device,
                            engine.mesh_bind_group_layout(),
                            engine.camera_buffer(),
                        );
                    }
                }
            }
        }
    });
}

/// Update GPU buffers for all structures.
/// This is nearly identical in both windowed and headless paths, except windowed also updates
/// pick uniforms for PointCloud, SurfaceMesh, and VolumeMesh (which are skipped here).
pub fn update_gpu_buffers(engine: &RenderEngine, update_pick_uniforms: bool) {
    crate::with_context(|ctx| {
        for structure in ctx.registry.iter() {
            if structure.type_name() == "PointCloud" {
                if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                    pc.update_gpu_buffers(&engine.queue, &engine.color_maps);
                    if update_pick_uniforms {
                        pc.update_pick_uniforms(&engine.queue);
                    }

                    // Update vector quantity uniforms
                    let model = structure.transform();
                    if let Some(vq) = pc.active_vector_quantity() {
                        vq.update_uniforms(&engine.queue, &model);
                    }
                }
            }

            if structure.type_name() == "SurfaceMesh" {
                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                    mesh.update_gpu_buffers(&engine.queue, &engine.color_maps);
                    if update_pick_uniforms {
                        mesh.update_pick_uniforms(&engine.queue);
                    }

                    // Update vertex vector quantity uniforms
                    let model = structure.transform();
                    if let Some(vq) = mesh.active_vertex_vector_quantity() {
                        vq.update_uniforms(&engine.queue, &model);
                    }

                    // Update face vector quantity uniforms
                    if let Some(vq) = mesh.active_face_vector_quantity() {
                        vq.update_uniforms(&engine.queue, &model);
                    }

                    // Update vertex intrinsic vector quantity uniforms
                    if let Some(iq) = mesh.active_vertex_intrinsic_vector_quantity() {
                        iq.update_uniforms(&engine.queue, &model);
                    }

                    // Update face intrinsic vector quantity uniforms
                    if let Some(iq) = mesh.active_face_intrinsic_vector_quantity() {
                        iq.update_uniforms(&engine.queue, &model);
                    }

                    // Update one-form quantity uniforms
                    if let Some(oq) = mesh.active_one_form_quantity() {
                        oq.update_uniforms(&engine.queue, &model);
                    }
                }
            }

            if structure.type_name() == "CurveNetwork" {
                if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                    cn.update_gpu_buffers(&engine.queue, &engine.color_maps);
                }
            }

            if structure.type_name() == "VolumeGrid" {
                if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                    vg.update_gpu_buffers(&engine.queue);
                }
            }

            if structure.type_name() == "VolumeMesh" {
                if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                    vm.update_gpu_buffers(&engine.queue);
                    if update_pick_uniforms {
                        vm.update_pick_uniforms(&engine.queue);
                    }
                }
            }
        }
    });
}
