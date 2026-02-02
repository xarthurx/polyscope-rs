# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.4] - 2025-02-03

### Added
- Module-level rustdoc with examples for curve_network, volume_mesh, volume_grid, camera_view, slice_plane, and groups modules

## [0.5.3] - 2025-02-03

### Added
- Comprehensive API coverage tests for slice planes, groups, transforms, and quantities
- Getting-started guide with full documentation of all structure types and quantities
- Improved rustdoc coverage with examples for init, point_cloud, and surface_mesh modules

### Fixed
- Rustdoc warnings for escaped brackets and code formatting

## [0.5.2] - 2025-02-03

### Changed
- **Code Refactoring:** Split `surface_mesh/mod.rs` into `geometry.rs` and `quantity_methods.rs` modules for improved maintainability
- **Code Refactoring:** Split `engine/pipelines.rs` into `pipelines/` directory with `structure.rs`, `effects.rs`, and `volume.rs` modules
- Camera demo updates and label improvements
- Screenshot gallery and headless screenshot generator

### Fixed
- Camera navigation issues
- Asset regeneration for bunny mesh with proper tetrahedralization

## [0.5.1] - 2025-02-01

### Added
- GPU picking for VolumeMesh cells and VolumeGrid gridcubes
- Flat 24-bit GPU picking with surface mesh face selection
- Gridcube and isosurface visualization for scalar quantities
- Group visibility propagation

### Changed
- Updated documentation to reflect RGBA color support

### Fixed
- Multi-pass egui layout to prevent Grid widget blink
- Various warning fixes

## [0.5.0] - 2025-01-30

### Added
- Full RGBA color support with per-element alpha
- Depth peeling transparency rendering
- All 6 camera navigation modes matching C++ Polyscope (Turntable, Free, Planar, Arcball, Flight, 2D)
- Full polygon mesh support via generic `IntoFaceList` trait
- Reset View button in UI

### Changed
- Updated to Rust edition 2024, MSRV 1.85
- Major refactoring: split monolithic `app.rs` into 4 focused modules
- Major refactoring: split monolithic `lib.rs` into 14 focused module files
- Color storage changed from Vec3 to Vec4 throughout codebase

### Fixed
- Camera pitch direction and gimbal clamping in turntable mode
- Yaw direction in view-matrix turntable orbit
- Exposure default to match C++ Polyscope
- All clippy warnings resolved (zero warnings policy)

## [0.2.0] - 2025-01-25

### Added
- Surface mesh rendering with quantities (scalar, color, vector)
- Curve network support with line and tube rendering
- Point cloud rendering with sphere impostors
- Ground plane with shadows and reflections
- Slice planes with capping
- SSAO (Screen Space Ambient Occlusion)
- Tone mapping
- GPU picking for structure and element selection
- egui-based UI with structure tree, quantity controls

## [0.1.0] - 2025-01-21

### Added
- Initial project scaffolding
- Basic windowed rendering with wgpu
- Point cloud visualization
- Basic camera controls
- Core data structures for 3D visualization
