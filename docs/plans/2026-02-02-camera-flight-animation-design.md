# Camera Flight Animation Design

**Goal:** Implement smooth animated camera flight for the "fly to" button on CameraView structures, matching C++ Polyscope's `startFlightTo()` / `updateFlight()` behavior.

**Approach:** Dual quaternion interpolation of the view matrix with smoothstep easing, matching the C++ implementation exactly.

## Architecture

### Dual Quaternion Module (`polyscope-render/src/dual_quat.rs`)

Minimal dual quaternion implementation (~60 lines). A dual quaternion is a pair of regular quaternions `(real, dual)` that encodes a rigid body transformation (rotation + translation).

Operations needed:
- `from_rotation_translation(rot: Mat3, t: Vec3)` — construct from rotation matrix + translation
- `to_rotation_translation() -> (Mat3, Vec3)` — decompose back
- `lerp(a, b, t) -> DualQuat` — dual linear blend interpolation
- `normalize() -> DualQuat` — renormalize after interpolation

The `dual` part encodes translation as: `dual = 0.5 * Quat::from_xyzw(t.x, t.y, t.z, 0) * real`.

### CameraFlight Struct (in `camera.rs`)

```rust
pub struct CameraFlight {
    start_time: std::time::Instant,
    duration: f32,             // seconds (default 0.4, matching C++)
    initial_view_r: DualQuat,  // start view rotation+translation
    target_view_r: DualQuat,   // end view rotation+translation
    initial_fov: f32,          // start FOV in radians
    target_fov: f32,           // end FOV in radians
}
```

Lives on `Camera` as `pub flight: Option<CameraFlight>`.

### Camera Methods

- `start_flight_to(params: &CameraParameters, duration: f32)`:
  1. Build current view matrix via `self.view_matrix()`
  2. Build target view matrix from CameraParameters
  3. Decompose both into DualQuat (split 3x3 rotation + translation, convert rotation to dual quat)
  4. Store as `CameraFlight`

- `update_flight()` (called every frame):
  1. If no active flight, return
  2. Compute `t = elapsed.as_secs_f32() / duration`
  3. If `t >= 1.0`: set camera to final position exactly, clear flight
  4. Else: `t_smooth = smoothstep(0, 1, t)` where `smoothstep = 3t^2 - 2t^3`
  5. Interpolate dual quats: `lerp(initial, target, t_smooth)`
  6. Interpolate FOV linearly: `(1-t) * initial + t * target` (note: C++ uses raw `t`, not `t_smooth` for FOV)
  7. Convert interpolated dual quat back to view matrix
  8. Extract position, target, up from view matrix and assign to Camera

- `cancel_flight()`: set `flight = None`

### View Matrix Decomposition

To go from view matrix to camera position/target/up:
- The view matrix `V = [R | t]` where R is 3x3 rotation, t is translation
- Camera position = `-R^T * t` (inverse of view transform)
- Camera forward = `-R[2]` (negated third row in RH system)
- Camera up = `R[1]` (second row)
- Target = position + forward * distance (use current camera distance or a reasonable default)

## Integration Points

### render.rs — Frame Update
At the top of `render()`, before UI building:
```rust
engine.camera.update_flight();
```

### render_ui.rs — Trigger Flight
Replace the instant camera set (lines 597-610) with:
```rust
if let Some(params) = &fly_to_camera {
    engine.camera.start_flight_to(params, 0.4);
}
```

### input.rs — Cancel on User Input
Any camera-manipulating input (mouse drag for orbit/pan, scroll for zoom, WASD movement) cancels the active flight:
```rust
engine.camera.cancel_flight();
```

## Files Changed

| File | Change |
|------|--------|
| `crates/polyscope-render/src/dual_quat.rs` | New — minimal dual quaternion math |
| `crates/polyscope-render/src/lib.rs` | Add `mod dual_quat;` |
| `crates/polyscope-render/src/camera.rs` | Add `CameraFlight`, `start_flight_to()`, `update_flight()`, `cancel_flight()` |
| `crates/polyscope/src/app/render.rs` | Add `update_flight()` call at frame start |
| `crates/polyscope/src/app/render_ui.rs` | Replace instant camera set with `start_flight_to()` |
| `crates/polyscope/src/app/input.rs` | Cancel flight on camera input |

## C++ Reference

From `~/repo/polyscope/src/view.cpp` lines 786-840:
- `startFlightTo()`: captures start/end as dual quaternions, sets start/end time
- `updateFlight()`: each frame, computes normalized time, applies smoothstep, lerps dual quats and FOV, reconstructs view matrix
- Flight duration: default 0.4 seconds
- Smoothstep: `glm::smoothstep(0, 1, t)`
