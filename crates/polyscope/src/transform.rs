use crate::{Mat4, with_context, with_context_mut};

/// Generates `set_<name>_transform` and `get_<name>_transform` functions for a structure type.
macro_rules! impl_transform_accessors {
    ($set_fn:ident, $get_fn:ident, $type_name:expr, $doc_name:expr) => {
        #[doc = concat!("Sets the transform of a ", $doc_name, " by name.")]
        pub fn $set_fn(name: &str, transform: Mat4) {
            with_context_mut(|ctx| {
                if let Some(s) = ctx.registry.get_mut($type_name, name) {
                    s.set_transform(transform);
                }
            });
        }

        #[doc = concat!("Gets the transform of a ", $doc_name, " by name.")]
        #[must_use]
        pub fn $get_fn(name: &str) -> Option<Mat4> {
            with_context(|ctx| {
                ctx.registry
                    .get($type_name, name)
                    .map(polyscope_core::Structure::transform)
            })
        }
    };
}

impl_transform_accessors!(
    set_point_cloud_transform,
    get_point_cloud_transform,
    "PointCloud",
    "point cloud"
);
impl_transform_accessors!(
    set_surface_mesh_transform,
    get_surface_mesh_transform,
    "SurfaceMesh",
    "surface mesh"
);
impl_transform_accessors!(
    set_curve_network_transform,
    get_curve_network_transform,
    "CurveNetwork",
    "curve network"
);
impl_transform_accessors!(
    set_volume_mesh_transform,
    get_volume_mesh_transform,
    "VolumeMesh",
    "volume mesh"
);
