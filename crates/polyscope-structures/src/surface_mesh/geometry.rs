//! Geometry computation methods for surface meshes.
//!
//! This module contains methods for computing derived mesh data from raw vertices and faces:
//! - Triangulation (fan triangulation for arbitrary polygons)
//! - Face and vertex normals
//! - Corner normals for shading
//! - Edge extraction and classification for wireframe rendering
//! - Tangent basis computation for intrinsic vectors

use glam::Vec3;
use std::collections::HashSet;

use super::{ShadeStyle, SurfaceMesh};

impl SurfaceMesh {
    // === Computation methods ===

    /// Recomputes all derived data (triangulation, normals, edges).
    pub(super) fn recompute(&mut self) {
        if !self.needs_recompute {
            return;
        }

        self.compute_triangulation();
        self.compute_face_normals();
        self.compute_vertex_normals();
        self.compute_corner_normals();
        self.compute_edges();
        self.compute_edge_is_real();

        self.needs_recompute = false;
    }

    /// Computes triangulation using fan triangulation.
    ///
    /// For a polygon with vertices [v0, v1, v2, v3, ...], creates triangles:
    /// [v0, v1, v2], [v0, v2, v3], [v0, v3, v4], ...
    fn compute_triangulation(&mut self) {
        self.triangulation.clear();
        self.face_to_tri_range.clear();

        for face in &self.faces {
            let start_tri = self.triangulation.len();

            if face.len() >= 3 {
                let v0 = face[0];
                // Fan triangulation: create (n-2) triangles for n-gon
                for i in 1..(face.len() - 1) {
                    self.triangulation.push([v0, face[i], face[i + 1]]);
                }
            }

            let end_tri = self.triangulation.len();
            self.face_to_tri_range.push(start_tri..end_tri);
        }
    }

    /// Computes face normals using cross product of first two edges.
    fn compute_face_normals(&mut self) {
        self.face_normals.clear();
        self.face_normals.reserve(self.faces.len());

        for face in &self.faces {
            if face.len() >= 3 {
                let v0 = self.vertices[face[0] as usize];
                let v1 = self.vertices[face[1] as usize];
                let v2 = self.vertices[face[2] as usize];

                let e1 = v1 - v0;
                let e2 = v2 - v0;
                let normal = e1.cross(e2).normalize_or_zero();
                self.face_normals.push(normal);
            } else {
                self.face_normals.push(Vec3::ZERO);
            }
        }
    }

    /// Computes vertex normals as area-weighted average of incident face normals.
    fn compute_vertex_normals(&mut self) {
        self.vertex_normals.clear();
        self.vertex_normals.resize(self.vertices.len(), Vec3::ZERO);

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                continue;
            }

            let face_normal = self.face_normals[face_idx];

            // Compute face area using triangulation
            let v0 = self.vertices[face[0] as usize];
            let mut area = 0.0;
            for i in 1..(face.len() - 1) {
                let v1 = self.vertices[face[i] as usize];
                let v2 = self.vertices[face[i + 1] as usize];
                let e1 = v1 - v0;
                let e2 = v2 - v0;
                area += e1.cross(e2).length() * 0.5;
            }

