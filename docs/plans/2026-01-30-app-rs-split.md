# Plan: Split app.rs into multiple modules

## Problem
`crates/polyscope/src/app.rs` is 3501 lines - the only file exceeding 2000 lines. It contains four logically distinct sections that should be separate modules for maintainability.

## Split Structure

```
crates/polyscope/src/app/
├── mod.rs      (~300 lines) - struct App, constructor, Default, run_app()
├── picking.rs  (~490 lines) - All ray-casting and GPU picking methods
├── render.rs   (~1960 lines) - render() and capture_screenshot()
└── input.rs    (~600 lines) - impl ApplicationHandler for App
```

## Module Boundaries

### mod.rs (lines 1-137, 3489-3501)
- All imports and `use` statements
- `pub struct App { ... }` definition
- `App::new()`, `request_auto_screenshot()`, `set_background_color()`
- `impl Default for App`
- `pub fn run_app()`
- Module declarations: `mod picking; mod render; mod input;`

### picking.rs (lines 138-654)
Methods on `impl App`:
- `gpu_pick_at()` - GPU pick buffer lookup
- `pick_structure_at_screen_pos()` - Screen-space picking dispatch
- `screen_ray()` - Screen position to 3D ray
- `pick_slice_plane_at_ray()` - Slice plane intersection
- `ray_intersect_triangle()` - Ray-triangle intersection test
- `ray_segment_closest_t()` - Ray-segment closest approach
- `pick_structure_at_ray()` - Ray-based structure picking
- `pick_point_cloud_at_ray()` - Point cloud ray picking
- `pick_curve_network_edge_at_ray()` - Curve network ray picking
- `select_slice_plane_by_name()` - Slice plane selection
- `deselect_slice_plane_selection()` - Slice plane deselection

### render.rs (lines 656-2891)
Methods on `impl App`:
- `render()` - Main render loop (~1960 lines)
- `capture_screenshot()` - Screenshot rendering (~270 lines)

### input.rs (lines 2893-3487)
- `impl ApplicationHandler for App` - All event handling
  - `resumed()` - Window/engine initialization
  - `window_event()` - Mouse, keyboard, resize, redraw, close events

## Implementation Steps

1. Create `crates/polyscope/src/app/` directory
2. Move `app.rs` to `app/mod.rs`
3. Extract picking methods to `app/picking.rs`
4. Extract render/screenshot to `app/render.rs`
5. Extract ApplicationHandler impl to `app/input.rs`
6. Add `use super::*;` in each submodule (all types are crate-private)
7. Add module declarations in `mod.rs`
8. Verify `cargo build` succeeds
9. Verify `cargo test` passes
10. Verify `cargo clippy` clean

## Visibility Notes
- `mod app` is private in lib.rs (no `pub use`)
- Only `crate::app::run_app()` is accessed externally (from init.rs)
- All struct fields and methods can remain as-is (crate-private visibility)
- Submodules use `use super::*;` to access App and its imports
