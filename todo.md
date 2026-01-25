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

## Future Work: Full GPU Picking (for more accurate picking)

The current screen-space picking is an approximation. For pixel-perfect picking:

- [ ] Create pick buffer texture in RenderEngine
- [ ] Create pick render pipeline for each structure type
- [ ] Render structures to pick buffer with unique color IDs
- [ ] Read back pixel at click position using staging buffer
- [ ] Decode color to determine clicked structure/element
- [ ] Add pick shaders for SurfaceMesh, CurveNetwork, etc.

## Current Limitations
- **Sparse point clouds**: Clicks between points may not register because we only sample up to 100 points. For very sparse point clouds, you may need to click directly on a visible point.
- **Accuracy**: Screen-space picking uses a 20px threshold, which may select unintended structures if they are close together in screen space.
- **Performance**: For structures with many elements, sampling is limited to 100 points for efficiency.

## Notes
- The `pick.wgsl` shader exists for PointCloud but the pick buffer infrastructure in RenderEngine is not implemented
- GPU picking would give pixel-perfect results but requires more infrastructure (pick buffer, staging buffer readback, etc.)