            // Add weighted normal to each vertex of this face
            let weighted_normal = face_normal * area;
            for &vi in face {
                self.vertex_normals[vi as usize] += weighted_normal;
            }
        }

        // Normalize all vertex normals
        for normal in &mut self.vertex_normals {
            *normal = normal.normalize_or_zero();
        }
    }

    /// Computes corner normals (per-corner of each triangle).
    pub(super) fn compute_corner_normals(&mut self) {
        self.corner_normals.clear();
        self.corner_normals.reserve(self.triangulation.len() * 3);

        for (face_idx, range) in self.face_to_tri_range.iter().enumerate() {
            let face_normal = self.face_normals[face_idx];

            for tri_idx in range.clone() {
                let tri = self.triangulation[tri_idx];
                for vi in tri {
                    // For tri-flat, we use face normals; for smooth, we use vertex normals
                    // Store both options - the shader will choose based on shade_style
                    match self.shade_style {
                        ShadeStyle::Smooth => {
                            self.corner_normals.push(self.vertex_normals[vi as usize]);
                        }
                        ShadeStyle::Flat | ShadeStyle::TriFlat => {
                            self.corner_normals.push(face_normal);
                        }
                    }
                }
            }
        }
    }

    /// Computes `edge_is_real` flags for wireframe rendering.
    /// Marks which edges in the triangulation are real polygon edges vs internal.
    fn compute_edge_is_real(&mut self) {
        self.edge_is_real.clear();
        self.edge_is_real.reserve(self.triangulation.len() * 3);

        for range in &self.face_to_tri_range {
            let num_tris = range.end - range.start;

            for (j, _tri_idx) in range.clone().enumerate() {
                // For fan triangulation from v0:
                // Triangle j has vertices [v0, v_{j+1}, v_{j+2}]
                // Edge 0 (v0 -> v_{j+1}): real only if j == 0
                // Edge 1 (v_{j+1} -> v_{j+2}): always real (it's a polygon edge)
                // Edge 2 (v_{j+2} -> v0): real only if j == num_tris - 1

                let edge0_real = if j == 0 { 1.0 } else { 0.0 };
                let edge1_real = 1.0; // middle edge always real
                let edge2_real = if j == num_tris - 1 { 1.0 } else { 0.0 };

                // Each triangle corner gets the edge_is_real for all three edges
                // This matches C++ Polyscope's approach
                let edge_real = Vec3::new(edge0_real, edge1_real, edge2_real);
                self.edge_is_real.push(edge_real);
                self.edge_is_real.push(edge_real);
                self.edge_is_real.push(edge_real);
            }
        }
    }

    /// Computes unique edges as sorted pairs.
    fn compute_edges(&mut self) {
        let mut edge_set: HashSet<(u32, u32)> = HashSet::new();

        for face in &self.faces {
            let n = face.len();
            for i in 0..n {
                let v0 = face[i];
                let v1 = face[(i + 1) % n];
                // Store as sorted pair to avoid duplicates
                let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };
                edge_set.insert(edge);
            }
        }

        self.edges = edge_set.into_iter().collect();
        self.edges.sort_unstable(); // Sort for deterministic ordering
    }

    /// Compute default per-face tangent basis from first edge direction.
    #[must_use]
    pub fn compute_face_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut basis_x = Vec::with_capacity(self.faces.len());
        let mut basis_y = Vec::with_capacity(self.faces.len());

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() >= 3 {
                let v0 = self.vertices[face[0] as usize];
                let v1 = self.vertices[face[1] as usize];
                let normal = self.face_normals[face_idx];

                let bx = (v1 - v0).normalize_or_zero();
                let by = normal.cross(bx).normalize_or_zero();
                basis_x.push(bx);
                basis_y.push(by);
            } else {
                basis_x.push(Vec3::X);
                basis_y.push(Vec3::Y);
            }
        }

        (basis_x, basis_y)
    }

    /// Compute default per-vertex tangent basis from area-weighted face bases.
    #[must_use]
    pub fn compute_vertex_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let (face_bx, _face_by) = self.compute_face_tangent_basis();

        let mut vert_bx = vec![Vec3::ZERO; self.vertices.len()];

        for (face_idx, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                continue;
            }

            // Compute face area
            let v0 = self.vertices[face[0] as usize];
            let mut area = 0.0f32;
            for i in 1..(face.len() - 1) {
                let v1 = self.vertices[face[i] as usize];
                let v2 = self.vertices[face[i + 1] as usize];
                area += (v1 - v0).cross(v2 - v0).length() * 0.5;
            }

            let weighted_bx = face_bx[face_idx] * area;
            for &vi in face {
                vert_bx[vi as usize] += weighted_bx;
            }
        }

        // Orthonormalize against vertex normals
        let mut basis_x = Vec::with_capacity(self.vertices.len());
        let mut basis_y = Vec::with_capacity(self.vertices.len());

        for (i, normal) in self.vertex_normals.iter().enumerate() {
            let mut bx = vert_bx[i];
            // Project out normal component and normalize
            bx -= *normal * normal.dot(bx);
            bx = bx.normalize_or_zero();

            // If degenerate, pick an arbitrary tangent
            if bx.length_squared() < 1e-6 {
                bx = if normal.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                bx -= *normal * normal.dot(bx);
                bx = bx.normalize_or_zero();
            }

            let by = normal.cross(bx).normalize_or_zero();
            basis_x.push(bx);
            basis_y.push(by);
        }

        (basis_x, basis_y)
    }
}
