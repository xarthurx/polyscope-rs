use crate::{with_context, with_context_mut, CurveNetwork, Vec3};

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

/// Gets a registered curve network by name.
#[must_use]
pub fn get_curve_network(name: &str) -> Option<CurveNetworkHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("CurveNetwork", name) {
            Some(CurveNetworkHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
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

/// Executes a closure with mutable access to a registered curve network.
///
/// Returns `None` if the curve network does not exist.
pub fn with_curve_network<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut CurveNetwork) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("CurveNetwork", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<CurveNetwork>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered curve network.
///
/// Returns `None` if the curve network does not exist.
pub fn with_curve_network_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&CurveNetwork) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .and_then(|s| s.as_any().downcast_ref::<CurveNetwork>())
            .map(f)
    })
}
