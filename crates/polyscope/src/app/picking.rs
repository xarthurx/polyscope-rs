use super::{App, CurveNetwork, PointCloud, SurfaceMesh, Vec3, VolumeMesh};

impl App {
    /// Performs GPU-based picking to find which structure and element is at the given screen position.
    ///
    /// Uses the GPU pick buffer to determine the exact structure and element at the click position.
    /// Returns (`type_name`, name, `element_index`) or None if clicking on empty space.
    pub(super) fn gpu_pick_at(&self, x: u32, y: u32) -> Option<(String, String, u32)> {
        let engine = self.engine.as_ref()?;

        // Read pick buffer — returns flat 24-bit global index
        let global_index = engine.pick_at(x, y)?;

        // Background check (index 0 means nothing was hit)
        if global_index == 0 {
            return None;
        }

        // Look up structure info from global index
        let (type_name, name, local_index) = engine.lookup_global_index(global_index)?;
        Some((type_name.to_string(), name.to_string(), local_index))
    }

    pub(super) fn screen_ray(
        &self,
        click_pos: glam::Vec2,
        screen_width: u32,
        screen_height: u32,
        camera: &polyscope_render::Camera,
    ) -> Option<(Vec3, Vec3)> {
        if screen_width == 0 || screen_height == 0 {
            return None;
        }

        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;
        let ndc_x = (click_pos.x / half_width) - 1.0;
        let ndc_y = 1.0 - (click_pos.y / half_height);

        let inv_view_proj = camera.view_projection_matrix().inverse();

        // wgpu-style NDC depth [0, 1]
        let near = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far = inv_view_proj * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        if near.w.abs() < 1e-6 || far.w.abs() < 1e-6 {
            return None;
        }

        let ray_origin = near.truncate() / near.w;
        let ray_far = far.truncate() / far.w;
        let ray_dir = (ray_far - ray_origin).normalize_or_zero();
        if ray_dir.length_squared() < 1e-12 {
            return None;
        }

        Some((ray_origin, ray_dir))
    }

