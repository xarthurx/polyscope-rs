# Upstream C++ Polyscope Follow-ups Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Port relevant bug fixes and UX improvements from recent C++ Polyscope commits to polyscope-rs.

**Architecture:** Three independent features touching the App struct's event handling (double-click, drag-and-drop) and the camera module (turntable drift fix). Each task is self-contained with no cross-dependencies.

**Tech Stack:** Rust, winit (event handling), polyscope-render (camera), polyscope-core (state/options)

---

## Task Overview

| # | Feature | Upstream Commit | Files | Effort |
|---|---------|-----------------|-------|--------|
| 1 | Double-click to set view center | `61fc32a` | `app.rs` | Small |
| 2 | Turntable orbit drift prevention | `129c680` | `camera.rs` | Small |
| 3 | Drag & drop file callback | `0ff26c2` | `app.rs`, `state.rs`, `lib.rs` | Small-Medium |

**Not ported (with rationale):**
- Orthographic ray-casting fix (`51953c2`) — N/A, we use instanced mesh geometry not raycasting
- Tangent vector bug (`892a332`) — N/A, same reason (shader rules system is OpenGL-specific)
- VolumeGrid isosurface indexing (`e91a709`) — N/A, we don't have isosurface rendering yet
- Colormap range sync (`ac0a2e1`) — Already works (our UI mutates range refs directly)
- Custom slice plane names (`24ec7e3`) — Already supported (SlicePlane has `name: String`)
- Gizmo overhaul (`502c80c`) — N/A, we use `transform-gizmo-egui`, not ImGuizmo
- Slice planes for floating quantities (`8968b55`) — N/A, our floating quantities are 2D egui overlays, not 3D geometry
- Template simplification (`19253de`) — C++ internal refactor, no Rust equivalent

---

### Task 1: Double-click to set view center

**Upstream:** `61fc32a` — Double-click or Ctrl+Shift+click sets the camera orbit center to the clicked 3D point.

**Files:**
- Modify: `crates/polyscope/src/app.rs` — App struct (add fields), `window_event()` (add double-click detection)

**Step 1: Add double-click tracking fields to App struct**

Add to `App` struct (after `last_click_pos` field, around line 43):

```rust
// Double-click detection
last_left_click_time: Option<std::time::Instant>,
last_left_click_screen_pos: Option<(f64, f64)>,
```

Initialize in `App::new()`:

```rust
last_left_click_time: None,
last_left_click_screen_pos: None,
```

**Step 2: Add double-click detection in left mouse release handler**

In `window_event()`, inside the `MouseButton::Left, ElementState::Released` branch (line 2604), before the existing click-vs-drag check, add double-click detection:

```rust
// Double-click detection
const DOUBLE_CLICK_TIME_MS: u128 = 500;
const DOUBLE_CLICK_DIST: f64 = 10.0;

let is_double_click = if let (Some(prev_time), Some(prev_pos)) =
    (self.last_left_click_time, self.last_left_click_screen_pos)
{
    let elapsed = prev_time.elapsed().as_millis();
    let dist = ((self.mouse_pos.0 - prev_pos.0).powi(2)
        + (self.mouse_pos.1 - prev_pos.1).powi(2))
    .sqrt();
    elapsed < DOUBLE_CLICK_TIME_MS && dist < DOUBLE_CLICK_DIST
} else {
    false
};

// Always record this click for next double-click check
if self.drag_distance < DRAG_THRESHOLD {
    self.last_left_click_time = Some(std::time::Instant::now());
    self.last_left_click_screen_pos = Some(self.mouse_pos);
} else {
    // Drags reset double-click tracking
    self.last_left_click_time = None;
    self.last_left_click_screen_pos = None;
}
```

**Step 3: Handle double-click to set view center**

After double-click detection, before the existing click handling, add:

```rust
if is_double_click && !mouse_in_ui_panel {
    // Double-click: set camera target to the clicked 3D point
    if let Some(engine) = &mut self.engine {
        let click_screen =
            glam::Vec2::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
        if let Some((ray_origin, ray_dir)) = self.screen_ray(
            click_screen,
            engine.width,
            engine.height,
            &engine.camera,
        ) {
            // Try to find the closest hit point using existing picking logic
            let plane_params = crate::with_context(|ctx| {
                ctx.slice_planes()
                    .filter(|p| p.is_enabled())
                    .map(|p| (p.origin(), p.normal()))
                    .collect::<Vec<_>>()
            });

            // Check structure hit for distance
            if let Some((_type_name, _name, t)) =
                self.pick_structure_at_ray(ray_origin, ray_dir, &plane_params)
            {
                let hit_point = ray_origin + ray_dir * t;
                engine.camera.target = hit_point;
                log::debug!(
                    "Double-click: set view center to ({:.3}, {:.3}, {:.3})",
                    hit_point.x, hit_point.y, hit_point.z
                );
            }
        }
    }
    // Reset double-click state so triple-click doesn't re-trigger
    self.last_left_click_time = None;
    self.last_left_click_screen_pos = None;
    self.last_click_pos = None;
    return;
}
```

Note: `self.screen_ray()` borrows `self` immutably but we need `&mut self.engine`. We'll need to factor this so we call `screen_ray` before the `&mut self.engine` borrow, or restructure slightly. The exact approach will depend on borrow checker constraints at implementation time.

