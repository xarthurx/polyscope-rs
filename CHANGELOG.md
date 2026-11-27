# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `RenderEngine::lookup_pick_owner` and `PickOwnerHit`: the owner of a pick
  index, with the index of the quantity that owns the range.
- `write_pick_global_start` for point clouds, surface meshes, curve networks,
  camera views, volume meshes and volume grid scalar quantities. It writes
  only the first pick index to the pick uniforms.

### Fixed
- A click on a surface mesh or a volume mesh selected element 0. A click now
  takes the element from the pick buffer, as C++ Polyscope does: the face of a
  surface mesh and the cell of a volume mesh. A click no longer tests each
  mesh triangle on the CPU.
- A click did not select a volume grid or a camera view. It now selects a
  volume grid with the cell of a cell quantity or the node of a node quantity,
  and a camera view with the edge of its wireframe.
- If another thread moved a structure during a frame, the pick buffer of that
  frame could use the new position while the window showed the old one. The
  frame preparation now writes the pick uniforms together with the render
  uniforms, and the pick pass writes only the first pick index of each
  structure.
- The pick pass used the curve network tube geometry of the frame before, so
  for one frame a tube was picked at its old place or with its old radius. The
  pick pass now runs after the scene passes, which make this geometry.

## [0.6.2] - 2026-11-26

### Added
- `RenderEngine::clear_pick_ranges`, `RenderEngine::assign_quantity_pick_range`
  and `RenderEngine::get_quantity_pick_range_start`.
- `set_pick_global_start` and `pick_global_start` for point clouds, surface
  meshes, curve networks, camera views, volume meshes and volume grid scalar
  quantities, and `CameraView::update_pick_uniforms`.

### Changed
- `RenderEngine::lookup_global_index` reports the volume grid name for a
  gridcube pick. Before, it reported `<grid>/<quantity>`. A grid name with `/`,
  or two quantities with the same name, could then give the wrong owner.

### Fixed
- The "Reset View" button refreshes the scene extents before it sets the home
  view, as C++ Polyscope does (upstream commit 08fd238). Before, the view used
  the extents of the last register or remove call. It did not follow a later
  transform change or position update.
- After the element count of a structure changed, a click on a new element
  selected an element of another structure, or nothing. Examples are
  `update_points` with more points, and a new registration of a structure with
  more elements. Each window frame now gives all structures their pick index
  ranges again, one after the other, from their current element counts. Thus
  a removed or empty structure keeps no pick indices, and a long session does
  not use up the 2^24 pick indices. Hidden structures get no pick indices.
  One lock of the context covers the whole pick pass, so another thread cannot
  change the registry between the pick ranges, the pick uniforms and the draws.
- A volume mesh kept its pick resources when a slice plane culled cells or
  showed them again, so the pick buffer drew the old cell geometry. A visible
  cell could then be missing from the pick buffer. The volume mesh now creates
  its pick resources again when its render data changes, and it takes the
  cell of each picked triangle from the render data.
- If a scene has more pickable elements than the 2^24 - 1 indices of the pick
  buffer, the pick buffer shows the elements past the limit as the background,
  and a warning is logged once. Before, their indices wrapped to the indices
  of other elements.

### Documentation
- [Feature Status](docs/feature-status.md): the upstream sync point is C++
  commit 4efedba. The page lists the new C++ features that are not ported:
  multiple viewports and the new `SimpleTriangleMesh` functions.
- Feature Status lists two known issues of the click code: a click on a
  surface mesh or a volume mesh selects element 0, and a click does not select
  a volume grid. It also lists one known issue of the pick pass: if another
  thread moves a structure during a frame, the pick buffer of that frame can
  use the new position while the window shows the old one.

## [0.6.1] - 2026-11-16

### Changed
- The default background color is white, as in C++ Polyscope. It was dark
  gray (0.1, 0.1, 0.1). Set `Options::background_color` for another color.
