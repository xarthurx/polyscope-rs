//! Surface mesh registration and manipulation.
//!
//! Surface meshes are triangular or polygonal meshes. This module provides
//! functions to register meshes and add quantities to them.
//!
//! polyscope-rs supports arbitrary n-gon faces (triangles, quads, pentagons, etc.).
//! Polygonal faces are automatically fan-triangulated for rendering while
//! preserving the original face structure for wireframe display.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     let vertices = vec![
//!         Vec3::new(0.0, 0.0, 0.0),
//!         Vec3::new(1.0, 0.0, 0.0),
//!         Vec3::new(1.0, 1.0, 0.0),
//!         Vec3::new(0.0, 1.0, 0.0),
//!     ];
//!     let faces = vec![[0u32, 1, 2], [0, 2, 3]];
//!
//!     let mesh = register_surface_mesh("quad", vertices, faces);
//!     mesh.set_surface_color(Vec3::new(0.2, 0.5, 0.8))
//!         .set_show_edges(true);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{with_context, with_context_mut, SurfaceMesh, Vec2, Vec3, Vec4};
use glam::UVec3;

/// Trait for face data that can be converted to the internal polygon format.
///
/// Implemented for:
/// - `Vec<UVec3>` — triangle-only meshes
/// - `Vec<[u32; 3]>` — triangle-only meshes (array form)
/// - `Vec<Vec<u32>>` — arbitrary polygon meshes (triangles, quads, pentagons, etc.)
pub trait IntoFaceList {
    /// Converts to the internal polygon face representation.
    fn into_face_list(self) -> Vec<Vec<u32>>;
}

impl IntoFaceList for Vec<UVec3> {
    fn into_face_list(self) -> Vec<Vec<u32>> {
        self.into_iter().map(|f| vec![f.x, f.y, f.z]).collect()
    }
}

impl IntoFaceList for Vec<[u32; 3]> {
    fn into_face_list(self) -> Vec<Vec<u32>> {
        self.into_iter().map(|f| f.to_vec()).collect()
    }
}

impl IntoFaceList for Vec<Vec<u32>> {
    fn into_face_list(self) -> Vec<Vec<u32>> {
        self
    }
}

/// Registers a surface mesh with polyscope.
///
/// Accepts any face format implementing [`IntoFaceList`]:
/// - `Vec<UVec3>` or `Vec<[u32; 3]>` for triangle meshes
/// - `Vec<Vec<u32>>` for arbitrary polygon meshes (triangles, quads, n-gons)
///
/// Polygonal faces are automatically fan-triangulated for rendering while
/// preserving the original face structure for quantities and wireframe display.
///
/// # Panics
///
/// Panics if any face has fewer than 3 vertices or contains out-of-bounds
/// vertex indices.
pub fn register_surface_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    faces: impl IntoFaceList,
) -> SurfaceMeshHandle {
    let name = name.into();
    let faces = faces.into_face_list();
    let n_verts = vertices.len();

    // Validate faces
    for (i, face) in faces.iter().enumerate() {
        assert!(
            face.len() >= 3,
            "Face {i} has {} vertices (minimum 3 required)",
            face.len()
        );
        for &idx in face {
            assert!(
                (idx as usize) < n_verts,
                "Face {i} contains vertex index {idx} but mesh only has {n_verts} vertices"
            );
        }
    }

    let mesh = SurfaceMesh::new(name.clone(), vertices, faces);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register surface mesh");
        ctx.update_extents();
    });

    SurfaceMeshHandle { name }
}

