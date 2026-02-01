use crate::{PointCloud, Vec3, with_context, with_context_mut};

/// Registers a point cloud with polyscope.
pub fn register_point_cloud(name: impl Into<String>, points: Vec<Vec3>) -> PointCloudHandle {
    let name = name.into();
    let point_cloud = PointCloud::new(name.clone(), points);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(point_cloud))
            .expect("failed to register point cloud");
        ctx.update_extents();
    });

    PointCloudHandle { name }
}

/// Gets a registered point cloud by name.
#[must_use]
pub fn get_point_cloud(name: &str) -> Option<PointCloudHandle> {
    with_context(|ctx| {
        if ctx.registry.contains("PointCloud", name) {
            Some(PointCloudHandle {
                name: name.to_string(),
            })
        } else {
            None
        }
    })
}

/// Handle for a registered point cloud.
#[derive(Clone)]
pub struct PointCloudHandle {
    name: String,
}

impl PointCloudHandle {
    /// Returns the name of this point cloud.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a scalar quantity to this point cloud.
    pub fn add_scalar_quantity(&self, name: &str, values: Vec<f32>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_scalar_quantity(name, values);
                }
            }
        });
        self
    }

    /// Adds a vector quantity to this point cloud.
    pub fn add_vector_quantity(&self, name: &str, vectors: Vec<Vec3>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_vector_quantity(name, vectors);
                }
            }
        });
        self
    }

    /// Adds a color quantity to this point cloud.
    pub fn add_color_quantity(&self, name: &str, colors: Vec<Vec3>) -> &Self {
        with_context_mut(|ctx| {
            if let Some(pc) = ctx.registry.get_mut("PointCloud", &self.name) {
                if let Some(pc) = (pc as &mut dyn std::any::Any).downcast_mut::<PointCloud>() {
                    pc.add_color_quantity(name, colors);
                }
            }
        });
        self
    }
}

/// Executes a closure with mutable access to a registered point cloud.
///
/// Returns `None` if the point cloud does not exist.
pub fn with_point_cloud<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&mut PointCloud) -> R,
{
    with_context_mut(|ctx| {
        ctx.registry
            .get_mut("PointCloud", name)
            .and_then(|s| s.as_any_mut().downcast_mut::<PointCloud>())
            .map(f)
    })
}

/// Executes a closure with immutable access to a registered point cloud.
///
/// Returns `None` if the point cloud does not exist.
pub fn with_point_cloud_ref<F, R>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&PointCloud) -> R,
{
    with_context(|ctx| {
        ctx.registry
            .get("PointCloud", name)
            .and_then(|s| s.as_any().downcast_ref::<PointCloud>())
            .map(f)
    })
}