- The "Add" button of the Slice Planes panel adds a plane with an automatic
  name ("Scene Slice Plane N") when the name field is empty, as the
  "Add plane" button of C++ Polyscope does. Before, it did nothing.

### Added
- `Camera::update_flight_at(now)` steps a camera flight to a given time.
  `Camera::update_flight` calls it with the current time.

### Documentation
- New README images, taken with the 0.6 render look and the white background.

## [0.6.0] - 2026-11-16

This release updates all dependencies and fixes the defects that an audit
against C++ Polyscope found. It contains breaking changes.
[Feature Status](docs/feature-status.md) lists the C++ features that are not
ported yet.

Rendered images look different from 0.5: the display gamma is applied once
(colors are darker and more saturated, as in C++ Polyscope), and screenshots
and headless renders show the same frame as the window.

### Breaking changes
- Dependencies: wgpu 27 → 30, egui 0.33 → 0.36, glam 0.29 → 0.34,
  transform-gizmo-egui 0.8 → 0.11, pollster 0.4 → 1.0, rfd 0.15 → 0.18. The
  public API uses `glam` types, so your code must use glam 0.34.
- The minimum Rust version is 1.96 (was 1.85).
- `Options`: `background_color` is a `Vec3` (was `Vec4`). The new field
  `ground_plane` (`GroundPlaneConfig`) holds the ground plane settings. The
  fields `ground_plane_enabled`, `ground_plane_mode`, `ground_plane_height`,
  `transparency_enabled`, `invoke_user_callback_for_nested_show` and
  `give_focus_on_show` are removed. They had no effect.
- `polyscope_core::options::GroundPlaneMode` (a duplicate enum) and
  `GroundPlaneConfig::shadow_blur_iters` are removed. They had no effect.
- View state JSON: `ground_plane.enabled` and `transparency.enabled` are
  removed. `ground_plane` has the new fields `height_is_relative`,
  `shadow_darkness` and `reflection_intensity`. The SSAA factor must be 1, 2 or 4.
- `VolumeMesh::build_egui_ui` and `VolumeGrid::build_egui_ui` take the list of
  material names. The `build_egui_ui` methods of the volume mesh quantities
  return `bool`.
- `TransformGizmo::interact` takes `&GizmoSettings`.
- `RenderEngine` screenshot API: `tone_map_to_screenshot_target` replaces
  `create_screenshot_target`, `apply_screenshot_tone_mapping`,
  `screenshot_texture_view` and `screenshot_depth_view`.
  `render_ground_plane` takes the scene bounds, not the minimum Y coordinate.
  `render_ground_plane` and `render_stencil_pass` always draw into the HDR
  texture and take no target view. `render_slice_planes_with_clear` takes an
  RGBA clear color.
- `save_image` and `save_to_buffer` take RGBA pixels and do not swap channels.
  `RenderEngine::capture_screenshot` returns RGBA for every capture format.
- `ToneMapUniforms` has the new field `output_is_srgb`.
- `CurveNetwork::update_gpu_buffers` and `CurveNetwork::update_pick_uniforms`
  take the scene length scale.
- The default cube size factor of a volume grid is 1 (full cell). It was 0,
  with an internal floor of 0.5.
- `VolumeMesh::cell_type` classifies cells by the number of sentinel indices
  (0/2/3/4 → Hex/Prism/Pyramid/Tet), not by `cell[4] == u32::MAX`. Mixed meshes
  from external pipelines must put the sentinels in the trailing slots. Tet
  meshes from `new_tet_mesh` work as before.
- The engine requests only the Vulkan, Metal, DirectX 12 and WebGPU backends.
  The OpenGL backend never worked: it stopped with a shader validation panic.
  A machine without a supported backend now gets the error
  `AdapterCreationFailed`.

