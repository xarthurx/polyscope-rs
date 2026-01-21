//! Structure implementations for polyscope-rs.
//!
//! This crate provides concrete implementations of geometric structures:
//! - Point clouds
//! - Surface meshes (triangles, polygons)
//! - Curve networks
//! - Volume meshes (tetrahedra, hexahedra)
//! - Volume grids (implicit surfaces)
//! - Camera views

pub mod camera_view;
pub mod curve_network;
pub mod point_cloud;
pub mod surface_mesh;
pub mod volume_grid;
pub mod volume_mesh;

pub use camera_view::CameraView;
pub use curve_network::CurveNetwork;
pub use point_cloud::PointCloud;
pub use surface_mesh::SurfaceMesh;
pub use volume_grid::VolumeGrid;
pub use volume_mesh::VolumeMesh;
