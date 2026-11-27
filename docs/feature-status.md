# Feature Status & Roadmap

Status of polyscope-rs 0.6.2 compared with C++ Polyscope 2.x.

## Summary

polyscope-rs ports the six main structures of C++ Polyscope: point clouds, surface meshes, curve networks, volume meshes, volume grids and camera views. Their main quantities draw in the viewer window.

The port does not have full feature parity. The largest gaps are:

- No per-frame user callback and no `frame_tick`. `show()` runs until the window closes.
- No global camera API. A `ViewState` or the UI sets the camera.
- Floating images are stored but not displayed. Render images and implicit surfaces are missing.
- Scalar quantities have no `DataType`, no isolines and no on-screen colorbar.
- Sparse Volume Grid and SimpleTriangleMesh are missing.

The section [Not ported yet](#not-ported-yet) lists each gap. The section [Differences from C++ Polyscope](#differences-from-c-polyscope) lists behaviour that exists but differs.

The tables use these status values:

- **Full**: the feature exists and has the options of C++ Polyscope, apart from the shared limits in the notes.
- **Partial**: the feature exists, but some options or parts are missing.
- **Missing**: the feature does not exist.
- **Rust only**: C++ Polyscope has no such feature.

## Structures

| Structure | Status | Notes |
|-----------|--------|-------|
| Point Cloud | Partial | Sphere impostors. Scalar, vector and color quantities. No parameterization quantities, quad render mode, radius or transparency quantities. |
| Surface Mesh | Partial | Triangles and arbitrary polygons. No edge, halfedge or corner scalars, no textures, no permutations. |
| Curve Network | Partial | Lines or tubes. One radius for the whole network: no per-node or per-edge radius. |
| Volume Mesh | Partial | Tet, hex, prism and pyramid cells. Interior faces are hidden. Slice planes remove whole cells. |
| Volume Grid | Partial | Node and cell scalars. A node scalar shows gridcubes or an isosurface, not both. |
| Camera View | Partial | Frustum widget. The "fly to" action is in the UI only. No image billboard. |
| Floating images | Partial | `register_floating_scalar_image` and `register_floating_color_image` store the image. Nothing displays it. |
| Sparse Volume Grid | Missing | |
| Simple Triangle Mesh | Missing | |

## Quantities

A quantity type is **Full** on a structure if it exists, draws and has the per-structure options of C++. These limits apply to every structure:

- Scalar quantities have no `DataType`, no isolines, no on-screen colorbar and no data histogram.
- No quantity type has an `update` method. Remove the quantity and add it again.
- Vector quantities have no `VectorType::AMBIENT`, no length range and no material.
- The `polyscope_rs` handles have no colormap or range setter, except `VolumeGridHandle::set_color_map`. The quantity panel sets both.

| Quantity | Point Cloud | Surface Mesh | Curve Network | Volume Mesh | Volume Grid |
|----------|-------------|--------------|---------------|-------------|-------------|
| Scalar | Full | Partial: vertex and face only | Full: node and edge | Full: vertex and cell | Partial: node and cell |
| Color (RGB) | Full | Full: vertex and face | Full: node and edge | Full: vertex and cell | — |
| Color (RGBA) | — | Rust only: vertex and face | — | — | — |
| Vector | Full | Full: vertex and face | Full: node and edge | Full: vertex and cell | — |
| Parameterization | Missing | Partial: vertex and corner | — | — | — |
| Intrinsic (tangent) vector | — | Full: vertex and face, with symmetry | — | — | — |
| One-form | — | Full | — | — | — |
| Edge, halfedge, corner scalar | — | Missing | — | — | — |
| Texture (scalar, color) | — | Missing | — | — | — |
| Distance, signed distance | — | Missing | — | — | — |
| Radius from a quantity | Missing | — | Missing | — | — |
| Transparency from a quantity | Missing | Missing | — | — | — |
| Level set | — | — | — | Missing | — |
| Image quantities on the structure | Missing | Missing | Missing | Missing | Missing |

A dash (—) means that C++ Polyscope also has no such quantity on the structure.

## Scene and Render Features

| Feature | Status | Notes |
|---------|--------|-------|
| Ground plane | Partial | Modes None, Tile, ShadowOnly and TileReflection. Follows the up direction. No height factor and no shadow blur setting. |
| Ground shadows | Partial | Only surface meshes cast shadows. |
| Ground reflections | Partial | Surface meshes, volume meshes, point clouds and curve networks show in the reflection. |
| Tone mapping | Full | Exposure, white level and gamma in the Tone Mapping panel. The displayed value is `pow(color, 1 / gamma)`, as in C++. |
| SSAO | Rust only | Screen-space ambient occlusion. No effect when the SSAA factor is above 1. |
| SSAA | Full | `Options::ssaa_factor`: 1, 2 or 4. |
| Transparency | Partial | Simple and Pretty (depth peeling). Only surface meshes have a structure transparency. |
| Materials | Full | 8 built-in matcap materials. `load_blendable_material` and `load_static_material` add more. |
| Colormaps | Partial | All 15 built-in maps of C++, with the C++ tables. No loader for a user colormap. |
| Slice planes | Partial | At most 4 planes act at a time. Volume meshes lose whole cells. No per-structure options. |
| Groups | Partial | Hierarchical groups. A disabled group hides its structures and child groups. |
| Screenshots | Partial | PNG and JPEG. Transparent background. No capture of the UI. |
| Headless rendering | Full | `render_to_image`, `render_to_file` and their `_with_options` forms. The same frame passes as the window: ground plane, shadows, reflections, tubes, gridcubes and isosurfaces. |
| Frame rate limit | Full | `Options::max_fps` (default 60, 0 = unlimited) limits the window. |
| Camera navigation | Full | Turntable, Free, Planar, Arcball, First Person and None. |
| Projection | Full | Perspective and orthographic. |
| Multiple viewports | Missing | One viewport and one camera for the window. |
| Camera flights | Partial | Double-click, view load and the camera view "fly to" action. The duration is fixed at 0.4 s. No API. |
| Scene extents | Full | Automatic, or manual in the Scene Extents panel. |
| Floating images | Missing | Stored, not displayed. |

## Interaction

| Feature | Status | Notes |
|---------|--------|-------|
| Mouse navigation | Partial | Rotate, pan, zoom and view center. No Ctrl+Shift+drag zoom and no Shift+scroll clip-plane shift. |
| Click selection | Full | The C++ click rules: delayed single-click pick, double-click, modifier keys, drag threshold. |
| Element picking | Partial | Point clouds report a point, surface meshes a face, curve networks an edge, volume meshes a cell, volume grids a node or a cell, and camera views an edge. No vertex, corner or node picks on surface meshes and curve networks. |
| Selection panel | Partial | Shows the structure and the element index. No quantity values. |
| Keyboard | Partial | W, A, S, D, Q, E in First Person navigation. Esc and F12. No Ctrl+C or Ctrl+V for the view. |
| Gizmo | Partial | One gizmo for the selected structure or slice plane. Translate, rotate or scale. Snap increments. |
| File drop | Full | `set_file_drop_callback`. |
| View save and load | Partial | UI buttons and API. Rust-native JSON format: C++ view files do not load. |

## Global API

| Area | Status | Notes |
|------|--------|-------|
| Init and show | Partial | `init`, `show`, `shutdown`, `remove_everything`. No user callback, `frame_tick`, `unshow` or nested `show`. |
| Options | Partial | `Options` holds the render look and the frame rate limit. No UI scale, panel width, window size, vsync or program name. |
| View and camera | Partial | Only through `ViewState`: `apply_view_state`, `current_view_state`, view files. No `look_at`, view-center or home-view functions. |
| Screenshots | Partial | `screenshot`, `screenshot_to_file` and their `_with_options` forms. No capture to memory while `show` runs. |
| Groups | Partial | `create_group`, `get_group`, `remove_group`, `GroupHandle`. No `unparent` and no top-level lookup. |
| Slice planes | Partial | `add_slice_plane`, `add_slice_plane_auto`, `add_slice_plane_with_pose`, `SlicePlaneHandle`. No transform or grid color setter. |
| Gizmo | Partial | `set_gizmo_mode`, `set_gizmo_space`, `set_gizmo_visible`, snap setters, selected-transform functions. No user gizmos. |
| Picking | Partial | `select_structure`, `get_selected_structure`, `has_selection`. No programmatic pick query. |
| Structures | Partial | `register_*`, `get_*`, `with_*`, `remove_structure`. No `has_*` functions and no type-qualified removal. |

---

## Not ported yet

One line per item. The C++ names are in backticks.

### Frame loop and user code

- `userCallback`: no per-frame user callback and no user UI panel (`openImGuiWindowForUserCallback`, `invokeUserCallbackForNestedShow`).
- `frameTick()`, `options::frameTickLimitFPSMode` and `LimitFPSMode`.
- `show(forFrames)`: `show()` runs until the window closes.
- `unshow()` and `windowRequestsClose()`.
- `pushContext()` and `popContext()`: no nested `show()`.
- `draw()`, `requestRedraw()`, `redrawRequested()` and `options::alwaysRedraw`.
- `refresh()` for all structures.
- `isHeadless()`, `init(backend)` and `state::backend`.
- `Widget` and `state::widgets`.
- `options::doDefaultMouseInteraction`.

### Options

- `options::programName`: the window title is always "polyscope-rs".
- `options::uiScale`, also its change at run time.
- `options::rightGuiPaneWidth`.
- `options::enableVSync`.
- `options::verbosity` and `options::printPrefix`.
- `options::autocenterStructures` and `options::autoscaleStructures`.
- `options::giveFocusOnShow` and `options::hideWindowAfterShow`.
- `options::warnForInvalidValues`.
- `options::displayMessagePopups`.
- `options::buildGui`, `options::buildDefaultGuiPanels` and `options::userGuiIsOnRightSide`.
- `options::renderScene`.
- `options::groundPlaneHeightFactor` (`GroundPlaneHeightMode` is a boolean in Rust).
- `options::shadowBlurIters`.
- `options::screenshotTransparency` and `options::screenshotExtension` for the UI screenshot.

### View and camera

- `lookAt`, `setViewCenter`, `getViewCenter`, `setNavigateStyle`, `setUpDir`, `setFrontDir`, `setProjectionMode`: only through a `ViewState`.
- `setVerticalFieldOfViewDegrees`, `setClipPlanes`, `getClipPlanes`, `setCameraViewMatrix`, `getCameraViewMatrix`: only through a `ViewState`.
- `getUpVec`, `getFrontVec`, `getLookVec` and `getCameraFrame`.
- `getAspectRatioWidthOverHeight`, `getCameraPerspectiveMatrix` and `getCameraWorldPosition`.
- `getCameraParametersForCurrentView` and `setViewToCamera`.
- `computeHomeView`, `resetCameraToHomeView` and `flyToHomeView`: the Reset View button only.
- `startFlightTo` with a duration, and `immediatelyEndFlight`.
- `moveScale` in a saved view (the Camera panel sets the move speed).
- `ViewRelativeMode`.
- Clip planes that follow the camera distance. The home view sets the far plane to 100 times the scene size.
- `processClipPlaneShift`: Shift+scroll does not move the near plane.
- `updateViewAndChangeNavigationStyle`: a new navigation style does not adjust the view.
- `setWindowSize`, `getWindowSize`, `getBufferSize`, `initWindowPosX`, `initWindowPosY` and `setWindowResizable`.
- `screenCoordsToWorldRay`, `bufferIndsToWorldRay` and `screenCoordsAndDepthToWorldPosition`.
- `screenCoordsToBufferInds` and `bufferIndsToScreenCoords`.
- `getViewAsJson` and `setViewFromJson` in the C++ JSON format.
- Ctrl+C and Ctrl+V to copy and paste the view.
- The alpha channel of `bgColor`.
- Multiple viewports in one window: `createViewportGrid`, `setNumViewports`, `activateViewport`, `setActiveViewportRelativeLocation`, `setActiveViewportRelativeSize` and `setViewportsShareNavigation` (upstream 16b6852).
- The viewport forms of the coordinate functions, such as `screenCoordsToWorldRayViewport`, `screenCoordsGlobalToViewport` and `bufferIndsViewportToGlobal` (upstream 16b6852, 4efedba).

### Picking and selection

- `pickAtScreenCoords` and `pickAtBufferInds`.
- `setSelection`, and `getSelection` with the picked element.
- `PickResult::isHit`, `quantity`, `quantityName`, `screenCoords` and `bufferInds`.
- `interpretPickResult` with typed results: `SurfaceMeshPickResult`, `CurveNetworkPickResult`, `VolumeMeshPickResult`, `VolumeGridPickResult`.
- Quantity picking (upstream 16139ad).
- `MeshSelectionMode` and `SurfaceMesh::setSelectionMode`.
- `SurfaceMesh::selectVertex`.
- `buildPickUI`: the Selection panel shows no quantity values.
- `pickAtScreenCoordsViewport` and `pickAtBufferIndsViewport`, and the `PickResult` fields `viewportInd`, `viewportScreenCoords` and `viewportBufferInds` (upstream 16b6852, 4efedba).

### Interaction and UI

- Ctrl+Shift+left drag to zoom.
- The Controls help window.
- `buildHistogram`: no data histogram in the quantity panel.
- `exportColorbarToSVG`.

### Gizmo

- `addTransformationGizmo`, `getTransformationGizmo`, `removeTransformationGizmo` and `removeAllTransformationGizmos`.
- `setAllowTranslation`, `setAllowRotation`, `setAllowScaling` and `setAllowNonUniformScaling`.
- `TransformationGizmo::setPosition` and `getPosition`.
- The gizmo size: `GizmoConfig::size` has no effect.
- `Structure::setTransformGizmoEnabled` and `Structure::getTransformGizmo`.

### Structures (all types)

- `setTransparency` on point clouds, curve networks, volume meshes, volume grids and camera views.
- `setIgnoreSlicePlane` and `setCullWholeElements`.
- `centerBoundingBox` and `rescaleToUnit`.
- `setPosition`, `translate` and `getPosition`.
- `enableIsolate` and `setEnabledAllOfType`.
- `removeAllQuantities` and `setAllQuantitiesEnabled`.
- `getStructure(type, name)`, `hasStructure` and the typed `has*` functions such as `hasVolumeMesh`.
- `removeStructure(type, name, errorIfAbsent)` and the typed `remove*` functions such as `removeVolumeMesh`.
- `getStructure(type, "")` for the only structure of a type (upstream d3e6ca6).
- The 2D input forms: `registerPointCloud2D`, `registerCurveNetwork2D`, `addVectorQuantity2D` and similar.
- `ScaledValue`: `set_point_radius` and the vector length and radius have no relative or absolute flag.
- `setViewportVisibility`, `setViewportVisibilityAll`, `setViewportVisibilityExclusive` and `getViewportVisibility` (upstream 16b6852).

### Quantities (all types)

- `DataType` (STANDARD, SYMMETRIC, MAGNITUDE, CATEGORICAL) for scalar quantities.
- Isolines: `setIsolinesEnabled`, `setIsolineStyle`, `setIsolinePeriod`, `setIsolineDarkness`, `setIsolineContourThickness`.
- `setOnscreenColorbarEnabled` and `setOnscreenColorbarLocation`.
- `resetMapRange` and `getDataRange`.
- `setColorMap` and `setMapRange` on the handles (only `VolumeGridHandle::set_color_map` exists).
- `updateData` and `updateCoords` for all quantity types.
- `VectorType::AMBIENT`.
- `setVectorLengthRange` and the vector `setMaterial`.
- Parameterization: `setGridColors`, `setColorMap` and `setAltDarkness`.
- `ParamVizStyle::CHECKER_ISLANDS` and `setIslandLabels`.
- `loadColorMap`: no user colormap from an image file.

### Point cloud

- `addParameterizationQuantity` and `addLocalParameterizationQuantity`.
- `setPointRenderMode` with `PointRenderMode::Quad`.
- `setPointRadiusQuantity` and `clearPointRadiusQuantity`.
- `setTransparencyQuantity` and `clearTransparencyQuantity`.

### Surface mesh

- `addEdgeScalarQuantity`, `addHalfedgeScalarQuantity` and `addCornerScalarQuantity`.
- `addTextureScalarQuantity`, `addTextureColorQuantity` and `TextureMapQuantity::setFilterMode`.
- `setEdgePermutation`, `setHalfedgePermutation`, `setCornerPermutation` and `setAllPermutations`.
- `addVertexDistanceQuantity` and `addVertexSignedDistanceQuantity`.
- `addLocalParameterizationQuantity`.
- `createCurveNetworkFromSeams`.
- `setTransparencyQuantity` and `clearTransparencyQuantity`.
- `nCorners` and `nHalfedges`.

### Curve network

- `setNodeRadiusQuantity`, `clearNodeRadiusQuantity`, `setEdgeRadiusQuantity` and `clearEdgeRadiusQuantity` (upstream 2012f51, 258cd7b).

### Volume mesh

- `updateVertexPositions`.
- `nFaces` and `nFacesTriangulation`.
- `SlicePlane::setVolumeMeshToInspect`: no cut surface where a slice plane crosses the mesh.
- The level set: `setLevelSetQuantity`, `setLevelSetValue`, `setEnabledLevelSet`, `setLevelSetVisibleQuantity`.

### Volume grid

- `addNodeScalarQuantityFromCallable`, `addCellScalarQuantityFromCallable` and their batch forms.
- `setGridcubeVizEnabled` and `setIsosurfaceVizEnabled` as two flags: gridcubes and isosurface together.
- `VolumeGridCellScalarQuantity::setGridcubeVizEnabled`.
- `setSlicePlanesAffectIsosurface`.
- `registerIsosurfaceAsMesh(name)`: a UI button only, with a time-stamped name.
- `gridSpacingReference` and `minGridSpacing`.
- `flattenCellIndex`, `unflattenCellIndex` and `positionOfCellIndex`.

### Camera view

- `setViewToThisCamera`: the UI "fly to" action only.
- `CameraIntrinsics::fromFoVDegHorizontalAndVertical`.
- `createInvalid`, `isValid` and `isfinite` of the camera parameter types.
- `CameraExtrinsics::fromMatrix`, and `getT` and `getR` of the extrinsics and parameters.
- `generateCameraRays` and `generateCameraRayCorners`.

### Missing structures

- `SparseVolumeGrid` (upstream 3fa1cb7, 8ac360d).
- `SimpleTriangleMesh`, with its vertex and face scalar and color quantities, vertex and face picks, `updateVertexPositions`, `updateMesh`, `reserveMeshCapacity` and edge wireframe (upstream cc3bcca, 8231aec, 296906a, 96d73bc).

### Images and implicit surfaces

- The display of floating images: `setShowFullscreen`, `setShowInImGuiWindow`, `setShowInCameraBillboard`, `setTransparency`.
- The image options: `ImageOrigin` and `DataType` arguments, and a handle for the image.
- `addColorAlphaImageQuantity` and `setIsPremultiplied`.
- `addDepthRenderImageQuantity`, `addColorRenderImageQuantity` and `addRawColorRenderImageQuantity`.
- `addScalarRenderImageQuantity` and `addRawColorAlphaRenderImageQuantity`.
- Render image options and updates: `setMaterial`, `setAllowFullscreenCompositing`, `updateBuffers`.
- Render images in depth peeling and under slice planes (upstream cc4d603, 8968b55, 7d3b9bc).
- Image quantities on structures: `Structure::addScalarImageQuantity` and the other `add*ImageQuantity` functions.
- `renderImplicitSurface` and its variants, `ImplicitRenderOpts` and `ImplicitRenderMode`.

### Slice planes

- `addSceneSlicePlane` and `removeLastSceneSlicePlane`.
- `SlicePlane::setGridLineColor`.
- `SlicePlane::setTransform` and `getTransform` on `SlicePlaneHandle`.

### Groups

- `Group::unparent`.
- `Group::getTopLevelGrandparent`.
- `Group::setHideDescendantsFromStructureLists`.

### Screenshots

- `ScreenshotOptions::includeUI` and `options::screenshotWithImGuiUI`.
- `screenshotToBuffer` for the running window.
- `resetScreenshotIndex`.

### Messages

- `info`, `warning`, `error`, `terminatingError` and `exception`.
- `showDelayedWarnings` and `clearMessages`.

---

## Differences from C++ Polyscope

These features exist in both libraries but behave differently.

- **Curve network render mode.** A new curve network draws one-pixel lines. Call `set_render_mode(1)` for tubes. C++ always draws tubes.
- **Gridcube size.** `set_cube_size_factor` sets a size: 1 is the full cell (the default) and 0 hides the cubes. In C++, `setCubeSizeFactor` sets a shrink amount: 0 is the full cell.
- **Tone mapping curve.** The Reinhard curve acts on each color channel. C++ applies it to the luminance, so bright saturated colors differ a little.
- **Front direction.** In polyscope-rs, the front direction is the direction that the camera looks along. In C++, it is the side where the camera sits.
- **Home view.** The home view distance comes from the field of view and the bounding sphere. C++ uses 1.5 times the length scale and an offset along the up direction.
- **`TransparencyMode::None`.** It renders like `Simple`. In C++, `None` turns transparency off.
- **Default values.** The ground plane mode is Tile; C++ uses TileReflection.
- **More default values.** The shadow darkness is 0.4; C++ uses 0.25. The transparency mode is Simple; C++ uses None.
- **Screenshot background.** `ScreenshotOptions::transparent_background` is false by default. C++ uses a transparent background by default.
- **View files.** The JSON format is Rust-native. It stores the field of view in radians and also stores the render look. C++ view files do not load.
- **Scalar colors.** Point clouds, surface meshes, curve networks and volume meshes compute one scalar color per element on the CPU. The GPU interpolates vertex colors across faces. C++ samples the colormap per pixel.
- **Shadows.** Shadows show in the Tile, TileReflection and ShadowOnly modes. C++ shows a shadow in the ShadowOnly mode only, from all structures.
- **Pretty transparency.** Depth peeling applies to surface meshes only. C++ peels all structures and the ground plane.
- **Volume mesh slicing.** A slice plane removes each cell whose center is on the cut side. No cut surface shows.
- **Slice plane count.** The GPU applies at most 4 slice planes. If more planes exist, only 4 of them act.
- **Slice plane names.** `add_slice_plane` with a name in use moves the existing plane to the scene center. C++ reports an error.
- **Slice plane gizmo.** It uses the world or local space of the Transform panel. C++ always uses local space. In scale mode, it shows the translate and rotate handles.
- **Groups.** `GroupHandle::set_enabled` sets the flag of the group only. C++ also sets the enabled flag of each child group and structure.
- **Picks.** A click on a surface mesh selects a face, and a click on a curve network selects an edge. C++ can also pick vertices, edges, corners and nodes.
- **File drop.** The callback receives one path per call. winit sends one event for each dropped file.
- **Handles.** The handles have fewer setters than the structure types. Use the `with_*` functions for the other setters.
- **Duplicate names.** A `register_*` call with a name in use panics.

## Known Issues

- **Intermittent SIGSEGV on WSL2.** Under WSL2 with GPU passthrough, the application can stop with SIGSEGV (exit code 139) inside the GPU driver.
- **SSAO and SSAA.** SSAO has no effect when the SSAA factor is above 1.
- **SSAO sample count.** The SSAO pass uses at most 32 samples. A larger `sample_count` has no further effect.
- **Pretty mode depth precision.** The min-depth texture of depth peeling is `Rgba16Float`, because WebGPU cannot blend `R32Float` without the optional `float32-blendable` feature. Thus the depth epsilon is `2e-3` (C++: `1e-6`). Pretty transparency can merge layers that are closer than about 0.002 in depth.
- **Pretty mode non-linear opacity.** Depth peeling renders both faces of closed meshes. The effective alpha is `2a - a^2`. C++ Polyscope gives the same result.
- **Tone mapping values.** Exposure, white level and gamma are not part of `Options` or of a saved view. Only the Tone Mapping panel sets them.
- **Changes from another thread during a frame.** The scene passes and the pick pass read the registry under separate locks. If another thread hides, shows, removes or updates a structure between them, the pick buffer of that frame and the window can differ. The next frame shows the change in both. A transform change does not cause this, because the pick pass uses the pick uniforms that the frame preparation writes together with the render uniforms.

---

## Roadmap

The order of the tiers is the order of priority. No dates are planned.

### Tier 1

- [ ] User callback and frame loop: per-frame user callback, `frame_tick`, `unshow`.
- [ ] Global view and camera API: look-at, view center, home view, clip planes, field of view, and programmatic picking.
- [ ] Display of floating images and render images.
- [ ] Scalar quantities: `DataType`, isolines, on-screen colorbar, and `update` methods for all quantity types.

### Tier 2

- [ ] Curve networks: upload the edge positions in object space and apply the model matrix in the edge and tube shaders. Today the CPU transforms the edge positions, which breaks the project rule that GPU positions are never in world space.
- [ ] Curve networks: draw tubes by default, as C++ does. Today a new curve network draws one-pixel lines until `set_render_mode(1)`.
- [ ] The missing quantities and options of each structure (see [Not ported yet](#not-ported-yet)).
- [ ] Sparse Volume Grid.
- [ ] SimpleTriangleMesh.
- [ ] Per-structure slice plane options: `setIgnoreSlicePlane`, `setCullWholeElements`, `setVolumeMeshToInspect`.

### Tier 3

- [ ] Gizmo overhaul with user gizmos.
- [ ] Remaining options: UI scale, panel width, window size, vsync, program name.
- [ ] Messages API.
- [ ] Group helpers: `unparent`, top-level lookup, hidden descendants.
- [ ] Multiple viewports: a viewport grid, a camera for each viewport or shared navigation, and a viewport visibility for each structure.

---

## Upstream Sync Point

C++ Polyscope `master` at commit 4efedba (2026-11-25) was audited on 2026-11-26. It has no newer commits.

### Completed upstream ports

- [x] 7b2bbd1: turntable pitch stops before the view aligns with the up axis.
- [x] 129c680: turntable pan moves the view center; turntable rotation does not drift.
- [x] c0b5310: the first-person look rate does not depend on the move speed.
- [x] 067f760: camera flights interpolate the inverse view matrix.
- [x] 1e5ff25: a change of the up direction keeps a valid view.
- [x] 61fc32a: a double-click sets the view center on any structure, and the view flies there.
- [x] cd131e0 (partial): Ctrl+Shift+click sets the view center; rotation pivots on it. No API for the view center.
- [x] 022deea: a single click selects after the double-click time; a double-click cancels it.
- [x] 1b41445 (partial): scroll zoom is relative to the view distance; Planar re-centering moves inside the plane. No `ViewRelativeMode`.
- [x] d034498 (partial): the near clip plane comes from the scene size. The far plane does not follow the camera.
- [x] 7570a40 and 59da72d (partial): view save and load with navigation style, up and front direction, view center and field of view. Rust-native format; no `viewRelativeMode`.
- [x] 0ff26c2 and e519efb: files-dropped callback; `remove_everything` clears it.
- [x] 8cbdf24: `shutdown()` clears the files-dropped callback.
- [x] f34f403: `remove_everything()` and `remove_all_groups()`.
- [x] 24ec7e3: `add_slice_plane(name)`, `add_slice_plane_auto()`, `get_slice_plane`, `remove_slice_plane`, `SlicePlaneHandle::remove`.
- [x] 69f0635: `SlicePlaneHandle::origin` and `SlicePlaneHandle::normal`.
- [x] e91a709: the volume grid isosurface has the correct X and Z order for non-uniform grids.
- [x] 51953c2: ray-cast spheres and tubes are correct in orthographic projection; the ground plane follows the up direction.
- [x] 3198ab5: a degenerate bounding box gets a tolerance; a one-point scene has a nonzero length scale.
- [x] dcbaedb (partial): prism and pyramid cells; the wireframe hides the diagonals of polygon faces. No "Inspect with slice plane".
- [x] 8740ddc: the SSAO noise uses a fixed seed, so it is the same on every run.
- [x] d7a2a04 (partial): `ScreenshotOptions::transparent_background`. No `includeUI` and no `screenshotToBuffer`.
- [x] 502c80c (partial): the gizmo has a mode selector and snap increments. No user gizmos.
- [x] 08fd238: the Reset View button refreshes the scene extents before it sets the home view. Structure bounds need no forced recompute, because each call computes them.

### Upstream changes that need no port

- 393bc80 to 243bb3a, 47d6133 and 93476ea: the `ManagedBuffer` refactor and the OpenGL texture types. polyscope-rs has its own wgpu buffers.
- a206e53: the C++ `SimpleTriangleMesh` shader now premultiplies its color, as the C++ surface mesh shader does. polyscope-rs has no `SimpleTriangleMesh`, and its depth-peel shader premultiplies (`surface_mesh_peel.wgsl`).
- 16b6852 (part): the projection mode is a run-time uniform. polyscope-rs has no shader variant for a projection mode.
- 16b6852 (part): the pick read uses the row `height - 1 - y`. polyscope-rs reads row `y`, because wgpu textures start at the top left.
- 16b6852 (part): more redraw requests for camera changes and camera flights. polyscope-rs draws every frame.
- 9a562d2, ce256d0 and 3da0aeb: fixes for the C++ CI and the MSVC compiler.

### Rust-only features

- **SSAO**: hemisphere sampling with normal-aware weights and a depth-aware bilateral blur.
- **RGBA color quantities**: per-element alpha on surface mesh vertex and face colors. C++ color quantities are RGB.
- **Tube picking for curve networks**: a ray-cylinder pick shader for thin curves.

---

## Mouse Controls

| Action | Input |
|--------|-------|
| Rotate (orbit) | Left drag (Turntable, Free, Arcball) |
| Look around | Left drag (First Person) |
| Pan | Right drag, or Shift + left drag |
| Zoom | Scroll wheel |
| Select | Left click without drag and without Shift, Ctrl or Alt |
| Clear the selection | Right click without drag, or left click on the background |
| Set the view center | Double-click, or Ctrl + Shift + left click (Cmd + Shift on macOS) |
| Move (First Person) | W, A, S, D, Q, E |
| Screenshot | F12 |
| Exit | Esc |

Planar navigation does not rotate. First Person navigation does not pan or zoom. None navigation ignores all camera input. If nothing is selected, a single click selects after 0.35 s. If a selection exists, the click applies at once.

## Technology Stack

| Component | C++ Polyscope | polyscope-rs |
|-----------|---------------|--------------|
| Graphics API | OpenGL | wgpu 30 (Vulkan, Metal, DirectX 12, WebGPU) |
| Shader Language | GLSL | WGSL |
| Windowing | GLFW | winit 0.30 |
| Math | GLM | glam 0.34 |
| UI | Dear ImGui | egui 0.36 |
| Gizmo | ImGuizmo | transform-gizmo-egui 0.11 |
| File dialogs | — | rfd 0.18 |
| Image Loading | stb_image | image 0.25 |
| JSON | nlohmann/json | serde_json |
| Build | CMake | Cargo (Rust 1.96 or later) |

## Platform Support

| Platform | wgpu backend | Status |
|----------|--------------|--------|
| Linux | Vulkan | Tested (WSL2) |
| Windows | DirectX 12 or Vulkan | Not tested |
| macOS | Metal | Not tested |
| Web | WebGPU | Not tested |

The engine uses only the Vulkan, Metal, DirectX 12 and WebGPU backends of wgpu. It does not support the OpenGL backend, because some shaders do not compile there.

If no supported backend is available:

- `render_to_image` and `render_to_file` return an error that contains the message "no graphics adapter found (Vulkan, Metal or DirectX 12 is necessary)".
- `show()` stops with a panic that contains the same message.
