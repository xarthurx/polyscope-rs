use crate::{with_context, with_context_mut, Mat4};

/// Sets the transform of a point cloud by name.
pub fn set_point_cloud_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(pc) = ctx.registry.get_mut("PointCloud", name) {
            pc.set_transform(transform);
        }
    });
}

/// Gets the transform of a point cloud by name.
#[must_use]
pub fn get_point_cloud_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("PointCloud", name)
            .map(polyscope_core::Structure::transform)
    })
}

/// Sets the transform of a surface mesh by name.
pub fn set_surface_mesh_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(mesh) = ctx.registry.get_mut("SurfaceMesh", name) {
            mesh.set_transform(transform);
        }
    });
}

/// Gets the transform of a surface mesh by name.
#[must_use]
pub fn get_surface_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("SurfaceMesh", name)
            .map(polyscope_core::Structure::transform)
    })
}

/// Sets the transform of a curve network by name.
pub fn set_curve_network_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(cn) = ctx.registry.get_mut("CurveNetwork", name) {
            cn.set_transform(transform);
        }
    });
}

/// Gets the transform of a curve network by name.
#[must_use]
pub fn get_curve_network_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("CurveNetwork", name)
            .map(polyscope_core::Structure::transform)
    })
}

/// Sets the transform of a volume mesh by name.
pub fn set_volume_mesh_transform(name: &str, transform: Mat4) {
    with_context_mut(|ctx| {
        if let Some(vm) = ctx.registry.get_mut("VolumeMesh", name) {
            vm.set_transform(transform);
        }
    });
}

/// Gets the transform of a volume mesh by name.
#[must_use]
pub fn get_volume_mesh_transform(name: &str) -> Option<Mat4> {
    with_context(|ctx| {
        ctx.registry
            .get("VolumeMesh", name)
            .map(polyscope_core::Structure::transform)
    })
}