    /// Tests whether a ray intersects a visible slice plane quad.
    pub(super) fn pick_slice_plane_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
    ) -> Option<(String, f32)> {
        let mut best_hit: Option<(String, f32)> = None;
        crate::with_context(|ctx| {
            for plane in ctx.slice_planes() {
                if !plane.is_enabled() || !plane.draw_plane() {
                    continue;
                }

                let normal = plane.normal();
                let denom = normal.dot(ray_dir);
                if denom.abs() < 1e-6 {
                    continue;
                }

                let t = (plane.origin() - ray_origin).dot(normal) / denom;
                if t < 0.0 {
                    continue;
                }

                let hit = ray_origin + ray_dir * t;

                // Compute local plane axes (match visualization orientation)
                let up = if normal.dot(Vec3::Y).abs() < 0.99 {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let y_axis = up.cross(normal).normalize();
                let z_axis = normal.cross(y_axis).normalize();

                let local = hit - plane.origin();
                let y = local.dot(y_axis);
                let z = local.dot(z_axis);
                let size = plane.plane_size();

                if y.abs() <= size && z.abs() <= size {
                    let is_better = best_hit.as_ref().is_none_or(|(_, best_t)| t < *best_t);
                    if is_better {
                        best_hit = Some((plane.name().to_string(), t));
                    }
                }
            }
        });

        best_hit
    }

    pub(super) fn ray_intersect_triangle(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
    ) -> Option<f32> {
        let eps = 1e-6;
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = ray_dir.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < eps {
            return None;
        }
        let f = 1.0 / a;
        let s = ray_origin - v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * ray_dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);
        if t > eps { Some(t) } else { None }
    }

    pub(super) fn ray_segment_closest_t(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        a: Vec3,
        b: Vec3,
    ) -> Option<f32> {
        let v = b - a;
        let c = v.dot(v);
        if c < 1e-12 {
            let t = ray_dir.dot(a - ray_origin);
            return (t >= 0.0).then_some(t);
        }

        let w0 = ray_origin - a;
        let a_dot = ray_dir.dot(ray_dir);
        let b_dot = ray_dir.dot(v);
        let d = ray_dir.dot(w0);
        let e = v.dot(w0);
        let denom = a_dot * c - b_dot * b_dot;

        let s;
        let mut t;
        if denom.abs() < 1e-8 {
            s = 0.0;
            t = ray_dir.dot(a - ray_origin);
        } else {
            s = (b_dot * d - a_dot * e) / denom;
            t = (b_dot * e - c * d) / denom;
        }

        if s < 0.0 {
            t = ray_dir.dot(a - ray_origin);
        } else if s > 1.0 {
            t = ray_dir.dot(b - ray_origin);
        }

        (t >= 0.0).then_some(t)
    }

    pub(super) fn pick_structure_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        plane_params: &[(Vec3, Vec3)],
    ) -> Option<(String, String, f32)> {
        let mut best_hit: Option<(String, String, f32)> = None;

        crate::with_context(|ctx| {
            for structure in ctx.registry.iter() {
                if !ctx.is_structure_visible(structure) {
                    continue;
                }

                match structure.type_name() {
                    "SurfaceMesh" => {
                        let Some(mesh) = structure.as_any().downcast_ref::<SurfaceMesh>() else {
                            continue;
                        };
                        let model = structure.transform();
                        let mut world_verts = Vec::with_capacity(mesh.vertices().len());
                        for v in mesh.vertices() {
                            world_verts.push((model * v.extend(1.0)).truncate());
                        }

                        let mut hit_t: Option<f32> = None;
                        for tri in mesh.triangulation() {
                            let v0 = world_verts[tri[0] as usize];
                            let v1 = world_verts[tri[1] as usize];
                            let v2 = world_verts[tri[2] as usize];
                            if let Some(t) =
                                self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2)
                            {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better =
                                best_hit.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit = Some((
                                    structure.type_name().to_string(),
                                    structure.name().to_string(),
                                    t,
                                ));
                            }
                        }
                    }
                    "VolumeMesh" => {
                        let Some(vm) = structure.as_any().downcast_ref::<VolumeMesh>() else {
                            continue;
                        };
                        let model = structure.transform();
                        let (positions, faces) = vm.pick_triangles(plane_params);
                        if positions.is_empty() || faces.is_empty() {
                            continue;
                        }
                        let mut world_positions = Vec::with_capacity(positions.len());
                        for v in positions {
                            world_positions.push((model * v.extend(1.0)).truncate());
                        }

                        let mut hit_t: Option<f32> = None;
                        for tri in faces {
                            let v0 = world_positions[tri[0] as usize];
                            let v1 = world_positions[tri[1] as usize];
                            let v2 = world_positions[tri[2] as usize];
                            if let Some(t) =
                                self.ray_intersect_triangle(ray_origin, ray_dir, v0, v1, v2)
                            {
                                hit_t = Some(hit_t.map_or(t, |best| best.min(t)));
                            }
                        }

                        if let Some(t) = hit_t {
                            let is_better =
                                best_hit.as_ref().is_none_or(|(_, _, best_t)| t < *best_t);
                            if is_better {
                                best_hit = Some((
                                    structure.type_name().to_string(),
                                    structure.name().to_string(),
                                    t,
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        best_hit
    }

    pub(super) fn pick_point_cloud_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        name: &str,
        element_index: u32,
    ) -> Option<f32> {
        crate::with_context(|ctx| {
            let structure = ctx.registry.get("PointCloud", name)?;
            let pc = structure.as_any().downcast_ref::<PointCloud>()?;
            let points = pc.points();
            let idx = element_index as usize;
            if idx >= points.len() {
                return None;
            }
            let model = structure.transform();
            let world_point = (model * points[idx].extend(1.0)).truncate();
            let t = (world_point - ray_origin).dot(ray_dir);
            if t < 0.0 {
                return None;
            }
            let closest = ray_origin + ray_dir * t;
            let dist = (world_point - closest).length();
            let radius_world = model
                .transform_vector3(Vec3::new(pc.point_radius(), 0.0, 0.0))
                .length();
            if dist <= radius_world.max(1e-4) * 1.5 {
                Some(t)
            } else {
                None
            }
        })
    }

    pub(super) fn pick_curve_network_edge_at_ray(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        name: &str,
        element_index: u32,
    ) -> Option<f32> {
        crate::with_context(|ctx| {
            let structure = ctx.registry.get("CurveNetwork", name)?;
            let cn = structure.as_any().downcast_ref::<CurveNetwork>()?;
            let edge_idx = element_index as usize;
            if edge_idx >= cn.edge_tail_inds().len() {
                return None;
            }
            let tail_idx = cn.edge_tail_inds()[edge_idx] as usize;
            let tip_idx = cn.edge_tip_inds()[edge_idx] as usize;
            if tail_idx >= cn.nodes().len() || tip_idx >= cn.nodes().len() {
                return None;
            }

            let model = structure.transform();
            let tail = (model * cn.nodes()[tail_idx].extend(1.0)).truncate();
            let tip = (model * cn.nodes()[tip_idx].extend(1.0)).truncate();

            self.ray_segment_closest_t(ray_origin, ray_dir, tail, tip)
        })
    }

    pub(super) fn select_slice_plane_by_name(&mut self, name: &str) {
        let mut selected_settings: Option<polyscope_ui::SlicePlaneSettings> = None;
        for settings in &mut self.slice_plane_settings {
            if settings.name == name {
                settings.is_selected = true;
                settings.draw_widget = true;
                selected_settings = Some(settings.clone());
            } else {
                settings.is_selected = false;
            }
        }
        crate::select_slice_plane_for_gizmo(name);
        if let Some(settings) = selected_settings {
            crate::apply_slice_plane_settings(&settings);
        }
    }

    pub(super) fn deselect_slice_plane_selection(&mut self) {
        for settings in &mut self.slice_plane_settings {
            settings.is_selected = false;
        }
        crate::deselect_slice_plane_gizmo();
    }
}
