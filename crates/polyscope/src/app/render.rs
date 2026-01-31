use super::{App, Structure, SlicePlaneUniforms, PointCloud, SurfaceMesh, CurveNetwork, CameraView, VolumeGrid, VolumeMesh, Vec3, GroundPlaneMode, reflection, ScreenDescriptor};
use polyscope_structures::volume_grid::{VolumeGridNodeScalarQuantity, VolumeGridCellScalarQuantity, VolumeGridVizMode};
use polyscope_render::{GridcubeRenderData, GridcubeUniforms, IsosurfaceRenderData, SimpleMeshUniforms};
use polyscope_core::structure::HasQuantities;
use polyscope_core::quantity::Quantity;

impl App {
    /// Renders a single frame.
    pub(super) fn render(&mut self) {
        let (Some(engine), Some(egui), Some(window)) =
            (&mut self.engine, &mut self.egui, &self.window)
        else {
            return;
        };

        // Check surface exists (but don't hold borrow yet - needed for structure ID assignment)
        if engine.surface.is_none() {
            return;
        }

        // Auto-fit camera to scene on first render with structures
        if !self.camera_fitted {
            let (has_structures, bbox) = crate::with_context(|ctx| {
                let has_structures = !ctx.registry.is_empty();
                (has_structures, ctx.bounding_box)
            });

            if has_structures {
                let (min, max) = bbox;
                // Only fit if bounding box is valid (not default zeros or infinities)
                if min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0 {
                    engine.camera.look_at_box(min, max);
                    self.camera_fitted = true;
                }
            }
        }

        // Update camera uniforms
        engine.update_camera_uniforms();

        // Update slice plane uniforms
        crate::with_context(|ctx| {
            engine.update_slice_plane_uniforms(ctx.slice_planes().map(SlicePlaneUniforms::from));
        });

        // Initialize GPU resources for any uninitialized point clouds and vector quantities
        // Collect deferred mesh registrations (from "Register as Surface Mesh" button)
        let mut meshes_to_register: Vec<(String, Vec<Vec3>, Vec<[u32; 3]>)> = Vec::new();
        crate::with_context_mut(|ctx| {
            // Collect slice plane data before the loop to avoid borrow conflicts
            let slice_planes: Vec<_> = ctx.slice_planes().cloned().collect();

            for structure in ctx.registry.iter_mut() {
                if structure.type_name() == "PointCloud" {
                    let structure_name = structure.name().to_string();
                    if let Some(pc) = structure.as_any_mut().downcast_mut::<PointCloud>() {
                        // Initialize point cloud render data
                        if pc.render_data().is_none() {
                            pc.init_gpu_resources(
                                &engine.device,
                                engine.point_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        }

                        // Initialize pick resources (after render data)
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
                        // Initialize pick resources (after render data)
                        if mesh.pick_bind_group().is_none() && mesh.render_data().is_some() {
                            let num_faces = mesh.num_faces() as u32;
                            let global_start = engine.assign_pick_range(
                                "SurfaceMesh",
                                mesh.name(),
                                num_faces,
                            );
                            mesh.init_pick_resources(
                                &engine.device,
                                engine.mesh_pick_bind_group_layout(),
                                engine.camera_buffer(),
                                global_start,
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
                        let needs_tube =
                            cn.render_data().is_some_and(|rd| !rd.has_tube_resources());
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

                        // Initialize pick resources (after render data)
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
                }

                if structure.type_name() == "CameraView" {
                    if let Some(cv) = structure.as_any_mut().downcast_mut::<CameraView>() {
                        if cv.render_data().is_none() {
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

                        // Initialize GPU resources for enabled scalar quantities
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
                            if let Some(nsq) = quantity.as_any_mut().downcast_mut::<VolumeGridNodeScalarQuantity>() {
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
                                                        let t = Vec3::new(i as f32, j as f32, k as f32) / cell_dim_f;
                                                        centers.push(bound_min + t * (bound_max - bound_min));
                                                    }
                                                }
                                            }
                                            let half_size = grid_spacing.min_element() * 0.5 * cube_size_factor.max(0.5);

                                            // Sample colormap
                                            let colormap_colors: Vec<Vec3> = if let Some(cm) = engine.color_maps.get(nsq.color_map()) {
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
                                        if nsq.isosurface_render_data().is_none() || nsq.isosurface_dirty() {
                                            let mesh = nsq.extract_isosurface();
                                            if !mesh.vertices.is_empty() {
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
                                            } else {
                                                // Isovalue outside data range — clear old surface
                                                nsq.clear_isosurface_render_data();
                                            }
                                        }
                                    }
                                }

                                // Update uniforms every frame (model matrix may change)
                                if let Some(rd) = nsq.gridcube_render_data() {
                                    let (data_min, data_max) = nsq.data_range();
                                    let uniforms = GridcubeUniforms {
                                        model: transform.to_cols_array_2d(),
                                        cube_size_factor: cube_size_factor.max(0.5),
                                        data_min,
                                        data_max,
                                        transparency: 0.0,
                                        slice_planes_enabled: 0,
                                        ..Default::default()
                                    };
                                    rd.update_uniforms(&engine.queue, &uniforms);
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
                            if let Some(nsq) = quantity.as_any_mut().downcast_mut::<VolumeGridNodeScalarQuantity>() {
                                if nsq.register_as_mesh_requested() {
                                    if let Some(mesh) = nsq.isosurface_mesh() {
                                        let verts = mesh.vertices.clone();
                                        let tris: Vec<[u32; 3]> = mesh.indices.chunks(3)
                                            .map(|c| [c[0], c[1], c[2]])
                                            .collect();
                                        let name = format!("{} isosurface", nsq.name());
                                        meshes_to_register.push((name, verts, tris));
                                    }
                                    nsq.clear_register_as_mesh_request();
                                }
                            }

                            // Cell scalar quantities: gridcube only
                            if let Some(csq) = quantity.as_any_mut().downcast_mut::<VolumeGridCellScalarQuantity>() {
                                if csq.gridcube_render_data().is_none() || csq.gridcube_dirty() {
                                    let cell_dim = node_dim.saturating_sub(glam::UVec3::ONE);
                                    let cell_spacing = (bound_max - bound_min) / Vec3::new(
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
                                                let node_pos = bound_min + Vec3::new(i as f32, j as f32, k as f32) * cell_spacing;
                                                centers.push(node_pos + half_cell_spacing);
                                            }
                                        }
                                    }
                                    let half_size = cell_spacing.min_element() * 0.5 * cube_size_factor.max(0.5);

                                    let colormap_colors: Vec<Vec3> = if let Some(cm) = engine.color_maps.get(csq.color_map()) {
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
                                        cube_size_factor: cube_size_factor.max(0.5),
                                        data_min,
                                        data_max,
                                        transparency: 0.0,
                                        slice_planes_enabled: 0,
                                        ..Default::default()
                                    };
                                    rd.update_uniforms(&engine.queue, &uniforms);
                                }
                            }
                        }
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
                            // Use cell culling: regenerate geometry with only visible cells
                            // (cells whose centroid is on the positive side of all enabled planes)
                            vm.update_render_data_with_culling(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                                &plane_params,
                            );
                        } else if vm.is_culled() {
                            // Was culled but no slice plane is active now - reset to show all cells
                            vm.reset_render_data(
                                &engine.device,
                                engine.mesh_bind_group_layout(),
                                engine.camera_buffer(),
                            );
                        } else if vm.render_data().is_none() {
                            // No slice plane active, initialize normally
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

        // Update GPU buffers for point clouds and vector quantities
        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if structure.type_name() == "PointCloud" {
                    if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                        pc.update_gpu_buffers(&engine.queue, &engine.color_maps);
                        // Update pick uniforms (point radius may have changed)
                        pc.update_pick_uniforms(&engine.queue);

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
                        mesh.update_pick_uniforms(&engine.queue);

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
                    }
                }
            }
        });

        // Register any isosurface meshes requested via UI
        for (name, vertices, triangles) in meshes_to_register {
            // Add human-readable timestamp to avoid duplicate name conflicts
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let unique_name = format!("{name}_{timestamp}");
            let faces: Vec<Vec<u32>> = triangles.iter().map(|t| vec![t[0], t[1], t[2]]).collect();
            crate::register_surface_mesh(unique_name, vertices, faces);
        }

        // Render pick pass (GPU picking)
        {
            let mut encoder =
                engine
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
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let (Some(pick_bind_group), Some(render_data)) =
                                    (pc.pick_bind_group(), pc.render_data())
                                {
                                    pick_pass.set_bind_group(0, pick_bind_group, &[]);
                                    // 6 vertices per quad, num_points instances
                                    pick_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
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
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                let Some(render_data) = cn.render_data() else {
                                    continue;
                                };

                                // Use tube picking when available - provides larger clickable area
                                if engine.has_curve_network_tube_pick_pipeline()
                                    && cn.tube_pick_bind_group().is_some()
                                    && render_data.generated_vertex_buffer.is_some()
                                {
                                    // Use tube-based picking (ray-cylinder intersection)
                                    pick_pass.set_pipeline(engine.curve_network_tube_pick_pipeline());
                                    pick_pass
                                        .set_bind_group(0, cn.tube_pick_bind_group().unwrap(), &[]);
                                    pick_pass.set_vertex_buffer(
                                        0,
                                        render_data
                                            .generated_vertex_buffer
                                            .as_ref()
                                            .unwrap()
                                            .slice(..),
                                    );
                                    // 36 vertices per edge (bounding box triangles)
                                    pick_pass.draw(0..render_data.num_edges * 36, 0..1);
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
                    }
                });

                // Draw surface meshes to pick buffer
                if engine.has_mesh_pick_pipeline() {
                    pick_pass.set_pipeline(engine.mesh_pick_pipeline());
                    crate::with_context(|ctx| {
                        for structure in ctx.registry.iter() {
                            if !ctx.is_structure_visible(structure) {
                                continue;
                            }
                            if structure.type_name() == "SurfaceMesh" {
                                if let Some(mesh) =
                                    structure.as_any().downcast_ref::<SurfaceMesh>()
                                {
                                    if let Some(pick_bind_group) = mesh.pick_bind_group() {
                                        pick_pass
                                            .set_bind_group(0, pick_bind_group, &[]);
                                        pick_pass.draw(
                                            0..mesh.num_triangulation_vertices(),
                                            0..1,
                                        );
                                    }
                                }
                            }
                        }
                    });
                }
            }

            engine.queue.submit(std::iter::once(encoder.finish()));
        }

