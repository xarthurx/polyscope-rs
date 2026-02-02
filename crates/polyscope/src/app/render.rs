use super::{
    App, CurveNetwork, GroundPlaneMode, PointCloud, ScreenDescriptor, Structure, SurfaceMesh, Vec3,
    VolumeGrid, VolumeMesh, reflection, render_scene,
};
use polyscope_core::quantity::Quantity;
use polyscope_core::structure::HasQuantities;
use polyscope_render::{
    GridcubeRenderData, GridcubeUniforms, IsosurfaceRenderData, SimpleMeshUniforms,
};
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

impl App {
    /// Renders a single frame.
    pub(super) fn render(&mut self) {
        let (Some(engine), Some(_egui), Some(_window)) =
            (&mut self.engine, &mut self.egui, &self.window)
        else {
            return;
        };

        // Check surface exists (but don't hold borrow yet - needed for structure ID assignment)
        if engine.surface.is_none() {
            return;
        }

        // Auto-fit camera to scene on first render with structures
        self.camera_fitted = super::render_init::auto_fit_camera(engine, self.camera_fitted);

        // Update camera flight animation (before uniforms so interpolated position is used)
        engine.camera.update_flight();

        // Drain deferred material load queue
        super::render_init::drain_material_queue(engine);

        // Update camera and slice plane uniforms
        super::render_init::update_uniforms(engine);

        // Initialize GPU resources for structures (shared between windowed and headless)
        super::render_init::init_structure_gpu_resources(engine);

        // Initialize windowed-specific GPU resources (pick resources and VolumeGrid quantities)
        // Collect deferred mesh registrations (from "Register as Surface Mesh" button)
        let mut meshes_to_register: Vec<(String, Vec<Vec3>, Vec<[u32; 3]>)> = Vec::new();
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
                if structure.type_name() == "SurfaceMesh" {
                    if let Some(mesh) = structure.as_any_mut().downcast_mut::<SurfaceMesh>() {
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
                }

                // CurveNetwork: windowed-only pick resources (edge and tube)
                if structure.type_name() == "CurveNetwork" {
                    if let Some(cn) = structure.as_any_mut().downcast_mut::<CurveNetwork>() {
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
                }

                // CameraView init moved to shared function

                // VolumeGrid: windowed-only quantity initialization (gridcube/isosurface)
                if structure.type_name() == "VolumeGrid" {
                    if let Some(vg) = structure.as_any_mut().downcast_mut::<VolumeGrid>() {
                        // Base wireframe render data init moved to shared function
                        // Initialize GPU resources for enabled scalar quantities (windowed-only)
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
                                        if nsq.gridcube_render_data().is_none()
                                            || nsq.gridcube_dirty()
                                        {
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
                                                        let t =
                                                            Vec3::new(i as f32, j as f32, k as f32)
                                                                / cell_dim_f;
                                                        centers.push(
                                                            bound_min + t * (bound_max - bound_min),
                                                        );
                                                    }
                                                }
                                            }
                                            let half_size = grid_spacing.min_element()
                                                * 0.5
                                                * cube_size_factor.max(0.5);

                                            // Sample colormap
                                            let colormap_colors: Vec<Vec3> = if let Some(cm) =
                                                engine.color_maps.get(nsq.color_map())
                                            {
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
                                        if nsq.isosurface_render_data().is_none()
                                            || nsq.isosurface_dirty()
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
                            if let Some(nsq) = quantity
                                .as_any_mut()
                                .downcast_mut::<VolumeGridNodeScalarQuantity>()
                            {
                                if nsq.register_as_mesh_requested() {
                                    if let Some(mesh) = nsq.isosurface_mesh() {
                                        let verts = mesh.vertices.clone();
                                        let tris: Vec<[u32; 3]> = mesh
                                            .indices
                                            .chunks(3)
                                            .map(|c| [c[0], c[1], c[2]])
                                            .collect();
                                        let name = format!("{} isosurface", nsq.name());
                                        meshes_to_register.push((name, verts, tris));
                                    }
                                    nsq.clear_register_as_mesh_request();
                                }
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
                                    let half_size = cell_spacing.min_element()
                                        * 0.5
                                        * cube_size_factor.max(0.5);

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

                        // --- VolumeGrid pick initialization ---
                        // Init gridcube pick pipeline if needed
                        if !engine.has_gridcube_pick_pipeline() {
                            // Only init if there are any enabled gridcube quantities
                            let has_enabled_gridcube = vg.quantities().iter().any(|q| {
                                if let Some(nsq) =
                                    q.as_any().downcast_ref::<VolumeGridNodeScalarQuantity>()
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
                }

                // VolumeMesh: windowed-only pick resources
                // (slice plane culling logic moved to shared function)
                if structure.type_name() == "VolumeMesh" {
                    if let Some(vm) = structure.as_any_mut().downcast_mut::<VolumeMesh>() {
                        // Initialize pick resources (after render data init by shared function)
                        if vm.pick_bind_group().is_none() && vm.render_data().is_some() {
                            if !engine.has_mesh_pick_pipeline() {
                                engine.init_mesh_pick_pipeline();
                            }
                            let num_cells = vm.num_cells() as u32;
                            let global_start =
                                engine.assign_pick_range("VolumeMesh", vm.name(), num_cells);
                            vm.init_pick_resources(
                                &engine.device,
                                engine.mesh_pick_bind_group_layout(),
                                engine.camera_buffer(),
                                global_start,
                            );
                        }
                    }
                }
            }
        });

        // Update GPU buffers for all structures (shared function, with pick uniforms for windowed)
        super::render_init::update_gpu_buffers(engine, true);

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
                                    pick_pass
                                        .set_pipeline(engine.curve_network_tube_pick_pipeline());
                                    pick_pass.set_bind_group(
                                        0,
                                        cn.tube_pick_bind_group().unwrap(),
                                        &[],
                                    );
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
                                        pick_pass
                                            .set_pipeline(engine.curve_network_pick_pipeline());
                                        pick_pass.set_bind_group(0, pick_bind_group, &[]);
                                        // 2 vertices per edge (LineList topology)
                                        pick_pass.draw(0..render_data.num_edges * 2, 0..1);
                                    }
                                }
                            }
                        }
                    }
                });

                // Draw surface meshes and volume meshes to pick buffer
                // (both use the same mesh pick pipeline with face/cell index mapping)
                if engine.has_mesh_pick_pipeline() {
                    pick_pass.set_pipeline(engine.mesh_pick_pipeline());
                    crate::with_context(|ctx| {
                        for structure in ctx.registry.iter() {
                            if !ctx.is_structure_visible(structure) {
                                continue;
                            }
                            if structure.type_name() == "SurfaceMesh" {
                                if let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>()
                                {
                                    if let Some(pick_bind_group) = mesh.pick_bind_group() {
                                        pick_pass.set_bind_group(0, pick_bind_group, &[]);
                                        pick_pass.draw(0..mesh.num_triangulation_vertices(), 0..1);
                                    }
                                }
                            }
                            if structure.type_name() == "VolumeMesh" {
                                if let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() {
                                    if let Some(pick_bind_group) = vm.pick_bind_group() {
                                        pick_pass.set_bind_group(0, pick_bind_group, &[]);
                                        pick_pass.draw(0..vm.num_render_vertices(), 0..1);
                                    }
                                }
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
                            if structure.type_name() == "VolumeGrid" {
                                if let Some(vg) = structure.as_any().downcast_ref::<VolumeGrid>() {
                                    for quantity in vg.quantities() {
                                        if !quantity.is_enabled() {
                                            continue;
                                        }
                                        if let Some(nsq) = quantity
                                            .as_any()
                                            .downcast_ref::<VolumeGridNodeScalarQuantity>(
                                        ) {
                                            if nsq.viz_mode() == VolumeGridVizMode::Gridcube {
                                                if let Some(pick_bg) = nsq.pick_bind_group() {
                                                    pick_pass.set_bind_group(0, pick_bg, &[]);
                                                    pick_pass
                                                        .draw(0..nsq.pick_total_vertices(), 0..1);
                                                }
                                            }
                                        }
                                        if let Some(csq) = quantity
                                            .as_any()
                                            .downcast_ref::<VolumeGridCellScalarQuantity>(
                                        ) {
                                            if let Some(pick_bg) = csq.pick_bind_group() {
                                                pick_pass.set_bind_group(0, pick_bg, &[]);
                                                pick_pass.draw(0..csq.pick_total_vertices(), 0..1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }

            engine.queue.submit(std::iter::once(encoder.finish()));
        }

        // Build UI (take engine/egui temporarily to satisfy borrow checker)
        let mut engine_temp = self.engine.take().unwrap();
        let mut egui_temp = self.egui.take().unwrap();
        let window_temp = self.window.clone().unwrap();

        let ui_result = self.build_ui(&mut engine_temp, &mut egui_temp, &window_temp);

        self.engine = Some(engine_temp);
        self.egui = Some(egui_temp);

        let engine = self.engine.as_mut().unwrap();
        let egui = self.egui.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();

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
                            if let Some(vol_mesh) = structure.as_any().downcast_ref::<VolumeMesh>()
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
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
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
                                    render_pass.draw_indexed(0..render_data.num_indices, 0, 0..1);
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
            // Check if there are any surface meshes to render
            let has_surface_meshes = crate::with_context(|ctx| {
                ctx.registry
                    .iter()
                    .any(|s| s.is_enabled() && s.type_name() == "SurfaceMesh")
            });

            if has_surface_meshes {
                engine.ensure_depth_peel_pass();

                let num_passes =
                    polyscope_core::with_context(|ctx| ctx.options.transparency_render_passes);

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
                        let mut peel_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: peel.peel_depth_view(),
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(1.0),
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
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
                                            peel_pass.set_bind_group(
                                                0,
                                                &render_data.bind_group,
                                                &[],
                                            );
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
            ui_result.egui_output,
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
}
