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

pub mod camera_view;
pub mod curve_network;
pub mod point_cloud;
pub mod surface_mesh;
pub mod volume_grid;
pub mod volume_mesh;

pub use camera_view::{CameraExtrinsics, CameraIntrinsics, CameraParameters, CameraView};
pub use curve_network::CurveNetwork;
pub use point_cloud::PointCloud;
pub use surface_mesh::SurfaceMesh;
pub use volume_grid::VolumeGrid;
pub use volume_mesh::{VolumeCellType, VolumeMesh};
