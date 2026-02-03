//! Volume grid registration and manipulation.
//!
//! Volume grids are regular 3D grids for visualizing scalar fields. They support
//! node-centered and cell-centered data, with visualization modes including
//! gridcubes (voxels) and isosurfaces (marching cubes).
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     // Create a 10x10x10 grid
//!     let grid = register_volume_grid(
//!         "my grid",
//!         glam::UVec3::new(10, 10, 10),
//!         Vec3::new(-1.0, -1.0, -1.0),
//!         Vec3::new(1.0, 1.0, 1.0),
//!     );
//!
//!     // Add a scalar field (e.g., signed distance to sphere)
//!     let mut values = Vec::new();
//!     for k in 0..10 {
//!         for j in 0..10 {
//!             for i in 0..10 {
//!                 let p = Vec3::new(i as f32, j as f32, k as f32) / 9.0 * 2.0 - 1.0;
//!                 values.push(p.length() - 0.5); // sphere SDF
//!             }
//!         }
//!     }
//!     grid.add_node_scalar_quantity("sdf", values);
//!     grid.set_quantity_enabled("sdf", true);
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{Vec3, VolumeGrid, with_context_mut};
use polyscope_core::structure::HasQuantities;
use polyscope_structures::volume_grid::{
    VolumeGridCellScalarQuantity, VolumeGridNodeScalarQuantity, VolumeGridVizMode,
};

/// Registers a volume grid with polyscope.
pub fn register_volume_grid(
    name: impl Into<String>,
    node_dim: glam::UVec3,
    bound_min: Vec3,
    bound_max: Vec3,
) -> VolumeGridHandle {
    let name = name.into();
    let grid = VolumeGrid::new(name.clone(), node_dim, bound_min, bound_max);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(grid))
            .expect("failed to register volume grid");
        ctx.update_extents();
    });

    VolumeGridHandle { name }
}

/// Registers a volume grid with uniform dimensions.
pub fn register_volume_grid_uniform(
    name: impl Into<String>,
    dim: u32,
    bound_min: Vec3,
    bound_max: Vec3,
) -> VolumeGridHandle {
    register_volume_grid(name, glam::UVec3::splat(dim), bound_min, bound_max)
}

impl_structure_accessors! {
    get_fn = get_volume_grid,
    with_fn = with_volume_grid,
    with_ref_fn = with_volume_grid_ref,
    handle = VolumeGridHandle,
    type_name = "VolumeGrid",
    rust_type = VolumeGrid,
    doc_name = "volume grid"
}

/// Handle for a registered volume grid.
#[derive(Clone)]
pub struct VolumeGridHandle {
    name: String,
}

impl VolumeGridHandle {
    /// Returns the name of this volume grid.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the edge color.
    pub fn set_edge_color(&self, color: Vec3) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_color(color);
        });
        self
    }

    /// Sets the edge width.
    pub fn set_edge_width(&self, width: f32) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_edge_width(width);
        });
        self
    }

    /// Adds a node scalar quantity.
    pub fn add_node_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.add_node_scalar_quantity(name, values);
        });
        self
    }

    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.add_cell_scalar_quantity(name, values);
        });
        self
    }

    /// Sets the cube size factor (0 = no cubes, 1 = full size).
    pub fn set_cube_size_factor(&self, factor: f32) -> &Self {
        with_volume_grid(&self.name, |vg| {
            vg.set_cube_size_factor(factor);
        });
        self
    }

    /// Enables a quantity by name.
    pub fn set_quantity_enabled(&self, quantity_name: &str, enabled: bool) -> &Self {
        with_volume_grid(&self.name, |vg| {
            if let Some(q) = vg.get_quantity_mut(quantity_name) {
                q.set_enabled(enabled);
            }
        });
        self
    }

    /// Sets the visualization mode for a node scalar quantity.
    pub fn set_node_scalar_viz_mode(&self, quantity_name: &str, mode: VolumeGridVizMode) -> &Self {
        with_volume_grid(&self.name, |vg| {
            if let Some(q) = vg.get_quantity_mut(quantity_name) {
                if let Some(nsq) = q
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    nsq.set_viz_mode(mode);
                }
            }
        });
        self
    }

    /// Sets the isosurface level for a node scalar quantity.
    pub fn set_isosurface_level(&self, quantity_name: &str, level: f32) -> &Self {
        with_volume_grid(&self.name, |vg| {
            if let Some(q) = vg.get_quantity_mut(quantity_name) {
                if let Some(nsq) = q
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    nsq.set_isosurface_level(level);
                }
            }
        });
        self
    }

    /// Sets the isosurface color for a node scalar quantity.
    pub fn set_isosurface_color(&self, quantity_name: &str, color: Vec3) -> &Self {
        with_volume_grid(&self.name, |vg| {
            if let Some(q) = vg.get_quantity_mut(quantity_name) {
                if let Some(nsq) = q
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    nsq.set_isosurface_color(color);
                }
            }
        });
        self
    }

    /// Sets the color map for a quantity (node or cell scalar).
    pub fn set_color_map(&self, quantity_name: &str, color_map: &str) -> &Self {
        with_volume_grid(&self.name, |vg| {
            if let Some(q) = vg.get_quantity_mut(quantity_name) {
                if let Some(nsq) = q
                    .as_any_mut()
                    .downcast_mut::<VolumeGridNodeScalarQuantity>()
                {
                    nsq.set_color_map(color_map);
                } else if let Some(csq) = q
                    .as_any_mut()
                    .downcast_mut::<VolumeGridCellScalarQuantity>()
                {
                    csq.set_color_map(color_map);
                }
            }
        });
        self
    }
}
