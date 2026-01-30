use crate::{with_context, with_context_mut, SurfaceMesh, Vec2, Vec3, Vec4};
use glam::UVec3;

/// Registers a surface mesh with polyscope.
pub fn register_surface_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    faces: Vec<UVec3>,
) -> SurfaceMeshHandle {
    let name = name.into();
    // Convert UVec3 faces to Vec<Vec<u32>> for the SurfaceMesh constructor
    let faces: Vec<Vec<u32>> = faces.into_iter().map(|f| vec![f.x, f.y, f.z]).collect();
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
    pub fn add_face_intrinsic_vector_quantity_auto(
        &self,
        name: &str,
        vectors: Vec<Vec2>,
    ) -> &Self {
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