### Changed
- `Options` is the source of truth for the render look. Background color,
  ground plane, transparency mode, transparency render passes, SSAA factor,
  SSAO and maximum frame rate take effect when you set them with
  `with_context_mut(|ctx| ctx.options…)`, in the viewer and in headless
  rendering. The UI panels show and edit the same values.
- The viewer limits its frame rate to `Options::max_fps` (default 60, 0 = no
  limit). The "Max FPS" control had no effect before.
- A single click selects after 0.35 s, so the first click of a double-click
  does not change the selection. A double-click needs the second click within
  0.30 s and 6 pixels. These are the values of C++ Polyscope.
- The transform gizmo shows the handles of one mode at a time. The default
  mode is `Translate`.
- Scroll zoom uses a step relative to the distance to the view center.
  First-person movement and camera flights also scale with the scene.
- The built-in colormaps use the exact tables of C++ Polyscope. The colors of
  `viridis`, `blues`, `reds`, `coolwarm` and `rainbow` change slightly.
- The SSAO noise texture uses a fixed seed, so renders are the same on each
  run (upstream commit 8740ddc). The `rand` dependency is removed.
- SSAO is off when the SSAA factor is above 1. The combination gave a wrong
  image before.
- Screenshots and headless renders use the frame passes and the tone mapping
  of the window. A windowed screenshot shows the frame on screen, without the
  UI. A transparent-background screenshot keeps the ground plane with the alpha
  of the ground plane shader, as C++ Polyscope does.
- The gallery generator (`generate_screenshots`) enables its quantities and
  draws tubes.
- The unused dependencies `ply-rs` and `log` (in `polyscope-core`) are removed.

### Added
- View save/restore: write the camera and the render look (background, ground
  plane, transparency, SSAO, SSAA) to a JSON file and load it back. API:
  `save_view_to_file`, `load_view_from_file`, `save_view_to_json`,
  `load_view_from_json`, `current_view_state`, `apply_view_state`, `ViewState`
  and `ViewTransition` (`Instant` / `FlyTo`), all exported from
  `polyscope_rs`. Two buttons in the View section open native file dialogs
  (`rfd`). The JSON format is Rust-native and versioned; it is not the format
  of C++ Polyscope. Headless rendering uses a loaded view and skips the
  automatic camera fit. New example `view_state_demo`.
