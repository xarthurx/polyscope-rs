# Polyscope-rs TODO

## Completed: Gizmo and Selection System

### Completed
- [x] Remove deselection on right-click (right-click is for camera rotation only)
- [x] Add model matrix support to structure rendering (gizmo transforms now work)
- [x] Fix viewport mismatch between gizmo and 3D rendering
- [x] Update GPU buffers immediately after gizmo interaction
- [x] Implement screen-space picking to detect clicks on structures
  - Projects sample points from structures to screen space
  - Checks if click position is near any projected point (20px threshold)
  - Selects structure if clicked on it, deselects if clicked on empty space
- [x] Fix event handling to allow deselection in 3D viewport
  - Replaced egui's CentralPanel with Area for gizmo overlay
  - Area uses `interactable(false)` to allow clicks to pass through
  - Camera control disabled when gizmo is being manipulated (using `is_using_pointer()`)
- [x] Rewrite mouse event handling for robustness
  - Physical button state tracked separately from egui event consumption
  - Prevents mouse state from getting "stuck" when egui intercepts events
  - Added proper modifier key tracking (Shift) for pan mode
  - Added drag distance accumulation for click vs drag detection
  - Fixed: Mouse position check to differentiate UI panel vs 3D viewport
  - Fixed: Only block events in UI panel, allow 3D viewport picking even with gizmo visible
  - Fixed: Check `is_using_pointer()` to skip picking when gizmo is being dragged

## Mouse Controls (matching C++ Polyscope)

- **Left drag** (no modifiers): Rotate/orbit camera
- **Shift + Left drag**: Pan camera
- **Right drag**: Pan camera
- **Scroll wheel**: Zoom
- **Left click** (no drag): Select structure at click position
- **Right click** (no drag): Clear selection/deselect

## Next: GPU Picking (Element-level)

Replace screen-space approximation with pixel-perfect GPU picking. Returns exact element clicked (point #42, face #123, etc.).

### Phase 1: Pick Buffer Infrastructure
- [ ] Create pick buffer texture (RGBA8, same size as viewport) in RenderEngine
- [ ] Create pick depth texture for proper occlusion
- [ ] Add staging buffer for GPU→CPU readback
- [ ] Add `pick_at(x, y)` method that reads single pixel from staging buffer

### Phase 2: Structure ID Encoding
- [ ] Design encoding scheme: structure_id (12 bits) + element_id (12 bits) = 24 bits RGB
- [ ] Create structure registry that assigns unique IDs to each structure
- [ ] Update PickResult to include both structure and element info

### Phase 3: Pick Shaders
- [ ] Update existing `pick.wgsl` for PointCloud (already exists, needs integration)
- [ ] Create `pick_mesh.wgsl` for SurfaceMesh (render faces with encoded face index)
- [ ] Create `pick_curve.wgsl` for CurveNetwork (render edges with encoded edge index)
- [ ] Create `pick_volume.wgsl` for VolumeMesh (render cells with encoded cell index)

### Phase 4: Pick Pipelines
- [ ] Create pick pipeline for PointCloud using pick.wgsl
- [ ] Create pick pipeline for SurfaceMesh
- [ ] Create pick pipeline for CurveNetwork
- [ ] Create pick pipeline for VolumeMesh
- [ ] Create pick pipeline for VolumeGrid
- [ ] Create pick pipeline for CameraView

### Phase 5: Integration
- [ ] Add `render_pick_pass()` method to RenderEngine
- [ ] Call pick pass on click (not every frame)
- [ ] Replace `pick_structure_at_screen_pos()` in app.rs with GPU picking
- [ ] Update selection UI to show element info (type, index)

### Phase 6: Testing
- [ ] Test picking accuracy with overlapping structures
- [ ] Test picking with transformed structures (model matrix)
- [ ] Test picking at viewport edges
- [ ] Test picking with slice planes active

## Future Work: Hover Highlighting (Option 3)
After GPU picking is complete, can add:
- [ ] Hover glow effect (pick every frame on mouse move)
- [ ] Selection outline shader
- [ ] Element info panel showing data values

## Current Limitations (to be removed after GPU picking)
- **Sparse point clouds**: Clicks between points may not register because we only sample up to 100 points. For very sparse point clouds, you may need to click directly on a visible point.
- **Accuracy**: Screen-space picking uses a 20px threshold, which may select unintended structures if they are close together in screen space.
- **Performance**: For structures with many elements, sampling is limited to 100 points for efficiency.

## Notes
- The `pick.wgsl` shader exists for PointCloud but the pick buffer infrastructure in RenderEngine is not implemented
- GPU picking would give pixel-perfect results but requires more infrastructure (pick buffer, staging buffer readback, etc.)
