# Camera Navigation Modes Implementation

## Overview

Implement all 6 camera navigation modes matching C++ Polyscope: Turntable, Free, Planar, Arcball, None, FirstPerson. Currently all modes produce identical Turntable behavior because the input handler doesn't check the navigation style.

## Behavior Matrix

| Mode | Left Drag | Shift+Left / Right Drag | Scroll | WASD Keys |
|------|-----------|------------------------|--------|-----------|
| Turntable | Orbit (gimbal-lock protected, forced lookAt) | Pan (moves view center) | Zoom | -- |
| Free | Free orbit (camera-local up/right) | Pan | Zoom | -- |
| Planar | Blocked | Pan | Zoom | -- |
| Arcball | Sphere-mapped rotation | Pan | Zoom | -- |
| None | Blocked | Blocked | Blocked | -- |
| FirstPerson | Mouse look (yaw/pitch) | Blocked | Blocked | W/S fwd/back, A/D strafe, Q/E up/down |

## Files Changed

1. `crates/polyscope-render/src/camera.rs` - Add Arcball variant, add per-mode camera methods
2. `crates/polyscope/src/app.rs` - Branch input handling on navigation style, add WASD key tracking
3. `crates/polyscope-ui/src/panels.rs` - Add Arcball to UI dropdown
4. `crates/polyscope/src/lib.rs` - Update settings sync for 6 modes

## Camera Methods

- `orbit_turntable(dx, dy)` - Gimbal-lock protected orbit, forced lookAt
- `orbit_free(dx, dy)` - Unconstrained rotation using camera-local axes
- `orbit_arcball(start, end)` - Unit sphere mapped rotation
- `mouse_look(dx, dy)` - First-person yaw/pitch (pre-multiply)
- `move_first_person(delta, dt)` - Camera-local WASD translation

## Input Handling

- Add `keys_down: HashSet<KeyCode>` to App for WASD tracking
- KeyboardInput handler: insert/remove keys
- CursorMoved handler: match on navigation_style for rotation dispatch
- Per-frame update: if FirstPerson + keys pressed, apply movement with delta time