/// Gets a registered surface mesh by name.
#[must_use]
pub fn get_surface_mesh(name: &str) -> Option<SurfaceMeshHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("SurfaceMesh", name) {
            Some(SurfaceMeshHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered surface mesh.
///
/// This handle provides methods to add quantities and configure the
/// appearance of a surface mesh. Methods return `&Self` to allow chaining.
///
/// # Example
///
/// ```no_run
/// use polyscope::*;
///
/// init().unwrap();
/// let vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
/// let faces = vec![[0u32, 1, 2]];
/// let mesh = register_surface_mesh("mesh", vertices, faces);
/// mesh.set_surface_color(Vec3::new(1.0, 0.5, 0.0))
///     .set_show_edges(true)
///     .add_vertex_scalar_quantity("height", vec![0.0, 1.0, 0.5]);
/// ```
#[derive(Clone)]
pub struct SurfaceMeshHandle {
    name: String,
}

impl SurfaceMeshHandle {
    /// Returns the name of this mesh.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    // -- Appearance setters --

    /// Sets the surface color.
    pub fn set_surface_color(&self, color: Vec3) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_surface_color(color);
        });
        self
    }

    /// Sets the edge color.
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_edge_width(width);
        });
        self
    }

    /// Sets whether edges are shown.
    pub fn set_show_edges(&self, show: bool) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_show_edges(show);
        });
        self
    }

    /// Sets the backface color.
    pub fn set_backface_color(&self, color: Vec3) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_backface_color(color);
        });
        self
    }

    /// Sets the transparency (0.0 = opaque, 1.0 = fully transparent).
    pub fn set_transparency(&self, transparency: f32) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_transparency(transparency);
        });
        self
    }

    /// Sets the material.
    pub fn set_material(&self, material: &str) -> &Self {
        use polyscope_core::Structure;
        with_surface_mesh(&self.name, |mesh| {
            mesh.set_material(material);
        });
        self
    }

    // -- Quantity methods --

    /// Adds a vertex scalar quantity.
    pub fn add_vertex_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a face scalar quantity.
    pub fn add_face_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a vertex color quantity (RGB, alpha defaults to 1.0).
    pub fn add_vertex_color_quantity(&self, name: &str, colors: Vec<Vec3>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_color_quantity(name, colors);
        });
        self
    }

    /// Adds a vertex color quantity with explicit per-vertex RGBA alpha values.
    ///
    /// Use this to specify per-vertex transparency. Requires Pretty (depth peeling)
    /// transparency mode to render correctly (set via appearance settings).
    pub fn add_vertex_color_quantity_with_alpha(&self, name: &str, colors: Vec<Vec4>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_color_quantity_with_alpha(name, colors);
        });
        self
    }

    /// Adds a face color quantity (RGB, alpha defaults to 1.0).
    pub fn add_face_color_quantity(&self, name: &str, colors: Vec<Vec3>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_color_quantity(name, colors);
        });
        self
    }

    /// Adds a face color quantity with explicit per-face RGBA alpha values.
    ///
    /// Use this to specify per-face transparency. Requires Pretty (depth peeling)
    /// transparency mode to render correctly (set via appearance settings).
    pub fn add_face_color_quantity_with_alpha(&self, name: &str, colors: Vec<Vec4>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_color_quantity_with_alpha(name, colors);
        });
        self
    }

    /// Adds a vertex vector quantity (auto-scaled).
    pub fn add_vertex_vector_quantity(&self, name: &str, vectors: Vec<Vec3>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_vector_quantity(name, vectors);
        });
        self
    }

    /// Adds a face vector quantity (auto-scaled).
    pub fn add_face_vector_quantity(&self, name: &str, vectors: Vec<Vec3>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_vector_quantity(name, vectors);
        });
        self
    }

    /// Adds a vertex parameterization (UV) quantity.
    pub fn add_vertex_parameterization_quantity(&self, name: &str, coords: Vec<Vec2>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_parameterization_quantity(name, coords);
        });
        self
    }

    /// Adds a corner parameterization (UV) quantity.
    pub fn add_corner_parameterization_quantity(&self, name: &str, coords: Vec<Vec2>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_corner_parameterization_quantity(name, coords);
        });
        self
    }

    /// Adds a vertex intrinsic vector quantity with explicit tangent basis (auto-scaled).
    pub fn add_vertex_intrinsic_vector_quantity(
        &self,
        name: &str,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_intrinsic_vector_quantity(name, vectors, basis_x, basis_y);
        });
        self
    }

    /// Adds a vertex intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_vertex_intrinsic_vector_quantity_auto(
        &self,
        name: &str,
        vectors: Vec<Vec2>,
    ) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_vertex_intrinsic_vector_quantity_auto(name, vectors);
        });
        self
    }

    /// Adds a face intrinsic vector quantity with explicit tangent basis (auto-scaled).
    pub fn add_face_intrinsic_vector_quantity(
        &self,
        name: &str,
        vectors: Vec<Vec2>,
        basis_x: Vec<Vec3>,
        basis_y: Vec<Vec3>,
    ) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_intrinsic_vector_quantity(name, vectors, basis_x, basis_y);
        });
        self
    }

    /// Adds a face intrinsic vector quantity with auto-computed tangent basis.
    pub fn add_face_intrinsic_vector_quantity_auto(&self, name: &str, vectors: Vec<Vec2>) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_face_intrinsic_vector_quantity_auto(name, vectors);
        });
        self
    }

    /// Adds a one-form quantity (edge-based differential form, auto-scaled).
    pub fn add_one_form_quantity(
        &self,
        name: &str,
        values: Vec<f32>,
        orientations: Vec<bool>,
    ) -> &Self {
        with_surface_mesh(&self.name, |mesh| {
            mesh.add_one_form_quantity(name, values, orientations);
        });
        self
    }
}

/// Executes a closure with mutable access to a registered surface mesh.
///
/// Returns `None` if the mesh does not exist.
pub fn with_surface_mesh<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut SurfaceMesh) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("SurfaceMesh", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<SurfaceMesh>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered surface mesh.
///
/// Returns `None` if the mesh does not exist.
pub fn with_surface_mesh_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&SurfaceMesh) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("SurfaceMesh", name)
            .and_then(|s| s.as_any().downcast_ref::<SurfaceMesh>())
            .map(f)
    })
}