- Volume meshes support prism (wedge) and pyramid cells in addition to
  tetrahedra and hexahedra: `register_prism_mesh`, `register_pyramid_mesh`,
  `VolumeMesh::new_prism_mesh`, `VolumeMesh::new_pyramid_mesh`. Mixed meshes
  use the 8-slot cell array with `u32::MAX` in unused slots (upstream PR #353).
  New helpers `slice_prism` and `slice_pyramid`. New example
  `volume_mesh_mixed_cells_demo`.
- All 15 built-in colormaps of C++ Polyscope: viridis, plasma, inferno, magma,
  gray, coolwarm, blues, reds, pink-green, phase, spectral, rainbow, jet,
  turbo, hsv. Each colormap selector lists them. `BUILTIN_COLOR_MAP_NAMES`
  holds the names. `scripts/extract_colormaps.py` extracts the tables from the
  C++ source.
- Volume mesh scalar and color quantities color the mesh. The volume mesh
  panel lists all quantities with their controls.
  `VolumeMeshHandle::set_quantity_enabled` enables a quantity from code.
- `set_material` on volume meshes and volume grids, with a material selector
  in their panels.
- Transparent-background screenshots (`ScreenshotOptions::transparent_background`),
  and `render_to_image_with_options` / `render_to_file_with_options` for
  headless rendering.
- The Transform panel has a gizmo mode selector (translate, rotate, scale) and
  snap fields. `set_gizmo_mode` and the snap setters take effect.
- A double-click or Ctrl+Shift+click sets the view center on every structure
  type, and the view flies to the new center. Camera views are in the pick
  pass. A right click without a drag clears the selection. A held Shift, Ctrl
  or Alt key suppresses the pick. (Upstream commits 61fc32a, 1b41445, 022deea.)
- The ground plane, its shadow light and its reflection follow the up
  direction. With Z up, the plane lies under the scene in the XY plane.
- `Camera::set_front_direction`. Reset View uses the up and front directions.
- The option types `GroundPlaneConfig`, `GroundPlaneMode`, `TransparencyMode`
  and `SsaoConfig` are exported from `polyscope_rs`.

### Fixed
- The display gamma was applied twice: the tone mapping shader applied it, and
  the sRGB output texture encoded the result again. Colors were pale, and a
  background of 0.1 showed as 163/255 instead of 94/255.
- `render_to_file` wrote files with red and blue swapped.
- Headless renders and screenshots did not show the ground plane, shadows,
  reflections, curve network tubes, volume grid gridcubes or isosurfaces.
- `show()` after a headless render, or a headless render after a window
  session, stopped with a wgpu error about a buffer of another device.
- Gridcubes were a quarter of their cell size, so a volume grid scalar showed
  as scattered dots.
- A relative curve network radius (`set_radius(r, true)`) drew as an absolute
  value. It is now a fraction of the scene length scale.
- In tube mode, the node spheres of a transformed curve network got the
  transform twice. The curve network pick radius did not follow radius changes.
- CI: a new clippy lint (`manual_midpoint`) and unit tests that used one
  global context at the same time made the CI of `main` fail. The tests now
  run one at a time and start from a clean scene.
- A windowed screenshot of a scene with a surface mesh or a volume mesh
  stopped the viewer with a wgpu validation error.
- A loaded view did not apply the transparency mode and the SSAA factor, and
  a saved view did not contain the transparency mode of the UI.
- Vector quantities on curve networks and on volume meshes were stored but
  not drawn.
- `PointCloudHandle::add_scalar_quantity`, `add_vector_quantity` and
  `add_color_quantity` did not add the quantity.
- `update_points`, `update_vertices` and `update_node_positions` did not
  change the image of a running viewer. A changed node count stopped the
  viewer.
- A point cloud, curve network or surface mesh with no elements stopped the
  renderer with a wgpu validation error.
- Wireframes showed the triangulation diagonals of quad and polygon faces, on
  volume meshes and on surface meshes.
- The bounding boxes of volume meshes and volume grids ignored the transform.
- Zoom moved the camera away from the scene in small scenes. Fixed clip plane
  and orthographic scale limits clipped small and large scenes.
- A change of the up direction to the axis of the view gave an empty view,
  and Reset View did not repair it (upstream commit 1e5ff25). An invalid view
  now falls back to the home view.
- A camera flight ended at the wrong view center, for example after
  "Load View…".
- A double-click rotated the view in Planar navigation.
- A scene with one point had a length scale of 0, so zoom, new slice planes
  and the shadow light did not work (upstream commit 3198ab5).
- The point cloud pick ignored the model transform.
- `shutdown()` did not clear the file-drop callback (upstream commit 8cbdf24).
- A removed structure or slice plane stayed selected.
- `init()` failed after `shutdown()`. It now starts again with an empty scene.
- The gizmo ignored `set_gizmo_mode` and the snap settings.
- A left drag in the 3D view did not orbit the camera while a structure was
  selected, because the gizmo took the pointer everywhere.
- A drag that started on the left panel moved the camera. A click inside the
  selection panel went through to the scene and cleared the selection. The
  pointer checks now use the display scale, so they are correct on HiDPI
  displays.
- Prism side quad 2 has the correct triangle winding; its normals pointed
  inward (the same defect is in upstream `stencilPrism`).

## [0.5.10] - 2026-05-04

### Fixed
- VolumeGrid isosurface indexing bug — VolumeGrid stores values as `i + j*nx + k*nx*ny` (z-slowest) but `marching_cubes` indexes as `(i*ny+j)*nz + k` (x-slowest), so isosurfaces were silently X/Z-transposed on uniform grids and visibly broken on non-uniform grids. Fixed by passing dims as `(nz, ny, nx)`, swizzling output positions/normals, and reversing triangle winding (the X↔Z swap is a reflection that flips handedness — without re-winding, `front_facing` shader checks and registered-mesh face normals were inverted). Added regression tests for non-uniform dimensions, anisotropic spacing, and winding consistency (upstream commit e91a709)
- Curve network tube rendering in orthographic projection — `curve_network_tube.wgsl`, `reflected_curve_network_tube.wgsl`, and `pick_curve_tube.wgsl` constructed perspective rays unconditionally (`normalize(world_pos - camera_pos)`), distorting tubes in ortho mode. The shaders now branch on a new `is_orthographic` flag in `CameraUniforms` and emit parallel rays along the world-space view forward direction in ortho mode. The ray-cylinder intersection routine also now handles parallel rays via end-cap intersection — without this, ortho viewing straight down a tube would produce NaN and the tube would disappear or fail to pick (upstream commit 51953c2)

### Added
- `add_slice_plane_auto()` — convenience constructor that picks the smallest unused "Scene Slice Plane N" index, mirroring C++ Polyscope's no-args `addSlicePlane()` (upstream commit 24ec7e3)
- `SlicePlaneHandle::remove(self)` — consume-self method for removing a slice plane, mirroring C++ `SlicePlane::remove()`

## [0.5.9] - 2026-03-02

### Changed
- Version bump for new crates.io release

## [0.5.8] - 2026-03-02

### Fixed
- CI formatting check and publish workflow issues

## [0.5.7] - 2026-03-01

### Added
- `remove_all_groups()` — bulk removal of all groups (upstream commit f34f403)
- `remove_everything()` — unified scene reset: removes all structures, groups, slice planes, floating quantities, and clears callbacks (upstream commit f34f403)

### Fixed
- Degenerate bounding box handling — when all points coincide, the bounding box is now perturbed by 1e-3 tolerance to prevent broken camera framing (upstream commit 3198ab5)
- Smoother camera flight animation — interpolates the inverse view matrix (camera-to-world) instead of the view matrix directly, producing more natural motion when far from the origin (upstream commit 067f760)

## [0.5.6] - 2025-02-03

### Changed
- **BREAKING: Crate renamed from `polyscope` to `polyscope-rs`** for crates.io publishing (the name `polyscope` was unavailable)
  - Update your `Cargo.toml`: `polyscope = "0.5"` → `polyscope-rs = "0.5"`
  - Update your imports: `use polyscope::*` → `use polyscope_rs::*`
- **Code cleanup:** Removed dead code (unused `HEX_ROTATION_MAP`, `pick_structure_at_screen_pos`, `set_background_color`, `frame_tick`, `request_redraw`)
- **Code cleanup:** Narrowed `#[allow(dead_code)]` from struct-level to field-level on `ShadowMapPass`, `CurveNetwork`, `VolumeGridCellScalarQuantity`; removed unnecessary annotation from `VolumeGrid`
- **Code consolidation:** Introduced `impl_transform_accessors!` macro to deduplicate transform getter/setter boilerplate
- **Code consolidation:** Introduced `impl_structure_accessors!` macro to deduplicate `get_*`/`with_*`/`with_*_ref` across 6 structure modules
- **Code consolidation:** Added `From<u32>` / `From<Enum> for u32` impls on `NavigationStyle`, `ProjectionMode`, `AxisDirection` to simplify `ui_sync.rs`
- Net reduction of ~350 lines of boilerplate

### Added
- Publishing metadata for all crates (keywords, categories, readme)

## [0.5.5] - 2025-02-03

### Added
- GitHub Actions CI workflow (check, clippy, fmt, test, doc)

### Fixed
- Clippy `unnecessary_unwrap` warnings
- Rustdoc warnings for unescaped brackets

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
