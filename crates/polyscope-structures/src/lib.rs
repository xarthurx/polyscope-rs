//! Structure implementations for polyscope-rs.
//!
//! This crate provides concrete implementations of geometric structures:
//! - Point clouds
//! - Surface meshes (triangles, polygons)
//! - Curve networks
//! - Volume meshes (tetrahedra, hexahedra)
//! - Volume grids (implicit surfaces)
//! - Camera views

// Type casts in geometry code: Conversions between index types (u32, usize) and
// coordinate types (f32, f64) are intentional. Mesh indices and vertex counts
// will not exceed u32::MAX in practice for 3D visualization.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
// Documentation lints: Detailed error/panic docs will be added as the API stabilizes.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// Method design: Some methods take &self for API consistency even when not using it.
#![allow(clippy::unused_self)]
// Struct design: Configuration structs may have many boolean fields.
#![allow(clippy::struct_excessive_bools)]
// Variable naming: In geometry code, similar variable names are common.
#![allow(clippy::similar_names)]
// Argument design: Some functions take ownership for API consistency.
#![allow(clippy::needless_pass_by_value)]
// Function signatures: Some geometry operations need many parameters.
#![allow(clippy::too_many_arguments)]
// Code style: Sometimes if-let-else is clearer than let-else.
#![allow(clippy::option_if_let_else)]
// Lifetimes: Some patterns require explicit lifetimes for clarity.
#![allow(clippy::needless_lifetimes)]
// Function length: Complex geometry operations are legitimately complex.
#![allow(clippy::too_many_lines)]

pub mod camera_view;
pub mod curve_network;
pub mod floating;
pub mod point_cloud;
pub mod surface_mesh;
pub mod volume_grid;
pub mod volume_mesh;

pub use camera_view::{CameraExtrinsics, CameraIntrinsics, CameraParameters, CameraView};
pub use curve_network::CurveNetwork;
pub use floating::{
    FloatingColorImage, FloatingColorRenderImage, FloatingDepthRenderImage, FloatingRawColorImage,
    FloatingScalarImage, ImageOrigin,
};
pub use point_cloud::PointCloud;
pub use surface_mesh::SurfaceMesh;
pub use volume_grid::VolumeGrid;
pub use volume_mesh::{VolumeCellType, VolumeMesh};

/// Returns the axis-aligned bounds of a box after a transform.
///
/// The function transforms the eight corners of the box and takes their bounds.
pub(crate) fn transform_bounds(
    min: glam::Vec3,
    max: glam::Vec3,
    transform: glam::Mat4,
) -> (glam::Vec3, glam::Vec3) {
    let mut world_min = glam::Vec3::splat(f32::MAX);
    let mut world_max = glam::Vec3::splat(f32::MIN);
    for i in 0..8 {
        let corner = glam::Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        );
        let world = transform.transform_point3(corner);
        world_min = world_min.min(world);
        world_max = world_max.max(world);
    }
    (world_min, world_max)
}
