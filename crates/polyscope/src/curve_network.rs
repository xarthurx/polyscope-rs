//! Curve network registration and manipulation.
//!
//! Curve networks represent graphs, paths, or lines as nodes connected by edges.
//! They can be rendered as lines or tubes, with quantities (scalar, vector, color)
//! defined on nodes or edges.
//!
//! # Example
//!
//! ```no_run
//! use polyscope::*;
//!
//! fn main() -> Result<()> {
//!     init()?;
//!
//!     // Create a simple path
//!     let nodes = vec![
//!         Vec3::new(0.0, 0.0, 0.0),
//!         Vec3::new(1.0, 0.5, 0.0),
//!         Vec3::new(2.0, 0.0, 0.0),
//!     ];
//!     let cn = register_curve_network_line("my path", nodes);
//!     cn.set_radius(0.02, false); // absolute radius
//!     cn.set_color(Vec3::new(1.0, 0.5, 0.0));
//!
//!     show();
//!     Ok(())
//! }
//! ```

use crate::{CurveNetwork, Vec3, with_context_mut};

/// Registers a curve network with explicit edges.
pub fn register_curve_network(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
    edges: Vec<[u32; 2]>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new(name.clone(), nodes, edges);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a connected line (0-1-2-3-...).
pub fn register_curve_network_line(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_line(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as a closed loop (0-1-2-...-n-0).
pub fn register_curve_network_loop(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_loop(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

/// Registers a curve network as separate segments (0-1, 2-3, 4-5, ...).
pub fn register_curve_network_segments(
    name: impl Into<String>,
    nodes: Vec<Vec3>,
) -> CurveNetworkHandle {
    let name = name.into();
    let cn = CurveNetwork::new_segments(name.clone(), nodes);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(cn))
            .expect("failed to register curve network");
        ctx.update_extents();
    });

    CurveNetworkHandle { name }
}

impl_structure_accessors! {
    get_fn = get_curve_network,
    with_fn = with_curve_network,
    with_ref_fn = with_curve_network_ref,
    handle = CurveNetworkHandle,
    type_name = "CurveNetwork",
    rust_type = CurveNetwork,
    doc_name = "curve network"
}

/// Handle for a registered curve network.
#[derive(Clone)]
pub struct CurveNetworkHandle {
    name: String,
}

impl CurveNetworkHandle {
    /// Returns the name of this curve network.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the base color.
    pub fn set_color(&self, color: Vec3) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_color(color);
        });
        self
    }

    /// Sets the radius.
    pub fn set_radius(&self, radius: f32, is_relative: bool) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_radius(radius, is_relative);
        });
        self
    }

    /// Sets the material.
    pub fn set_material(&self, material: &str) -> &Self {
        with_curve_network(&self.name, |cn| {
            cn.set_material(material);
        });
        self
    }
}