**Step 4: Build and test**

```bash
cargo build --examples
cargo test
```

Manual test: Run demo, double-click on a structure — camera should re-center on the clicked point. Single-click should still work for picking.

**Step 5: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: add double-click to set view center (upstream 61fc32a)"
```

---

### Task 2: Turntable orbit drift prevention

**Upstream:** `129c680` — After orbit rotation in turntable mode, re-enforce `lookAt()` to prevent numerical drift where the view center shifts relative to the view matrix.

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs` — `orbit()` method

**Step 1: Examine current orbit method**

Current `orbit()` (camera.rs lines 237-251):
```rust
pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
    let offset = self.position - self.target;
    let distance = offset.length();

    // Calculate current angles
    let current_pitch = (offset.y / distance).asin();
    let current_yaw = offset.z.atan2(offset.x);

    // Apply deltas with pitch clamping
    let new_pitch = (current_pitch - delta_pitch).clamp(-1.5, 1.5);
    let new_yaw = current_yaw - delta_yaw;

    // Convert back to position
    self.position = self.target + Vec3::new(
        distance * new_pitch.cos() * new_yaw.cos(),
        distance * new_pitch.sin(),
        distance * new_pitch.cos() * new_yaw.sin(),
    );
}
```

The C++ fix adds a `lookAt()` call after orbit to prevent the view from drifting. Our orbit directly computes position from target + angles, which is already drift-resistant since `self.target` is the source of truth. However, we should still enforce that the camera looks at the target after orbit.

**Step 2: Add lookAt enforcement after orbit**

Add at the end of `orbit()`, after updating `self.position`:

```rust
// Re-enforce look-at to prevent numerical drift in turntable mode.
// This ensures the camera always points exactly at the target.
let look_dir = (self.target - self.position).normalize_or_zero();
if look_dir.length_squared() > 0.5 {
    // Recompute position to be exactly `distance` from target along the look direction
    self.position = self.target - look_dir * distance;
}
```

**Step 3: Build and test**

```bash
cargo build
cargo test
```

Manual test: Extended orbiting should not cause the view center to visually drift.

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/camera.rs
git commit -m "fix: enforce lookAt after orbit to prevent turntable drift (upstream 129c680)"
```

---

### Task 3: Drag & drop file callback

**Upstream:** `0ff26c2` — Add a customizable callback invoked when files are dropped onto the window.

**Files:**
- Modify: `crates/polyscope-core/src/state.rs` — Add callback field to Context
- Modify: `crates/polyscope/src/lib.rs` — Add public API for setting the callback
- Modify: `crates/polyscope/src/app.rs` — Handle `WindowEvent::DroppedFile`

**Step 1: Add callback storage to Context**

In `crates/polyscope-core/src/state.rs`, add to the `Context` struct:

```rust
/// Callback invoked when files are dropped onto the window.
pub file_drop_callback: Option<Box<dyn FnMut(&[std::path::PathBuf]) + Send + Sync>>,
```

Initialize as `None` in `Context::new()`.

**Step 2: Add public API**

In `crates/polyscope/src/lib.rs`, add:

```rust
/// Sets a callback that is invoked when files are dropped onto the polyscope window.
///
/// The callback receives a slice of file paths that were dropped.
///
/// # Example
/// ```no_run
/// polyscope::set_file_drop_callback(|paths| {
///     for path in paths {
///         println!("Dropped: {}", path.display());
///     }
/// });
/// ```
pub fn set_file_drop_callback(callback: impl FnMut(&[std::path::PathBuf]) + Send + Sync + 'static) {
    with_context_mut(|ctx| {
        ctx.file_drop_callback = Some(Box::new(callback));
    });
}

/// Clears the file drop callback.
pub fn clear_file_drop_callback() {
    with_context_mut(|ctx| {
        ctx.file_drop_callback = None;
    });
}
```

**Step 3: Handle WindowEvent::DroppedFile in app.rs**

In `window_event()`, add a new arm in the main `match event` block (before the `_ => {}` catch-all):

```rust
WindowEvent::DroppedFile(path) => {
    log::info!("File dropped: {}", path.display());
    crate::with_context_mut(|ctx| {
        if let Some(callback) = &mut ctx.file_drop_callback {
            callback(&[path]);
        }
    });
}
```

Note: winit fires one `DroppedFile` event per file. If multiple files are dropped at once, we get multiple events. The C++ version batches them. For simplicity we pass one at a time — the callback signature accepts a slice so it's forward-compatible with batching later.

**Step 4: Build and test**

```bash
cargo build --examples
cargo test
```

Manual test: Set a file drop callback in an example, drag a file onto the window, verify the callback fires.

**Step 5: Commit**

```bash
git add crates/polyscope-core/src/state.rs crates/polyscope/src/lib.rs crates/polyscope/src/app.rs
git commit -m "feat: add file drop callback support (upstream 0ff26c2)"
```

---

## Post-Implementation

After all tasks are complete:

1. Run full test suite: `cargo test`
2. Run clippy: `cargo clippy`
3. Run formatter: `cargo fmt`
4. Update documentation if needed
