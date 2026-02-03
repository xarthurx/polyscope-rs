//! Volume mesh registration and manipulation.
//!
//! Volume meshes represent 3D volumetric data as tetrahedral or hexahedral cells.
//! Quantities (scalar, vector, color) can be defined on vertices or cells.
//!
//! # Example
//!
//! ```no_run
//! use polyscope_rs::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     // A single tetrahedron
//!     let vertices = vec![
//!         Vec3::new(0.0, 0.0, 0.0),
//!         Vec3::new(1.0, 0.0, 0.0),
//!         Vec3::new(0.5, 1.0, 0.0),
//!         Vec3::new(0.5, 0.5, 1.0),
//!     ];
//!     let tets = vec![[0, 1, 2, 3]];
//!
//!     let vm = register_tet_mesh("my tet", vertices, tets);
//!     vm.add_vertex_scalar_quantity("height", vec![0.0, 0.0, 0.0, 1.0]);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{Vec3, VolumeMesh, with_context_mut};

/// Registers a tetrahedral mesh with polyscope.
pub fn register_tet_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    tets: Vec<[u32; 4]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_tet_mesh(name.clone(), vertices, tets);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register tet mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Registers a hexahedral mesh with polyscope.
pub fn register_hex_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    hexes: Vec<[u32; 8]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_hex_mesh(name.clone(), vertices, hexes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register hex mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Registers a generic volume mesh with polyscope.
///
/// Cells are stored as 8-index arrays. For tetrahedra, indices 4-7 should be `u32::MAX`.
pub fn register_volume_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    cells: Vec<[u32; 8]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new(name.clone(), vertices, cells);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register volume mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

impl_structure_accessors! {
    get_fn = get_volume_mesh,
    with_fn = with_volume_mesh,
    with_ref_fn = with_volume_mesh_ref,
    handle = VolumeMeshHandle,
    type_name = "VolumeMesh",
    rust_type = VolumeMesh,
    doc_name = "volume mesh"
}

/// Handle for a registered volume mesh.
#[derive(Clone)]
pub struct VolumeMeshHandle {
    name: String,
}

impl VolumeMeshHandle {
    /// Returns the name of this volume mesh.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_color(color);
        });
        self
    }

    /// Sets the interior color.
    pub fn set_interior_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_interior_color(color);
        });
        self
    }

    /// Sets the edge color.
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_mesh(&self.name, |vm| {
            vm.set_edge_width(width);
        });
        self
    }

    /// Adds a vertex scalar quantity.
    pub fn add_vertex_scalar_quantity(&self, name: impl Into<String>, values: Vec<f32>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(&self, name: impl Into<String>, values: Vec<f32>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a vertex color quantity.
    pub fn add_vertex_color_quantity(&self, name: impl Into<String>, colors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_color_quantity(name, colors);
        });
        self
    }

    /// Adds a cell color quantity.
    pub fn add_cell_color_quantity(&self, name: impl Into<String>, colors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_color_quantity(name, colors);
        });
        self
    }

    /// Adds a vertex vector quantity.
    pub fn add_vertex_vector_quantity(&self, name: impl Into<String>, vectors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_vertex_vector_quantity(name, vectors);
        });
        self
    }

    /// Adds a cell vector quantity.
    pub fn add_cell_vector_quantity(&self, name: impl Into<String>, vectors: Vec<Vec3>) -> &Self {
        let name = name.into();
        with_volume_mesh(&self.name, |vm| {
            vm.add_cell_vector_quantity(name, vectors);
        });
        self
    }
}
