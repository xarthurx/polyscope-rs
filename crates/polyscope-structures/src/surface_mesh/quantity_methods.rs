//! Quantity management methods for surface meshes.
//!
//! This module contains methods for adding and accessing quantities on surface meshes:
//! - Scalar quantities (vertex and face)
//! - Color quantities (vertex and face, RGB and RGBA)
//! - Vector quantities (vertex and face)
//! - Parameterization quantities (vertex and corner UV)
//! - Intrinsic vector quantities (vertex and face, with tangent basis)
//! - One-form quantities (edge-based differential forms)

use glam::{Vec2, Vec3, Vec4};
use polyscope_core::quantity::QuantityKind;
use polyscope_core::structure::{HasQuantities, Structure};

use super::{
    MeshCornerParameterizationQuantity, MeshFaceColorQuantity, MeshFaceIntrinsicVectorQuantity,
    MeshFaceScalarQuantity, MeshFaceVectorQuantity, MeshOneFormQuantity, MeshVertexColorQuantity,
    MeshVertexIntrinsicVectorQuantity, MeshVertexParameterizationQuantity,
    MeshVertexScalarQuantity, MeshVertexVectorQuantity, SurfaceMesh,
};

impl SurfaceMesh {
    // === Quantity add methods ===

    /// Adds a vertex scalar quantity to this mesh.
    pub fn add_vertex_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = MeshVertexScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face scalar quantity to this mesh.
    pub fn add_face_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let quantity = MeshFaceScalarQuantity::new(name, self.name.clone(), values);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex color quantity to this mesh (RGB, alpha defaults to 1.0).
    pub fn add_vertex_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshVertexColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex color quantity with explicit per-vertex RGBA alpha values.
    pub fn add_vertex_color_quantity_with_alpha(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> &mut Self {
        let quantity = MeshVertexColorQuantity::new_with_alpha(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face color quantity to this mesh (RGB, alpha defaults to 1.0).
    pub fn add_face_color_quantity(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> &mut Self {
        let quantity = MeshFaceColorQuantity::new(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face color quantity with explicit per-face RGBA alpha values.
    pub fn add_face_color_quantity_with_alpha(
        &mut self,
        name: impl Into<String>,
        colors: Vec<Vec4>,
    ) -> &mut Self {
        let quantity = MeshFaceColorQuantity::new_with_alpha(name, self.name.clone(), colors);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex vector quantity to this mesh.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_vertex_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshVertexVectorQuantity::new(name, self.name.clone(), vectors);
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face vector quantity to this mesh.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_face_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshFaceVectorQuantity::new(name, self.name.clone(), vectors);
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex parameterization (UV) quantity to this mesh.
    pub fn add_vertex_parameterization_quantity(
        &mut self,
        name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> &mut Self {
        let quantity = MeshVertexParameterizationQuantity::new(name, self.name.clone(), coords);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a corner parameterization (UV) quantity to this mesh.
    pub fn add_corner_parameterization_quantity(
        &mut self,
        name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> &mut Self {
        let quantity = MeshCornerParameterizationQuantity::new(name, self.name.clone(), coords);
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex intrinsic vector quantity with explicit tangent basis.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_vertex_intrinsic_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshVertexIntrinsicVectorQuantity::new(
            name,
            self.name.clone(),
            vectors,
            basis_x,
            basis_y,
        );
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a vertex intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_vertex_intrinsic_vector_quantity_auto(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
    ) -> &mut Self {
        let (bx, by) = self.compute_vertex_tangent_basis();
        self.add_vertex_intrinsic_vector_quantity(name, vectors, bx, by)
    }

    /// Adds a face intrinsic vector quantity with explicit tangent basis.
    ///
    /// Arrow length and radius are auto-scaled based on mesh extent and vector magnitudes.
    pub fn add_face_intrinsic_vector_quantity(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &mut Self {
        let mut quantity = MeshFaceIntrinsicVectorQuantity::new(
            name,
            self.name.clone(),
            vectors,
            basis_x,
            basis_y,
        );
        quantity.auto_scale(self.length_scale());
        self.add_quantity(Box::new(quantity));
        self
    }

    /// Adds a face intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_face_intrinsic_vector_quantity_auto(
        &mut self,
        name: impl Into<String>,
        vectors: Vec<Vec2>,
    ) -> &mut Self {
        let (bx, by) = self.compute_face_tangent_basis();
        self.add_face_intrinsic_vector_quantity(name, vectors, bx, by)
    }

    /// Adds a one-form quantity to this mesh.
    ///
    /// A one-form assigns a scalar value to each edge, rendered as arrows
    /// at edge midpoints. The `orientations` array specifies the sign convention
    /// for each edge (true = canonical low→high vertex direction).
    /// Arrow length and radius are auto-scaled based on mesh extent and edge flow magnitudes.
    pub fn add_one_form_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
        orientations: Vec<bool>,
    ) -> &mut Self {
        let mut quantity = MeshOneFormQuantity::new(name, self.name.clone(), values, orientations);
        quantity.auto_scale(self.length_scale(), &self.vertices, &self.edges);
        self.add_quantity(Box::new(quantity));
        self
    }

    // === Active quantity accessors ===

    /// Returns the currently active vertex scalar quantity, if any.
    #[must_use]
    pub fn active_vertex_scalar_quantity(&self) -> Option<&MeshVertexScalarQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<MeshVertexScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active face scalar quantity, if any.
    #[must_use]
    pub fn active_face_scalar_quantity(&self) -> Option<&MeshFaceScalarQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Scalar {
                if let Some(sq) = q.as_any().downcast_ref::<MeshFaceScalarQuantity>() {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex color quantity, if any.
    #[must_use]
    pub fn active_vertex_color_quantity(&self) -> Option<&MeshVertexColorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<MeshVertexColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active face color quantity, if any.
    #[must_use]
    pub fn active_face_color_quantity(&self) -> Option<&MeshFaceColorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Color {
                if let Some(cq) = q.as_any().downcast_ref::<MeshFaceColorQuantity>() {
                    return Some(cq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex vector quantity (immutable), if any.
    #[must_use]
    pub fn active_vertex_vector_quantity(&self) -> Option<&MeshVertexVectorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any().downcast_ref::<MeshVertexVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex vector quantity (mutable), if any.
    pub fn active_vertex_vector_quantity_mut(&mut self) -> Option<&mut MeshVertexVectorQuantity> {
        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any_mut().downcast_mut::<MeshVertexVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active face vector quantity (immutable), if any.
    #[must_use]
    pub fn active_face_vector_quantity(&self) -> Option<&MeshFaceVectorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any().downcast_ref::<MeshFaceVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Returns the currently active face vector quantity (mutable), if any.
    pub fn active_face_vector_quantity_mut(&mut self) -> Option<&mut MeshFaceVectorQuantity> {
        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(vq) = q.as_any_mut().downcast_mut::<MeshFaceVectorQuantity>() {
                    return Some(vq);
                }
            }
        }
        None
    }

    /// Computes face centroids (average of face vertices).
    #[must_use]
    pub fn face_centroids(&self) -> Vec<Vec3> {
        self.faces
            .iter()
            .map(|face| {
                if face.is_empty() {
                    return Vec3::ZERO;
                }
                let sum: Vec3 = face.iter().map(|&i| self.vertices[i as usize]).sum();
                sum / face.len() as f32
            })
            .collect()
    }

    /// Returns the currently active vertex parameterization quantity, if any.
    #[must_use]
    pub fn active_vertex_parameterization_quantity(
        &self,
    ) -> Option<&MeshVertexParameterizationQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Parameterization {
                if let Some(pq) = q
                    .as_any()
                    .downcast_ref::<MeshVertexParameterizationQuantity>()
                {
                    return Some(pq);
                }
            }
        }
        None
    }

    /// Returns the currently active corner parameterization quantity, if any.
    #[must_use]
    pub fn active_corner_parameterization_quantity(
        &self,
    ) -> Option<&MeshCornerParameterizationQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Parameterization {
                if let Some(pq) = q
                    .as_any()
                    .downcast_ref::<MeshCornerParameterizationQuantity>()
                {
                    return Some(pq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex intrinsic vector quantity (immutable), if any.
    #[must_use]
    pub fn active_vertex_intrinsic_vector_quantity(
        &self,
    ) -> Option<&MeshVertexIntrinsicVectorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any()
                    .downcast_ref::<MeshVertexIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active vertex intrinsic vector quantity (mutable), if any.
    pub fn active_vertex_intrinsic_vector_quantity_mut(
        &mut self,
    ) -> Option<&mut MeshVertexIntrinsicVectorQuantity> {
        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any_mut()
                    .downcast_mut::<MeshVertexIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active face intrinsic vector quantity (immutable), if any.
    #[must_use]
    pub fn active_face_intrinsic_vector_quantity(
        &self,
    ) -> Option<&MeshFaceIntrinsicVectorQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q.as_any().downcast_ref::<MeshFaceIntrinsicVectorQuantity>() {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active face intrinsic vector quantity (mutable), if any.
    pub fn active_face_intrinsic_vector_quantity_mut(
        &mut self,
    ) -> Option<&mut MeshFaceIntrinsicVectorQuantity> {
        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(iq) = q
                    .as_any_mut()
                    .downcast_mut::<MeshFaceIntrinsicVectorQuantity>()
                {
                    return Some(iq);
                }
            }
        }
        None
    }

    /// Returns the currently active one-form quantity (immutable), if any.
    #[must_use]
    pub fn active_one_form_quantity(&self) -> Option<&MeshOneFormQuantity> {
        for q in &self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(oq) = q.as_any().downcast_ref::<MeshOneFormQuantity>() {
                    return Some(oq);
                }
            }
        }
        None
    }

    /// Returns the currently active one-form quantity (mutable), if any.
    pub fn active_one_form_quantity_mut(&mut self) -> Option<&mut MeshOneFormQuantity> {
        for q in &mut self.quantities {
            if q.is_enabled() && q.kind() == QuantityKind::Vector {
                if let Some(oq) = q.as_any_mut().downcast_mut::<MeshOneFormQuantity>() {
                    return Some(oq);
                }
            }
        }
        None
    }
}