        // Begin egui frame
        egui.begin_frame(window);

        // Build UI
        let mut bg_color = [
            self.background_color.x,
            self.background_color.y,
            self.background_color.z,
        ];

        // Extract ground plane settings for UI
        let mut gp_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::Tile => 1u32,
            GroundPlaneMode::ShadowOnly => 2u32,
            GroundPlaneMode::TileReflection => 3u32,
        };
        let mut gp_height = self.ground_plane.height;
        let mut gp_height_is_relative = self.ground_plane.height_is_relative;
        let mut gp_shadow_blur_iters = self.ground_plane.shadow_blur_iters;
        let mut gp_shadow_darkness = self.ground_plane.shadow_darkness;
        let mut gp_reflection_intensity = self.ground_plane.reflection_intensity;

        // Sync camera settings from engine
        self.camera_settings = crate::camera_to_settings(&engine.camera);

        // Sync scene extents from context
        self.scene_extents = crate::get_scene_extents();

        let mut camera_changed = false;
        let mut scene_extents_changed = false;
        let mut screenshot_requested = false;
        let mut reset_view_requested = false;
        let mut ssaa_changed = false;

        let panel_width = polyscope_ui::build_left_panel(&egui.context, |ui| {
            let view_action = polyscope_ui::build_controls_section(ui, &mut bg_color);
            match view_action {
                polyscope_ui::ViewAction::Screenshot => {
                    screenshot_requested = true;
                }
                polyscope_ui::ViewAction::ResetView => {
                    reset_view_requested = true;
                }
                polyscope_ui::ViewAction::None => {}
            }

            // Camera settings panel
            if polyscope_ui::build_camera_settings_section(ui, &mut self.camera_settings) {
                camera_changed = true;
            }

            // Scene extents panel
            if polyscope_ui::build_scene_extents_section(ui, &mut self.scene_extents) {
                scene_extents_changed = true;
            }

            // Appearance settings panel
            if polyscope_ui::build_appearance_section(ui, &mut self.appearance_settings) {
                // Sync SSAO settings to global options
                polyscope_core::with_context_mut(|ctx| {
                    ctx.options.ssao.enabled = self.appearance_settings.ssao_enabled;
                    ctx.options.ssao.radius = self.appearance_settings.ssao_radius;
                    ctx.options.ssao.intensity = self.appearance_settings.ssao_intensity;
                    ctx.options.ssao.bias = self.appearance_settings.ssao_bias;
                    ctx.options.ssao.sample_count = self.appearance_settings.ssao_sample_count;
                    ctx.options.ssaa_factor = self.appearance_settings.ssaa_factor;
                });

                // Mark SSAA as changed (will apply outside closure)
                ssaa_changed = true;
            }

            // Tone mapping settings panel
            polyscope_ui::panels::build_tone_mapping_section(ui, &mut self.tone_mapping_settings);

            // Slice Planes section
            let slice_action = polyscope_ui::panels::build_slice_planes_section(
                ui,
                &mut self.slice_plane_settings,
                &mut self.new_slice_plane_name,
            );
            if slice_action != polyscope_ui::SlicePlanesAction::None {
                crate::handle_slice_plane_action(
                    slice_action.clone(),
                    &mut self.slice_plane_settings,
                );
                if matches!(slice_action, polyscope_ui::SlicePlanesAction::Add(_)) {
                    self.new_slice_plane_name.clear();
                }
            }

            // Sync selection info from context
            self.selection_info = crate::get_selection_info();

            // Gizmo section
            let gizmo_action = polyscope_ui::panels::build_gizmo_section(
                ui,
                &mut self.gizmo_settings,
                &mut self.selection_info,
            );
            if gizmo_action != polyscope_ui::GizmoAction::None {
                crate::handle_gizmo_action(
                    gizmo_action,
                    &self.gizmo_settings,
                    &self.selection_info,
                );
            }

            polyscope_ui::build_ground_plane_section(
                ui,
                &mut gp_mode,
                &mut gp_height,
                &mut gp_height_is_relative,
                &mut gp_shadow_blur_iters,
                &mut gp_shadow_darkness,
                &mut gp_reflection_intensity,
            );

            // Groups section (only shown if groups were created via API)
            let groups_action = polyscope_ui::panels::build_groups_section(
                ui,
                &mut self.group_settings,
            );
            if groups_action != polyscope_ui::GroupsAction::None {
                crate::handle_group_action(groups_action, &mut self.group_settings);
            }

            // Collect colormap names for VolumeGrid UI
            let colormap_names: Vec<String> = engine.color_maps.names().map(String::from).collect();
            let colormap_name_refs: Vec<&str> = colormap_names.iter().map(String::as_str).collect();

            // Collect structure info
            let structures: Vec<(String, String, bool)> = crate::with_context(|ctx| {
                ctx.registry
                    .iter()
                    .map(|s| {
                        (
                            s.type_name().to_string(),
                            s.name().to_string(),
                            s.is_enabled(),
                        )
                    })
                    .collect()
            });

            polyscope_ui::build_structure_tree_with_ui(
                ui,
                &structures,
                |type_name, name, enabled| {
                    crate::with_context_mut(|ctx| {
                        if let Some(s) = ctx.registry.get_mut(type_name, name) {
                            s.set_enabled(enabled);
                        }
                    });
                },
                |ui, type_name, name| {
                    // Build structure-specific UI
                    crate::with_context_mut(|ctx| {
                        if let Some(s) = ctx.registry.get_mut(type_name, name) {
                            if type_name == "PointCloud" {
                                if let Some(pc) = s.as_any_mut().downcast_mut::<PointCloud>() {
                                    pc.build_egui_ui(ui);
                                }
                            }
                            if type_name == "SurfaceMesh" {
                                if let Some(mesh) = s.as_any_mut().downcast_mut::<SurfaceMesh>() {
                                    mesh.build_egui_ui(ui);
                                }
                            }
                            if type_name == "CurveNetwork" {
                                if let Some(cn) = s.as_any_mut().downcast_mut::<CurveNetwork>() {
                                    cn.build_egui_ui(ui);
                                }
                            }
                            if type_name == "CameraView" {
                                if let Some(cv) = s.as_any_mut().downcast_mut::<CameraView>() {
                                    cv.build_egui_ui(ui);
                                }
                            }
                            if type_name == "VolumeGrid" {
                                if let Some(vg) = s.as_any_mut().downcast_mut::<VolumeGrid>() {
                                    vg.build_egui_ui(ui, &colormap_name_refs);
                                }
                            }
                            if type_name == "VolumeMesh" {
                                if let Some(vm) = s.as_any_mut().downcast_mut::<VolumeMesh>() {
                                    vm.build_egui_ui(ui);
                                }
                            }
                        }
                    });
                },
            );
        });
        // Update dynamic panel width (with small margin to account for resize handle)
        self.left_panel_width = f64::from(panel_width) + 5.0;

        // Show selection panel if we have a selection
        if let Some(ref selection) = self.selection {
            if selection.hit {
                polyscope_ui::build_selection_panel(&egui.context, selection, |ui| {
                    // Structure-specific pick UI (placeholder for now)
                    ui.label("Quantity values would appear here");
                });
            }
        }

        // Common gizmo setup - check if pointer is over UI panel
        let panel_w = self.left_panel_width as f32;
        let pointer_over_ui = egui.context.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|pos| pos.x <= panel_w)
        });

        // Get camera matrices from engine - MUST match what's used for 3D rendering
        let view_matrix = engine.camera.view_matrix();
        let projection_matrix = engine.camera.projection_matrix();

        // Common viewport for gizmo rendering
        let full_window_viewport = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(engine.width as f32, engine.height as f32),
        );

        // Render transform gizmo if visible and a structure is selected
        if self.gizmo_settings.visible && self.selection_info.has_selection {
            // Use centroid for gizmo position (so it appears at the center of the geometry)
            // but keep the rotation and scale from the actual transform
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.selection_info.centroid),
                glam::Vec3::from(self.selection_info.rotation_degrees),
                glam::Vec3::from(self.selection_info.scale),
            );

            // Use Area instead of CentralPanel to avoid consuming all mouse events
            // The gizmo handles its own interaction detection
            egui::Area::new(egui::Id::new("gizmo_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .interactable(false) // Don't consume mouse events at the area level
                .show(&egui.context, |ui| {
                    // Set the clip rect to full window
                    ui.set_clip_rect(full_window_viewport);

                    // Skip gizmo interaction when pointer is over UI panel to prevent flickering
                    if pointer_over_ui {
                        return;
                    }

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.local_space,
                        full_window_viewport,
                    ) {
                        // Decompose the new transform from gizmo
                        let (new_gizmo_pos, new_rotation_deg, new_scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);

                        // Get old values
                        let old_translation = glam::Vec3::from(self.selection_info.translation);
                        let old_rotation_deg =
                            glam::Vec3::from(self.selection_info.rotation_degrees);
                        let old_scale = glam::Vec3::from(self.selection_info.scale);
                        let world_centroid = glam::Vec3::from(self.selection_info.centroid);

                        // Compute local centroid (center of geometry in object space)
                        // world_centroid = translation + rotation * (local_centroid * scale)
                        // local_centroid = inverse(rotation) * (world_centroid - translation) / scale
                        let old_rotation = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            old_rotation_deg.x.to_radians(),
                            old_rotation_deg.y.to_radians(),
                            old_rotation_deg.z.to_radians(),
                        );
                        let local_centroid =
                            old_rotation.inverse() * (world_centroid - old_translation) / old_scale;

                        // Convert new rotation to quaternion
                        let new_rotation = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            new_rotation_deg.x.to_radians(),
                            new_rotation_deg.y.to_radians(),
                            new_rotation_deg.z.to_radians(),
                        );

                        // Compute new translation to keep world_centroid fixed during rotation/scale
                        // For pure rotation/scale: new_translation = world_centroid - new_rotation * (local_centroid * new_scale)
                        // For translation: the gizmo moves, so we use the new gizmo position as the new world_centroid

                        // Check if the gizmo position changed (user translated)
                        let gizmo_moved = (new_gizmo_pos - world_centroid).length() > 0.0001;

                        let (new_translation, new_centroid) = if gizmo_moved {
                            // User translated: new world_centroid = new_gizmo_pos
                            let new_world_centroid = new_gizmo_pos;
                            let new_trans =
                                new_world_centroid - new_rotation * (local_centroid * new_scale);
                            (new_trans, new_world_centroid)
                        } else {
                            // User rotated/scaled only: keep world_centroid fixed
                            let new_trans =
                                world_centroid - new_rotation * (local_centroid * new_scale);
                            (new_trans, world_centroid)
                        };

                        // Update selection info
                        self.selection_info.translation = new_translation.into();
                        self.selection_info.centroid = new_centroid.into();
                        self.selection_info.rotation_degrees = new_rotation_deg.into();
                        self.selection_info.scale = new_scale.into();

                        // Apply to selected structure
                        crate::handle_gizmo_action(
                            polyscope_ui::GizmoAction::TransformChanged,
                            &self.gizmo_settings,
                            &self.selection_info,
                        );

                        // Immediately update GPU buffers so structure renders at new position this frame
                        crate::with_context(|ctx| {
                            if let Some((type_name, name)) = ctx.selected_structure() {
                                if let Some(structure) = ctx.registry.get(type_name, name) {
                                    if type_name == "PointCloud" {
                                        if let Some(pc) =
                                            structure.as_any().downcast_ref::<PointCloud>()
                                        {
                                            pc.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "SurfaceMesh" {
                                        if let Some(mesh) =
                                            structure.as_any().downcast_ref::<SurfaceMesh>()
                                        {
                                            mesh.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "CurveNetwork" {
                                        if let Some(cn) =
                                            structure.as_any().downcast_ref::<CurveNetwork>()
                                        {
                                            cn.update_gpu_buffers(
                                                &engine.queue,
                                                &engine.color_maps,
                                            );
                                        }
                                    } else if type_name == "VolumeMesh" {
                                        if let Some(vm) =
                                            structure.as_any().downcast_ref::<VolumeMesh>()
                                        {
                                            vm.update_gpu_buffers(&engine.queue);
                                        }
                                    }
                                }
                            }
                        });
                    }
                });
        }

        // Render slice plane gizmo if a slice plane is selected
        // Check if any slice plane is selected via UI
        self.slice_plane_selection = crate::get_slice_plane_selection_info();

        // Also sync selection from UI settings
        for settings in &self.slice_plane_settings {
            if settings.is_selected && settings.enabled && settings.draw_widget {
                if !self.slice_plane_selection.has_selection
                    || self.slice_plane_selection.name != settings.name
                {
                    crate::select_slice_plane_for_gizmo(&settings.name);
                    self.slice_plane_selection = crate::get_slice_plane_selection_info();
                }
            } else if !settings.is_selected
                && self.slice_plane_selection.has_selection
                && self.slice_plane_selection.name == settings.name
            {
                crate::deselect_slice_plane_gizmo();
                self.slice_plane_selection = crate::get_slice_plane_selection_info();
            }
        }

        if self.gizmo_settings.visible && self.slice_plane_selection.has_selection {
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.slice_plane_selection.origin),
                glam::Vec3::from(self.slice_plane_selection.rotation_degrees),
                glam::Vec3::ONE, // No scale for slice planes
            );

            egui::Area::new(egui::Id::new("slice_plane_gizmo_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .interactable(false)
                .show(&egui.context, |ui| {
                    ui.set_clip_rect(full_window_viewport);

                    if pointer_over_ui {
                        return;
                    }

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.local_space,
                        full_window_viewport,
                    ) {
                        let (new_origin, rotation, _scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);

                        self.slice_plane_selection.origin = new_origin.into();
                        self.slice_plane_selection.rotation_degrees = rotation.into();

                        // Apply to selected slice plane
                        crate::apply_slice_plane_gizmo_transform(
                            self.slice_plane_selection.origin,
                            self.slice_plane_selection.rotation_degrees,
                        );

                        // Update UI settings to reflect new position
                        for settings in &mut self.slice_plane_settings {
                            if settings.name == self.slice_plane_selection.name {
                                settings.origin = self.slice_plane_selection.origin;
                                // Normal is derived from transform, update it
                                let rotation = glam::Quat::from_euler(
                                    glam::EulerRot::XYZ,
                                    self.slice_plane_selection.rotation_degrees[0].to_radians(),
                                    self.slice_plane_selection.rotation_degrees[1].to_radians(),
                                    self.slice_plane_selection.rotation_degrees[2].to_radians(),
                                );
                                let normal = rotation * glam::Vec3::X;
                                settings.normal = normal.to_array();
                                break;
                            }
                        }
                    }
                });
        }

        // Update background color if changed
        self.background_color = Vec3::new(bg_color[0], bg_color[1], bg_color[2]);

        // Update ground plane settings from UI
        self.ground_plane.mode = match gp_mode {
            0 => GroundPlaneMode::None,
            1 => GroundPlaneMode::Tile,
            2 => GroundPlaneMode::ShadowOnly,
            _ => GroundPlaneMode::TileReflection,
        };
        self.ground_plane.height = gp_height;
        self.ground_plane.height_is_relative = gp_height_is_relative;
        self.ground_plane.shadow_blur_iters = gp_shadow_blur_iters;
        self.ground_plane.shadow_darkness = gp_shadow_darkness;
        self.ground_plane.reflection_intensity = gp_reflection_intensity;

        // Apply camera settings if changed
        if camera_changed {
            crate::apply_camera_settings(&mut engine.camera, &self.camera_settings);
        }

        // Apply scene extents settings if changed
        if scene_extents_changed {
            crate::set_auto_compute_extents(self.scene_extents.auto_compute);
            // When auto-compute is OFF, sync manual edits back to context
            if !self.scene_extents.auto_compute {
                polyscope_core::state::with_context_mut(|ctx| {
                    ctx.length_scale = self.scene_extents.length_scale;
                    ctx.bounding_box = (
                        glam::Vec3::from_array(self.scene_extents.bbox_min),
                        glam::Vec3::from_array(self.scene_extents.bbox_max),
                    );
                });
            }
        }

        // Apply SSAA settings if changed
        if ssaa_changed && engine.ssaa_factor() != self.appearance_settings.ssaa_factor {
            engine.set_ssaa_factor(self.appearance_settings.ssaa_factor);
        }

        // Queue screenshot request from UI button (will be processed after render)
        if screenshot_requested {
            let filename = format!("screenshot_{:04}.png", self.screenshot_counter);
            self.screenshot_counter += 1;
            self.screenshot_pending = Some(filename);
        }

        // Reset camera to home view (matching C++ Polyscope's resetCameraToHomeView)
        if reset_view_requested {
            let bbox = crate::with_context(|ctx| ctx.bounding_box);
            let (min, max) = bbox;
            if min.x.is_finite() && max.x.is_finite() && (max - min).length() > 0.0 {
                engine.camera.look_at_box(min, max);
                engine.camera.fov = std::f32::consts::FRAC_PI_4; // Reset FOV to default 45°
            }
        }

        // End egui frame
        let egui_output = egui.end_frame(window);

        // Now borrow surface for rendering
        let surface = engine.surface.as_ref().expect("surface checked above");
        let output = match surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                engine.resize(engine.width, engine.height);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of memory");
                self.close_requested = true;
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Surface timeout");
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
                log::warn!("Surface error: other");
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        // HDR texture is always available for scene rendering
        // Update tone mapping uniforms - use passthrough values if disabled
        // Get SSAO settings from global options
        let ssao_enabled = polyscope_core::with_context(|ctx| ctx.options.ssao.enabled);
        engine.update_tone_mapping(
            self.tone_mapping_settings.exposure,
            self.tone_mapping_settings.white_level,
            self.tone_mapping_settings.gamma,
            ssao_enabled,
        );

        // Store background color for use in render passes
        let bg_r = f64::from(self.background_color.x);
        let bg_g = f64::from(self.background_color.y);
        let bg_b = f64::from(self.background_color.z);

        // Store ground plane settings for later use
        let gp_enabled = self.ground_plane.mode != GroundPlaneMode::None;
        let gp_height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        // Shadow mode: 0=none (disabled), 1=shadow_only, 2=tile_with_shadow
        let gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            // TileReflection also uses tile mode with shadows
            GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
        };

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
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                if let Some(render_data) = cn.render_data() {
                                    if let Some(compute_bg) = &render_data.compute_bind_group {
                                        compute_pass.set_bind_group(0, compute_bg, &[]);
                                        let num_workgroups = render_data.num_edges.div_ceil(64);
                                        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                                    }
                                }
                            }
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
            let light_dir = glam::Vec3::new(0.5, -1.0, 0.3).normalize();
            let light_matrix = polyscope_render::ShadowMapPass::compute_light_matrix(
                scene_center,
                scene_radius,
                light_dir,
            );

            // Update light uniforms
            shadow_map_pass.update_light(&engine.queue, light_matrix, light_dir);

            // Begin shadow pass
            {
                let mut shadow_pass = shadow_map_pass.begin_shadow_pass(&mut encoder);
                shadow_pass.set_pipeline(shadow_pipeline);

                // Render shadow-casting structures (SurfaceMesh only for now)
                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !ctx.is_structure_visible(structure) {
                            continue;
                        }
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(shadow_bg) = mesh.shadow_bind_group() {
                                    shadow_pass.set_bind_group(0, shadow_bg, &[]);
                                    if let Some(rd) = mesh.render_data() {
                                        shadow_pass.draw(0..rd.num_vertices(), 0..1);
                                    }
                                }
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
        engine.render_slice_planes_with_clear(
            &mut encoder,
            &slice_planes,
            length_scale_for_planes,
            [bg_r as f32, bg_g as f32, bg_b as f32],
        );

        let use_depth_peel = self.appearance_settings.transparency_mode == 2;

        // Render ground plane BEFORE surface mesh passes so transparent objects
        // composite correctly over the ground. Without this, either:
        // - Simple mode: meshes with no depth write get overwritten by later ground plane
        // - Pretty mode: mesh depth prepass blocks ground from rendering, making peeled
        //   transparent meshes appear gray (no ground behind them to show through)
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });

        if self.ground_plane.mode == GroundPlaneMode::TileReflection {
            // Compute ground height
            let ground_height = if self.ground_plane.height_is_relative {
                scene_min_y - length_scale * 0.001
            } else {
                self.ground_plane.height
            };

            // Update reflection uniforms
            let reflection_matrix = reflection::ground_reflection_matrix(ground_height);
            engine.update_reflection(
                reflection_matrix,
                self.ground_plane.reflection_intensity,
                ground_height,
            );

            // 1. Render stencil pass (mark ground plane region)
            engine.render_stencil_pass(
                &mut encoder,
                &view,
                ground_height,
                scene_center,
                length_scale,
            );

            // 2. Render ground plane FIRST (opaque base)
            engine.render_ground_plane(
                &mut encoder,
                &view,
                true, // enabled
                scene_center,
                scene_min_y,
                length_scale,
                gp_height_override,
                self.ground_plane.shadow_darkness,
                gp_shadow_mode,
                0.0, // No transparency - fully opaque ground
            );

            // 3. Render reflected meshes ON TOP of ground
            {
                let hdr_view = engine.hdr_texture_view().unwrap_or(&view);
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
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(mesh_data) = mesh.render_data() {
                                    if let Some(bind_group) =
                                        engine.create_reflected_mesh_bind_group(mesh_data)
                                    {
                                        engine.render_reflected_mesh(
                                            &mut render_pass,
                                            &bind_group,
                                            mesh_data.vertex_count(),
                                            structure.material(),
                                        );
                                    }
                                }
                            }
                        }
                        // VolumeMesh (uses SurfaceMeshRenderData)
                        if structure.type_name() == "VolumeMesh" {
                            if let Some(vol_mesh) =
                                structure.as_any().downcast_ref::<VolumeMesh>()
                            {
                                if let Some(mesh_data) = vol_mesh.render_data() {
                                    if let Some(bind_group) =
                                        engine.create_reflected_mesh_bind_group(mesh_data)
                                    {
                                        engine.render_reflected_mesh(
                                            &mut render_pass,
                                            &bind_group,
                                            mesh_data.vertex_count(),
                                            structure.material(),
                                        );
                                    }
                                }
                            }
                        }
                        // PointCloud
                        if structure.type_name() == "PointCloud" {
                            if let Some(pc) = structure.as_any().downcast_ref::<PointCloud>() {
                                if let Some(pc_data) = pc.render_data() {
                                    if let Some(bind_group) =
                                        engine.create_reflected_point_cloud_bind_group(pc_data)
                                    {
                                        engine.render_reflected_point_cloud(
                                            &mut render_pass,
                                            &bind_group,
                                            pc_data.num_points,
                                            structure.material(),
                                        );
                                    }
                                }
                            }
                        }
                        // CurveNetwork
                        if structure.type_name() == "CurveNetwork" {
                            if let Some(cn) = structure.as_any().downcast_ref::<CurveNetwork>() {
                                if let Some(cn_data) = cn.render_data() {
                                    if let Some(bind_group) =
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
                            }
                        }
                    }
                });
            }
        } else {
            // Non-reflection ground plane modes
            engine.render_ground_plane(
                &mut encoder,
                &view,
                gp_enabled,
                scene_center,
                scene_min_y,
                length_scale,
                gp_height_override,
                self.ground_plane.shadow_darkness,
                gp_shadow_mode,
                0.0,
            );
        }

        // Surface mesh depth prepass for Pretty mode (opaque meshes only)
        if use_depth_peel {
            if let Some(depth_pipeline) = engine.mesh_depth_normal_pipeline.as_ref() {
                let hdr_view = engine.hdr_view().expect("HDR view should be available");
                let normal_view = engine
                    .normal_view()
                    .expect("Normal view should be available");

                let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Surface Mesh Depth Prepass"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: hdr_view,
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
                        if structure.type_name() == "SurfaceMesh" {
                            if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() {
                                if let Some(render_data) = mesh.render_data() {
                                    prepass.set_bind_group(
                                        2,
                                        engine.matcap_bind_group_for(structure.material()),
                                        &[],
                                    );
                                    prepass.set_bind_group(0, &render_data.bind_group, &[]);
                                    prepass.set_index_buffer(
                                        render_data.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    prepass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                }
                            }
                        }
                    }
                });
            }
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
            if let Some(pipeline) = &engine.point_pipeline {
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

            // Draw vector quantities
            if let Some(pipeline) = &engine.vector_pipeline {
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

            // Note: Surface meshes and volume meshes are rendered in a separate pass
            // with MRT (multiple render targets) for SSAO normal output

            // Draw curve network edges (line mode) and camera views
            if let Some(pipeline) = &engine.curve_network_edge_pipeline {
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

            // Draw curve network tubes (tube mode)
            if let Some(pipeline) = &engine.curve_network_tube_pipeline {
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

            // Draw curve network node spheres (tube mode - fills gaps at joints)
            if let Some(pipeline) = &engine.point_pipeline {
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
        } // End of main render pass scope

        // Surface mesh render pass with MRT (HDR color + normal G-buffer for SSAO)
        if let Some(mesh_pipeline) = &engine.mesh_pipeline {
            let mesh_depth_pipeline = engine.mesh_depth_normal_pipeline.as_ref();
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
                if let Some(depth_pipeline) = mesh_depth_pipeline {
                    render_pass.set_pipeline(depth_pipeline);
                } else {
                    render_pass.set_pipeline(mesh_pipeline);
                }

                // Surface meshes: depth/normal only (color handled by depth peeling)
                // All surface meshes go through depth peeling for color, so we only
                // write depth+normals here for SSAO regardless of transparency.
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
                    }
                });

                // Volume meshes: full color/normal pass (not peeled)
                render_pass.set_pipeline(mesh_pipeline);
                crate::with_context(|ctx| {
                    for structure in ctx.registry.iter() {
                        if !ctx.is_structure_visible(structure) {
                            continue;
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
            } else {
                // Simple/None mode: render all surface meshes through the normal
                // pipeline (alpha blending with depth write). The ground plane has
                // already been rendered before this pass, so alpha blending correctly
                // composites transparent meshes over the ground. Depth write ensures
                // proper occlusion between meshes and prevents later passes from
                // overwriting mesh pixels. When alpha=1.0, this produces the same
                // visual result as fully opaque rendering.
                render_pass.set_pipeline(mesh_pipeline);
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

            // Draw volume grid isosurfaces (simple mesh pipeline, same MRT pass)
            if let Some(iso_pipeline) = &engine.simple_mesh_pipeline {
                render_pass.set_pipeline(iso_pipeline);
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

            // Draw volume grid gridcubes (gridcube pipeline, same MRT pass)
            if let Some(gc_pipeline) = &engine.gridcube_pipeline {
                render_pass.set_pipeline(gc_pipeline);
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
        }

        // Note: Ground plane is rendered earlier (before depth prepass and surface mesh pass)
        // so that transparent meshes can correctly composite over it.

        // Depth peeling transparency pass for surface meshes
        // All surface meshes go through depth peeling to handle overlapping geometry correctly
        if use_depth_peel {
            // Check if there are any surface meshes to render
            let has_surface_meshes = crate::with_context(|ctx| {
                ctx.registry
                    .iter()
                    .any(|s| s.is_enabled() && s.type_name() == "SurfaceMesh")
            });

            if has_surface_meshes {
                engine.ensure_depth_peel_pass();

                let num_passes = polyscope_core::with_context(|ctx| {
                    ctx.options.transparency_render_passes
                });

                // Clear final buffer to transparent black
                {
                    let peel = engine.depth_peel_pass().unwrap();
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
                }

                // Clear min-depth buffer to 0.0 (no depth peeled yet)
                {
                    let peel = engine.depth_peel_pass().unwrap();
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
                }

                for _i_pass in 0..num_passes {
                    // Peel pass: render all surface meshes, discarding already-peeled fragments
                    {
                        let peel = engine.depth_peel_pass().unwrap();
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
                                if structure.type_name() == "SurfaceMesh" {
                                    if let Some(mesh) =
                                        structure.as_any().downcast_ref::<SurfaceMesh>()
                                    {
                                        if let Some(render_data) = mesh.render_data() {
                                            peel_pass.set_bind_group(
                                                2,
                                                engine.matcap_bind_group_for(structure.material()),
                                                &[],
                                            );
                                            peel_pass
                                                .set_bind_group(0, &render_data.bind_group, &[]);
                                            peel_pass.set_index_buffer(
                                                render_data.index_buffer.slice(..),
                                                wgpu::IndexFormat::Uint32,
                                            );
                                            peel_pass.draw_indexed(
                                                0..render_data.num_indices,
                                                0,
                                                0..1,
                                            );
                                        }
                                    }
                                }
                            }
                        });
                    }

                    // Composite this peel layer into the final buffer (alpha-under)
                    {
                        let peel = engine.depth_peel_pass().unwrap();
                        peel.composite_layer(&mut encoder, &engine.device);
                    }

                    // Update min-depth from this peel's depth output (Max blend)
                    {
                        let peel = engine.depth_peel_pass().unwrap();
                        peel.update_min_depth(&mut encoder, &engine.device);
                    }
                }

                // Composite final peeled result onto the HDR scene
                {
                    let hdr_view = engine.hdr_view().expect("HDR view should be available");
                    let peel = engine.depth_peel_pass().unwrap();
                    peel.composite_final_to_scene(&mut encoder, &engine.device, hdr_view);
                }
            }
        }

        // Render SSAO if enabled
        if ssao_enabled {
            polyscope_core::with_context(|ctx| {
                engine.render_ssao(&mut encoder, &ctx.options.ssao);
            });
        }

        // Apply tone mapping from HDR to surface (always runs, uses passthrough if disabled)
        engine.render_tone_mapping(&mut encoder, &view);

        // Render egui on top (directly to surface, after tone mapping)
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [engine.width, engine.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        egui.render(
            &engine.device,
            &engine.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
            egui_output,
        );

        engine.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Handle screenshot if pending (local request from F12 key)
        if let Some(filename) = self.screenshot_pending.take() {
            self.capture_screenshot(filename);
        }

        // Handle screenshot request from public API (screenshot() / screenshot_to_file())
        if let Some(request) = crate::take_screenshot_request() {
            let filename = request.filename.unwrap_or_else(|| {
                let name = format!("screenshot_{:04}.png", self.screenshot_counter);
                self.screenshot_counter += 1;
                name
            });
            self.capture_screenshot(filename);
        }
    }

    /// Captures a screenshot by re-rendering to a dedicated texture.
    pub(super) fn capture_screenshot(&mut self, filename: String) {
        let Some(engine) = &mut self.engine else {
            log::error!("Cannot capture screenshot: engine not initialized");
            return;
        };

        // Create screenshot target
        let screenshot_view = engine.create_screenshot_target();

        // Re-render to screenshot texture
        let mut encoder = engine
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screenshot_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.background_color.x),
                            g: f64::from(self.background_color.y),
                            b: f64::from(self.background_color.z),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: engine.screenshot_depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw point clouds
            if let Some(pipeline) = &engine.point_pipeline {
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
                                    render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..6, 0..render_data.num_points);
                                }
                            }
                        }
                    }
                });
            }

            // Draw vector quantities
            if let Some(pipeline) = &engine.vector_pipeline {
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

            // Draw surface meshes and volume meshes
            if let Some(pipeline) = &engine.mesh_pipeline {
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
                                    render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.set_index_buffer(
                                        render_data.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "VolumeMesh" {
                            if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                                if let Some(render_data) = vm.render_data() {
                                    render_pass.set_bind_group(2, engine.matcap_bind_group_for(structure.material()), &[]);
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.set_index_buffer(
                                        render_data.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
                                }
                            }
                        }
                    }
                });
            }

            // Draw curve networks, camera views, and volume grids
            if let Some(pipeline) = &engine.curve_network_edge_pipeline {
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
                                if let Some(render_data) = cn.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "CameraView" {
                            if let Some(cv) = structure.as_any().downcast_ref::<CameraView>() {
                                if let Some(render_data) = cv.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                        if structure.type_name() == "VolumeGrid" {
                            if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                                if let Some(render_data) = vg.render_data() {
                                    render_pass.set_bind_group(0, &render_data.bind_group, &[]);
                                    render_pass.draw(0..render_data.num_edges * 2, 0..1);
                                }
                            }
                        }
                    }
                });
            }
        }

        // Render ground plane for screenshot
        let (scene_center, scene_min_y, length_scale) = crate::with_context(|ctx| {
            let center = ctx.center();
            (
                [center.x, center.y, center.z],
                ctx.bounding_box.0.y,
                ctx.length_scale,
            )
        });
        let height_override = if self.ground_plane.height_is_relative {
            None
        } else {
            Some(self.ground_plane.height)
        };
        let screenshot_gp_shadow_mode = match self.ground_plane.mode {
            GroundPlaneMode::None => 0u32,
            GroundPlaneMode::ShadowOnly => 1u32,
            GroundPlaneMode::Tile | GroundPlaneMode::TileReflection => 2u32,
        };
        let screenshot_reflection_intensity =
            if self.ground_plane.mode == GroundPlaneMode::TileReflection {
                self.ground_plane.reflection_intensity
            } else {
                0.0
            };
        engine.render_ground_plane(
            &mut encoder,
            &screenshot_view,
            self.ground_plane.mode != GroundPlaneMode::None,
            scene_center,
            scene_min_y,
            length_scale,
            height_override,
            self.ground_plane.shadow_darkness,
            screenshot_gp_shadow_mode,
            screenshot_reflection_intensity,
        );

        // Apply tone mapping from HDR to final screenshot texture
        engine.apply_screenshot_tone_mapping(&mut encoder);

        engine.queue.submit(std::iter::once(encoder.finish()));

        // Capture the screenshot
        match engine.capture_screenshot() {
            Ok(data) => {
                let (width, height) = engine.dimensions();
                match polyscope_render::save_image(&filename, &data, width, height) {
                    Ok(()) => {
                        log::info!("Screenshot saved to {filename}");
                    }
                    Err(e) => {
                        log::error!("Failed to save screenshot: {e}");
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to capture screenshot: {e}");
            }
        }
    }
}
