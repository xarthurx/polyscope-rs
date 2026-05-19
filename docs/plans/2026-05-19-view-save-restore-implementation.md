# View Save / Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the design from `docs/plans/2026-05-19-view-save-restore-design.md` — JSON-based save/restore of camera pose + render-look state, with both a programmatic API and UI buttons.

**Architecture:** The state to save is distributed across `RenderEngine::camera`, `App` fields (`background_color`, `ground_plane`), `AppearanceSettings` (transparency), and `Context::options` (SSAO). To present a clean free-function API despite this distribution, two new fields are added to `Context`: `view_state_snapshot` (App-published, frame-by-frame) and `pending_view_apply` (queued by callers, App-consumed). The App drains pending at frame start and publishes snapshot at frame end. Headless rendering drains the pending queue once during `render_to_image()` and skips auto-fit when a view was applied. UI emits *intent* (`RequestSaveView`/`RequestLoadView`) from the egui builder; the actual `rfd` file dialog opens in the app loop *after* the egui frame, avoiding multi-pass reruns.

**Tech Stack:** Rust 2024, `serde` + `serde_json` for serialization, `rfd` 0.15 for native file dialogs, existing `Camera::start_flight_to` for `FlyTo` animation, `tempfile` for tests.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `crates/polyscope-core/src/error.rs` (modify) | Add 2 enum variants. |
| `crates/polyscope-core/src/state.rs` (modify) | Add 2 buffer fields to `Context`. |
| `crates/polyscope-core/src/view_state.rs` (new) | `RenderState`, `ViewState`, `PartialViewState`, `ViewTransition`, validation, the 6 public free functions. |
| `crates/polyscope-core/src/lib.rs` (modify) | Declare + re-export `view_state` module. |
| `crates/polyscope-render/Cargo.toml` (modify) | Add `serde` dep. |
| `crates/polyscope-render/src/camera.rs` (modify) | `CameraState` DTO, `Camera::current_state`, `Camera::apply_state`. |
| `crates/polyscope/Cargo.toml` (modify) | Add `serde_json` + `rfd` deps. |
| `crates/polyscope/src/app/mod.rs` (modify) | `App::current_view_state`, `App::apply_view_state`. |
| `crates/polyscope/src/app/render.rs` (modify) | Drain `pending_view_apply` before render; publish snapshot after. |
| `crates/polyscope/src/app/render_ui.rs` (modify) | Dispatch new `ViewAction` variants (open rfd dialog, call save/load). |
| `crates/polyscope/src/headless.rs` (modify) | Drain pending state; skip auto-fit if applied. |
| `crates/polyscope-ui/src/panels.rs` (modify) | Two new buttons in `build_controls_section`. |
| `crates/polyscope-ui/src/lib.rs` (modify) | (Only if `ViewAction` is re-exported here.) |
| `crates/polyscope/tests/view_state.rs` (new) | Integration tests including headless reproducibility. |
| `CHANGELOG.md`, `docs/feature-status.md` (modify) | Feature documentation. |

---

### Task 1: Foundation — error variants + Context buffer fields

**Files:**
- Modify: `crates/polyscope-core/src/error.rs:55` (end of enum)
- Modify: `crates/polyscope-core/src/state.rs` (the `Context` struct + its `Default` impl)

- [ ] **Step 1: Write the failing tests**

Add to a new `#[cfg(test)] mod tests` block at the bottom of `crates/polyscope-core/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_view_state_displays_reason() {
        let e = PolyscopeError::InvalidViewState("bad fov".to_string());
        let msg = format!("{e}");
        assert!(msg.contains("bad fov"), "got: {msg}");
        assert!(msg.contains("view"), "got: {msg}");
    }

    #[test]
    fn test_no_active_view_displays_static_message() {
        let e = PolyscopeError::NoActiveView;
        let msg = format!("{e}");
        assert!(msg.contains("no view"), "got: {msg}");
    }
}
```

Add to a new `#[cfg(test)] mod state_view_tests` block at the bottom of `crates/polyscope-core/src/state.rs`:

```rust
#[cfg(test)]
mod state_view_tests {
    use super::*;

    #[test]
    fn test_context_starts_with_no_view_buffers() {
        let ctx = Context::default();
        assert!(ctx.view_state_snapshot.is_none());
        assert!(ctx.pending_view_apply.is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-core test_invalid_view_state_displays_reason test_no_active_view_displays_static_message test_context_starts_with_no_view_buffers -- --nocapture`
Expected: FAIL — variants and fields do not exist.

- [ ] **Step 3: Add error variants**

In `crates/polyscope-core/src/error.rs`, insert before the closing `}` of the enum (after line 54):

```rust

    /// View save/restore validation or version error.
    #[error("invalid view state: {0}")]
    InvalidViewState(String),

    /// Save called before any view frame has been rendered.
    #[error("no view available yet (render at least one frame before saving)")]
    NoActiveView,
```

- [ ] **Step 4: Add Context fields**

In `crates/polyscope-core/src/state.rs`, locate `pub struct Context {` (around line 35). Add forward-declared imports near the top of the file if missing:

```rust
// (Add to the existing imports near the top of the file)
// Note: ViewState lives in this crate's view_state module added in Task 5.
// Forward-declare via the module path.
```

Find the `Context` struct and add two new fields. The struct definition is `pub struct Context { ... }`. Add inside, after the last existing field (e.g. after the slice_planes / options fields — check the actual structure when editing):

```rust
    /// Latest view-state snapshot, published by the App once per frame.
    /// `None` until the first frame is rendered.
    pub(crate) view_state_snapshot: Option<crate::view_state::ViewState>,

    /// Pending view state queued by a caller, consumed by the App on the next frame.
    pub(crate) pending_view_apply: Option<(crate::view_state::ViewState, crate::view_state::ViewTransition)>,
```

In the `impl Default for Context` block (around line 80, where `Registry::new()` is initialized), add these two field initializers at the end of the struct literal:

```rust
            view_state_snapshot: None,
            pending_view_apply: None,
```

Note: `crate::view_state::ViewState` and `ViewTransition` don't exist yet — they're added in Task 5. The compiler will error here until then. To make this task pass-able in isolation, add a temporary stub module in `crates/polyscope-core/src/lib.rs`:

```rust
pub mod view_state {
    /// Placeholder, replaced in Task 5.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewState;

    /// Placeholder, replaced in Task 5.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ViewTransition {
        #[default]
        Instant,
        FlyTo,
    }
}
```

This stub is replaced wholesale by the real module in Task 5.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p polyscope-core test_invalid_view_state_displays_reason test_no_active_view_displays_static_message test_context_starts_with_no_view_buffers -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Run clippy + fmt**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

Run: `cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add crates/polyscope-core/src/error.rs crates/polyscope-core/src/state.rs crates/polyscope-core/src/lib.rs
git commit -m "feat(view_state): foundation - error variants + Context buffers"
```

---

### Task 2: CameraState DTO + Camera::current_state / apply_state (Instant)

**Files:**
- Modify: `crates/polyscope-render/Cargo.toml`
- Modify: `crates/polyscope-render/src/camera.rs`

- [ ] **Step 1: Add `serde` dep**

In `crates/polyscope-render/Cargo.toml`, add to the `[dependencies]` section:

```toml
serde = { workspace = true }
```

(`serde` is already a workspace dep — see root `Cargo.toml`.)

- [ ] **Step 2: Write the failing tests**

Add at the bottom of `crates/polyscope-render/src/camera.rs` (before any existing `mod tests` if present; otherwise add a new `#[cfg(test)] mod camera_state_tests`):

```rust
#[cfg(test)]
mod camera_state_tests {
    use super::*;

    fn make_camera() -> Camera {
        let mut c = Camera::new(1.5);
        c.position = Vec3::new(2.0, 3.0, 4.0);
        c.target = Vec3::new(0.5, 0.0, 0.0);
        c.up = Vec3::new(0.0, 1.0, 0.0);
        c.fov = 0.8;
        c.near = 0.1;
        c.far = 500.0;
        c.projection_mode = ProjectionMode::Orthographic;
        c.ortho_scale = 2.5;
        c.navigation_style = NavigationStyle::Arcball;
        c.up_direction = AxisDirection::NegY;
        c.front_direction = AxisDirection::PosZ;
        c
    }

    #[test]
    fn test_camera_state_serializes_snake_case_enums() {
        let s = CameraState::from_camera(&make_camera());
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["projection_mode"], "orthographic");
        assert_eq!(v["navigation_style"], "arcball");
        assert_eq!(v["up_direction"], "neg_y");
        assert_eq!(v["front_direction"], "pos_z");
    }

    #[test]
    fn test_camera_state_roundtrip() {
        let cam = make_camera();
        let state = CameraState::from_camera(&cam);
        let json = serde_json::to_string(&state).unwrap();
        let parsed: CameraState = serde_json::from_str(&json).unwrap();

        let mut fresh = Camera::new(1.5);
        parsed.apply(&mut fresh, ViewTransition::Instant);

        let eps = 1e-5;
        assert!((fresh.position - cam.position).length() < eps);
        assert!((fresh.target - cam.target).length() < eps);
        assert!((fresh.up - cam.up).length() < eps);
        assert!((fresh.fov - cam.fov).abs() < eps);
        assert!((fresh.near - cam.near).abs() < eps);
        assert!((fresh.far - cam.far).abs() < eps);
        assert_eq!(fresh.projection_mode, cam.projection_mode);
        assert!((fresh.ortho_scale - cam.ortho_scale).abs() < eps);
        assert_eq!(fresh.navigation_style, cam.navigation_style);
        assert_eq!(fresh.up_direction, cam.up_direction);
        assert_eq!(fresh.front_direction, cam.front_direction);
    }

    #[test]
    fn test_camera_state_instant_does_not_start_flight() {
        let cam = make_camera();
        let state = CameraState::from_camera(&cam);
        let mut fresh = Camera::new(1.5);
        state.apply(&mut fresh, ViewTransition::Instant);
        assert!(fresh.flight.is_none());
    }
}
```

`ViewTransition` is imported from `polyscope_core::view_state::ViewTransition` — add `use polyscope_core::view_state::ViewTransition;` near the test's `use super::*;`.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p polyscope-render camera_state -- --nocapture`
Expected: FAIL — `CameraState` not defined.

- [ ] **Step 4: Implement `CameraState`**

In `crates/polyscope-render/src/camera.rs`, near the top after the existing `use` declarations (or alongside the existing enums), add:

```rust
use serde::{Deserialize, Serialize};

/// Serializable snapshot of a `Camera` covering everything needed to
/// reproduce its rendered view.
///
/// Enum-string fields are DTO-local — the public enums on `Camera`
/// (`NavigationStyle`, `ProjectionMode`, `AxisDirection`) keep their
/// existing serialization (which is not relied on anywhere shipping).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraState {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub projection_mode: String,
    pub ortho_scale: f32,
    pub navigation_style: String,
    pub up_direction: String,
    pub front_direction: String,
}

impl CameraState {
    /// Snapshot the current camera into a `CameraState`.
    #[must_use]
    pub fn from_camera(c: &Camera) -> Self {
        Self {
            position: c.position.into(),
            target: c.target.into(),
            up: c.up.into(),
            fov: c.fov,
            near: c.near,
            far: c.far,
            projection_mode: projection_mode_name(c.projection_mode).to_owned(),
            ortho_scale: c.ortho_scale,
            navigation_style: navigation_style_name(c.navigation_style).to_owned(),
            up_direction: axis_direction_name(c.up_direction).to_owned(),
            front_direction: axis_direction_name(c.front_direction).to_owned(),
        }
    }

    /// Apply this state to `camera`. For `Instant`, all fields snap to the
    /// stored values. For `FlyTo`, position/target/up/fov animate via the
    /// existing camera flight (~0.4 s) and the rest snap immediately.
    pub fn apply(&self, camera: &mut Camera, transition: polyscope_core::view_state::ViewTransition) {
        use polyscope_core::view_state::ViewTransition;

        let new_pos = Vec3::from(self.position);
        let new_target = Vec3::from(self.target);
        let new_up = Vec3::from(self.up);

        // Non-pose fields always snap (no fade for ortho mode, FOV when animated, etc.)
        camera.near = self.near;
        camera.far = self.far;
        camera.projection_mode = parse_projection_mode(&self.projection_mode)
            .unwrap_or(camera.projection_mode);
        camera.ortho_scale = self.ortho_scale;
        camera.navigation_style = parse_navigation_style(&self.navigation_style)
            .unwrap_or(camera.navigation_style);
        camera.up_direction = parse_axis_direction(&self.up_direction)
            .unwrap_or(camera.up_direction);
        camera.front_direction = parse_axis_direction(&self.front_direction)
            .unwrap_or(camera.front_direction);

        match transition {
            ViewTransition::Instant => {
                camera.position = new_pos;
                camera.target = new_target;
                camera.up = new_up;
                camera.fov = self.fov;
                camera.flight = None;
            }
            ViewTransition::FlyTo => {
                // Build the target view matrix and reuse existing start_flight_to.
                let target_view = Mat4::look_at_rh(new_pos, new_target, new_up);
                camera.start_flight_to(target_view, self.fov, 0.4);
            }
        }
    }
}

// ─── enum-string helpers (DTO-local) ───

fn navigation_style_name(s: NavigationStyle) -> &'static str {
    match s {
        NavigationStyle::Turntable => "turntable",
        NavigationStyle::Free => "free",
        NavigationStyle::Planar => "planar",
        NavigationStyle::Arcball => "arcball",
        NavigationStyle::FirstPerson => "first_person",
    }
}

fn parse_navigation_style(s: &str) -> Option<NavigationStyle> {
    Some(match s {
        "turntable" => NavigationStyle::Turntable,
        "free" => NavigationStyle::Free,
        "planar" => NavigationStyle::Planar,
        "arcball" => NavigationStyle::Arcball,
        "first_person" => NavigationStyle::FirstPerson,
        _ => return None,
    })
}

fn projection_mode_name(p: ProjectionMode) -> &'static str {
    match p {
        ProjectionMode::Perspective => "perspective",
        ProjectionMode::Orthographic => "orthographic",
    }
}

fn parse_projection_mode(s: &str) -> Option<ProjectionMode> {
    Some(match s {
        "perspective" => ProjectionMode::Perspective,
        "orthographic" => ProjectionMode::Orthographic,
        _ => return None,
    })
}

fn axis_direction_name(a: AxisDirection) -> &'static str {
    match a {
        AxisDirection::PosX => "pos_x",
        AxisDirection::NegX => "neg_x",
        AxisDirection::PosY => "pos_y",
        AxisDirection::NegY => "neg_y",
        AxisDirection::PosZ => "pos_z",
        AxisDirection::NegZ => "neg_z",
    }
}

fn parse_axis_direction(s: &str) -> Option<AxisDirection> {
    Some(match s {
        "pos_x" => AxisDirection::PosX,
        "neg_x" => AxisDirection::NegX,
        "pos_y" => AxisDirection::PosY,
        "neg_y" => AxisDirection::NegY,
        "pos_z" => AxisDirection::PosZ,
        "neg_z" => AxisDirection::NegZ,
        _ => return None,
    })
}
```

The `parse_*` helpers are also used by `CameraState::apply`, where unknown values fall back to the current field. The free-function validator (Task 5/6) is the one that raises `InvalidViewState` for unknown strings.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p polyscope-render camera_state -- --nocapture`
Expected: All 3 PASS.

- [ ] **Step 6: Clippy + fmt**

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add crates/polyscope-render/Cargo.toml crates/polyscope-render/src/camera.rs
git commit -m "feat(view_state): CameraState DTO and Camera::apply (Instant)"
```

---

### Task 3: CameraState FlyTo transition

This task validates that `ViewTransition::FlyTo` actually starts a flight; Task 2's roundtrip uses `Instant`.

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs` (tests only)

- [ ] **Step 1: Write the failing test**

Add to the `camera_state_tests` module:

```rust
    #[test]
    fn test_camera_state_flyto_starts_flight() {
        let cam = make_camera();
        let state = CameraState::from_camera(&cam);
        let mut fresh = Camera::new(1.5);
        // Different starting pose so the flight is meaningful
        fresh.position = Vec3::new(-1.0, -1.0, -1.0);
        fresh.target = Vec3::ZERO;
        fresh.up = Vec3::Y;

        state.apply(&mut fresh, ViewTransition::FlyTo);
        assert!(
            fresh.flight.is_some(),
            "FlyTo should start a camera flight"
        );
        // Non-pose fields apply immediately
        assert_eq!(fresh.projection_mode, cam.projection_mode);
        assert!((fresh.ortho_scale - cam.ortho_scale).abs() < 1e-5);
    }

    #[test]
    fn test_camera_state_unknown_enum_falls_back_in_apply() {
        let mut bad = CameraState::from_camera(&make_camera());
        bad.projection_mode = "definitely_not_a_mode".to_string();
        bad.navigation_style = "also_not_real".to_string();

        let mut target = Camera::new(1.5);
        let original_proj = target.projection_mode;
        let original_nav = target.navigation_style;
        bad.apply(&mut target, ViewTransition::Instant);

        assert_eq!(target.projection_mode, original_proj);
        assert_eq!(target.navigation_style, original_nav);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p polyscope-render camera_state -- --nocapture`
Expected: Both PASS (Task 2's `apply` already handles FlyTo + unknown fallback).

If they fail, fix the implementation. The Task 2 code is already written to handle both cases; this task is the assertion that it works.

- [ ] **Step 3: Commit**

```bash
git add crates/polyscope-render/src/camera.rs
git commit -m "test(view_state): CameraState FlyTo flight + unknown-enum fallback"
```

---

### Task 4: RenderState DTO

**Files:**
- Modify: `crates/polyscope-core/src/lib.rs` (replace the stub `view_state` module with a real `mod view_state;`)
- Create: `crates/polyscope-core/src/view_state.rs`

- [ ] **Step 1: Replace the stub module**

In `crates/polyscope-core/src/lib.rs`, find the temporary stub `pub mod view_state { ... }` block from Task 1 and replace it with:

```rust
pub mod view_state;
```

- [ ] **Step 2: Write failing tests**

Create `crates/polyscope-core/src/view_state.rs` with the following skeleton (just the tests + minimal scaffolding so tests compile but fail):

```rust
//! View save / restore — DTOs, validation, and public API.
//!
//! See `docs/plans/2026-05-19-view-save-restore-design.md` for the design.

use crate::ground_plane::GroundPlaneMode;
use crate::options::TransparencyMode;
use crate::ssao::SsaoConfig;
use serde::{Deserialize, Serialize};

/// Whether to animate the camera transition when loading a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewTransition {
    #[default]
    Instant,
    FlyTo,
}

/// Render-look state (background, ground plane, transparency, SSAO, SSAA).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderState {
    pub background_color: [f32; 3],
    pub ground_plane: GroundPlaneState,
    pub transparency: TransparencyState,
    pub ssao: SsaoConfig,
    pub ssaa_factor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundPlaneState {
    pub enabled: bool,
    pub mode: String,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransparencyState {
    pub enabled: bool,
    pub mode: String,
    pub render_passes: u32,
}

/// Convert a `GroundPlaneMode` to the JSON string used in `RenderState`.
#[must_use]
pub fn ground_plane_mode_name(m: GroundPlaneMode) -> &'static str {
    match m {
        GroundPlaneMode::None => "none",
        GroundPlaneMode::Tile => "tile",
        GroundPlaneMode::ShadowOnly => "shadow_only",
        GroundPlaneMode::TileReflection => "tile_reflection",
    }
}

/// Parse a `GroundPlaneMode` from its JSON string form.
pub fn parse_ground_plane_mode(s: &str) -> Option<GroundPlaneMode> {
    Some(match s {
        "none" => GroundPlaneMode::None,
        "tile" => GroundPlaneMode::Tile,
        "shadow_only" => GroundPlaneMode::ShadowOnly,
        "tile_reflection" => GroundPlaneMode::TileReflection,
        _ => return None,
    })
}

#[must_use]
pub fn transparency_mode_name(m: TransparencyMode) -> &'static str {
    match m {
        TransparencyMode::Simple => "simple",
        TransparencyMode::Pretty => "pretty",
    }
}

pub fn parse_transparency_mode(s: &str) -> Option<TransparencyMode> {
    Some(match s {
        "simple" => TransparencyMode::Simple,
        "pretty" => TransparencyMode::Pretty,
        _ => return None,
    })
}

// Placeholder — replaced in Task 5.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_plane_mode_names() {
        assert_eq!(ground_plane_mode_name(GroundPlaneMode::TileReflection), "tile_reflection");
        assert_eq!(parse_ground_plane_mode("tile_reflection"), Some(GroundPlaneMode::TileReflection));
        assert_eq!(parse_ground_plane_mode("xyz"), None);
    }

    #[test]
    fn test_transparency_mode_names() {
        assert_eq!(transparency_mode_name(TransparencyMode::Pretty), "pretty");
        assert_eq!(parse_transparency_mode("simple"), Some(TransparencyMode::Simple));
    }

    #[test]
    fn test_render_state_roundtrip() {
        let r = RenderState {
            background_color: [0.1, 0.2, 0.3],
            ground_plane: GroundPlaneState {
                enabled: true,
                mode: "tile_reflection".to_string(),
                height: 1.5,
            },
            transparency: TransparencyState {
                enabled: true,
                mode: "simple".to_string(),
                render_passes: 6,
            },
            ssao: SsaoConfig::default(),
            ssaa_factor: 2,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: RenderState = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
```

Note: This task introduces real `RenderState`/etc. while leaving `ViewState` as a placeholder (replaced in Task 5). The `view_state` module also re-exports `ViewTransition`, which Task 2's tests already imported — that import will resolve now.

- [ ] **Step 3: Run tests**

Run: `cargo test -p polyscope-core view_state -- --nocapture`
Expected: All 3 PASS.

If `polyscope_core::ground_plane::GroundPlaneMode` doesn't have exactly the variants `None`, `Tile`, `ShadowOnly`, `TileReflection`, adjust the names match the actual enum (verified at design time: `ground_plane.rs:7` has `TileReflection`).

If `TransparencyMode` variants differ, adjust similarly. (Spec assumes `Simple` and `Pretty`.)

- [ ] **Step 4: Clippy + fmt + commit**

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope-core/src/view_state.rs crates/polyscope-core/src/lib.rs
git commit -m "feat(view_state): RenderState DTO + enum string helpers"
```

---

### Task 5: ViewState + PartialViewState + validation + version

**Files:**
- Modify: `crates/polyscope-core/src/view_state.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/polyscope-core/src/view_state.rs` (inside the existing `#[cfg(test)] mod tests` block):

```rust
    #[test]
    fn test_view_state_unknown_version_rejected() {
        let json = r#"{"version": 99, "camera": null, "render": null}"#;
        let err = ViewState::from_json_partial_then_validate(json, &dummy_view_state()).unwrap_err();
        match err {
            crate::error::PolyscopeError::InvalidViewState(reason) => {
                assert!(reason.contains("version"), "{reason}");
            }
            other => panic!("expected InvalidViewState, got {other:?}"),
        }
    }

    #[test]
    fn test_view_state_missing_version_rejected() {
        let json = r#"{}"#;
        let err = ViewState::from_json_partial_then_validate(json, &dummy_view_state()).unwrap_err();
        match err {
            crate::error::PolyscopeError::InvalidViewState(_)
            | crate::error::PolyscopeError::JsonError(_) => {}
            other => panic!("expected InvalidViewState or JsonError, got {other:?}"),
        }
    }

    #[test]
    fn test_view_state_missing_fields_preserve_current() {
        // Only version present — every other field should fall back to current.
        let json = r#"{"version": 1}"#;
        let current = make_view_state_with_fov(0.5);
        let merged = ViewState::from_json_partial_then_validate(json, &current).unwrap();
        assert_eq!(merged.camera.fov, 0.5);
    }

    #[test]
    fn test_view_state_partial_camera_overrides_only_specified() {
        let current = make_view_state_with_fov(0.5);
        let json = r#"{"version": 1, "camera": {"fov": 1.2}}"#;
        let merged = ViewState::from_json_partial_then_validate(json, &current).unwrap();
        assert!((merged.camera.fov - 1.2).abs() < 1e-6);
        // Other camera fields preserved
        assert_eq!(merged.camera.projection_mode, current.camera.projection_mode);
    }

    #[test]
    fn test_validation_rejects_nonfinite_fov() {
        let mut s = dummy_view_state();
        s.camera.fov = f32::NAN;
        let err = ViewState::validate(&s).unwrap_err();
        assert!(matches!(err, crate::error::PolyscopeError::InvalidViewState(_)));
    }

    #[test]
    fn test_validation_rejects_fov_out_of_range() {
        let mut s = dummy_view_state();
        s.camera.fov = 0.0;
        assert!(ViewState::validate(&s).is_err());
        s.camera.fov = std::f32::consts::PI + 0.1;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_near_geq_far() {
        let mut s = dummy_view_state();
        s.camera.near = 10.0;
        s.camera.far = 1.0;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_zero_ssaa_factor() {
        let mut s = dummy_view_state();
        s.render.ssaa_factor = 0;
        assert!(ViewState::validate(&s).is_err());
    }

    #[test]
    fn test_validation_rejects_unknown_enum_strings() {
        let mut s = dummy_view_state();
        s.camera.projection_mode = "definitely_not_a_mode".to_string();
        assert!(ViewState::validate(&s).is_err());
    }

    fn dummy_view_state() -> ViewState {
        ViewState {
            version: ViewState::CURRENT_VERSION,
            camera: CameraStateOwned {
                position: [0.0, 0.0, 3.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                fov: 0.7854,
                near: 0.01,
                far: 1000.0,
                projection_mode: "perspective".to_string(),
                ortho_scale: 1.0,
                navigation_style: "turntable".to_string(),
                up_direction: "pos_y".to_string(),
                front_direction: "neg_z".to_string(),
            },
            render: RenderState {
                background_color: [1.0, 1.0, 1.0],
                ground_plane: GroundPlaneState {
                    enabled: true,
                    mode: "tile_reflection".to_string(),
                    height: 0.0,
                },
                transparency: TransparencyState {
                    enabled: true,
                    mode: "simple".to_string(),
                    render_passes: 8,
                },
                ssao: SsaoConfig::default(),
                ssaa_factor: 1,
            },
        }
    }

    fn make_view_state_with_fov(fov: f32) -> ViewState {
        let mut s = dummy_view_state();
        s.camera.fov = fov;
        s
    }
```

- [ ] **Step 2: Implement `ViewState` + `PartialViewState` + validation**

Replace the placeholder `pub struct ViewState;` in `crates/polyscope-core/src/view_state.rs` with the real implementation. Append the following (and remove the placeholder):

```rust
use crate::error::{PolyscopeError, Result};

/// Plain DTO mirroring `polyscope_render::CameraState` but living in core
/// (avoids a render → core dependency for the wrapper).
///
/// Field-by-field equivalent of the render-side `CameraState`. Wired up via
/// the `From` impls in the `polyscope` main crate during Task 7.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraStateOwned {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub projection_mode: String,
    pub ortho_scale: f32,
    pub navigation_style: String,
    pub up_direction: String,
    pub front_direction: String,
}

/// Top-level view state. Composes camera + render-look.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewState {
    pub version: u32,
    pub camera: CameraStateOwned,
    pub render: RenderState,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialViewState {
    version: u32,
    #[serde(default)]
    camera: Option<PartialCameraState>,
    #[serde(default)]
    render: Option<PartialRenderState>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialCameraState {
    position: Option<[f32; 3]>,
    target: Option<[f32; 3]>,
    up: Option<[f32; 3]>,
    fov: Option<f32>,
    near: Option<f32>,
    far: Option<f32>,
    projection_mode: Option<String>,
    ortho_scale: Option<f32>,
    navigation_style: Option<String>,
    up_direction: Option<String>,
    front_direction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialRenderState {
    background_color: Option<[f32; 3]>,
    ground_plane: Option<PartialGroundPlaneState>,
    transparency: Option<PartialTransparencyState>,
    ssao: Option<SsaoConfig>,
    ssaa_factor: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialGroundPlaneState {
    enabled: Option<bool>,
    mode: Option<String>,
    height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialTransparencyState {
    enabled: Option<bool>,
    mode: Option<String>,
    render_passes: Option<u32>,
}

impl ViewState {
    /// JSON format version this build writes and accepts.
    pub const CURRENT_VERSION: u32 = 1;

    /// Parse a JSON string permissively (missing fields preserve `current`),
    /// then validate the resulting full state.
    pub fn from_json_partial_then_validate(json: &str, current: &ViewState) -> Result<ViewState> {
        let partial: PartialViewState = serde_json::from_str(json)?;
        if partial.version != Self::CURRENT_VERSION {
            return Err(PolyscopeError::InvalidViewState(format!(
                "unsupported view version {} (this build accepts {})",
                partial.version,
                Self::CURRENT_VERSION
            )));
        }
        let merged = Self::merge(partial, current);
        Self::validate(&merged)?;
        Ok(merged)
    }

    fn merge(partial: PartialViewState, current: &ViewState) -> ViewState {
        let cam_p = partial.camera.unwrap_or(PartialCameraState {
            position: None,
            target: None,
            up: None,
            fov: None,
            near: None,
            far: None,
            projection_mode: None,
            ortho_scale: None,
            navigation_style: None,
            up_direction: None,
            front_direction: None,
        });
        let camera = CameraStateOwned {
            position: cam_p.position.unwrap_or(current.camera.position),
            target: cam_p.target.unwrap_or(current.camera.target),
            up: cam_p.up.unwrap_or(current.camera.up),
            fov: cam_p.fov.unwrap_or(current.camera.fov),
            near: cam_p.near.unwrap_or(current.camera.near),
            far: cam_p.far.unwrap_or(current.camera.far),
            projection_mode: cam_p
                .projection_mode
                .unwrap_or_else(|| current.camera.projection_mode.clone()),
            ortho_scale: cam_p.ortho_scale.unwrap_or(current.camera.ortho_scale),
            navigation_style: cam_p
                .navigation_style
                .unwrap_or_else(|| current.camera.navigation_style.clone()),
            up_direction: cam_p
                .up_direction
                .unwrap_or_else(|| current.camera.up_direction.clone()),
            front_direction: cam_p
                .front_direction
                .unwrap_or_else(|| current.camera.front_direction.clone()),
        };

        let render_p = partial.render.unwrap_or(PartialRenderState {
            background_color: None,
            ground_plane: None,
            transparency: None,
            ssao: None,
            ssaa_factor: None,
        });
        let gp_p = render_p.ground_plane.unwrap_or(PartialGroundPlaneState {
            enabled: None,
            mode: None,
            height: None,
        });
        let tp_p = render_p.transparency.unwrap_or(PartialTransparencyState {
            enabled: None,
            mode: None,
            render_passes: None,
        });

        let render = RenderState {
            background_color: render_p
                .background_color
                .unwrap_or(current.render.background_color),
            ground_plane: GroundPlaneState {
                enabled: gp_p.enabled.unwrap_or(current.render.ground_plane.enabled),
                mode: gp_p.mode.unwrap_or_else(|| current.render.ground_plane.mode.clone()),
                height: gp_p.height.unwrap_or(current.render.ground_plane.height),
            },
            transparency: TransparencyState {
                enabled: tp_p.enabled.unwrap_or(current.render.transparency.enabled),
                mode: tp_p.mode.unwrap_or_else(|| current.render.transparency.mode.clone()),
                render_passes: tp_p
                    .render_passes
                    .unwrap_or(current.render.transparency.render_passes),
            },
            ssao: render_p.ssao.unwrap_or(current.render.ssao.clone()),
            ssaa_factor: render_p.ssaa_factor.unwrap_or(current.render.ssaa_factor),
        };

        ViewState {
            version: Self::CURRENT_VERSION,
            camera,
            render,
        }
    }

    /// Validate that all field values are sensible.
    pub fn validate(s: &ViewState) -> Result<()> {
        let invalid = |reason: &str| Err(PolyscopeError::InvalidViewState(reason.to_string()));

        let finite_arr = |a: &[f32; 3]| a.iter().all(|x| x.is_finite());
        if !finite_arr(&s.camera.position) {
            return invalid("camera.position has non-finite values");
        }
        if !finite_arr(&s.camera.target) {
            return invalid("camera.target has non-finite values");
        }
        if !finite_arr(&s.camera.up) {
            return invalid("camera.up has non-finite values");
        }
        if !s.camera.fov.is_finite() || s.camera.fov <= 0.0 || s.camera.fov >= std::f32::consts::PI {
            return invalid("camera.fov must be in (0, π)");
        }
        if !s.camera.near.is_finite() || s.camera.near <= 0.0 {
            return invalid("camera.near must be > 0");
        }
        if !s.camera.far.is_finite() || s.camera.far <= s.camera.near {
            return invalid("camera.far must be > camera.near");
        }
        if !s.camera.ortho_scale.is_finite() || s.camera.ortho_scale <= 0.0 {
            return invalid("camera.ortho_scale must be > 0");
        }
        // String enums — accept any value that parses on the consumer side.
        // We do a minimum check that the value is in our known set here so
        // bad files fail early.
        const KNOWN_PROJ: &[&str] = &["perspective", "orthographic"];
        const KNOWN_NAV: &[&str] =
            &["turntable", "free", "planar", "arcball", "first_person"];
        const KNOWN_AXIS: &[&str] = &["pos_x", "neg_x", "pos_y", "neg_y", "pos_z", "neg_z"];
        if !KNOWN_PROJ.contains(&s.camera.projection_mode.as_str()) {
            return invalid(&format!("unknown projection_mode: {}", s.camera.projection_mode));
        }
        if !KNOWN_NAV.contains(&s.camera.navigation_style.as_str()) {
            return invalid(&format!("unknown navigation_style: {}", s.camera.navigation_style));
        }
        if !KNOWN_AXIS.contains(&s.camera.up_direction.as_str()) {
            return invalid(&format!("unknown up_direction: {}", s.camera.up_direction));
        }
        if !KNOWN_AXIS.contains(&s.camera.front_direction.as_str()) {
            return invalid(&format!("unknown front_direction: {}", s.camera.front_direction));
        }

        // Render
        if !s.render.background_color.iter().all(|x| x.is_finite()) {
            return invalid("render.background_color has non-finite values");
        }
        const KNOWN_GP: &[&str] = &["none", "tile", "shadow_only", "tile_reflection"];
        if !KNOWN_GP.contains(&s.render.ground_plane.mode.as_str()) {
            return invalid(&format!("unknown ground_plane.mode: {}", s.render.ground_plane.mode));
        }
        const KNOWN_TR: &[&str] = &["simple", "pretty"];
        if !KNOWN_TR.contains(&s.render.transparency.mode.as_str()) {
            return invalid(&format!("unknown transparency.mode: {}", s.render.transparency.mode));
        }
        if s.render.transparency.render_passes == 0 {
            return invalid("render.transparency.render_passes must be ≥ 1");
        }
        if !s.render.ground_plane.height.is_finite() {
            return invalid("render.ground_plane.height must be finite");
        }
        const KNOWN_SSAA: &[u32] = &[1, 2, 4, 8];
        if !KNOWN_SSAA.contains(&s.render.ssaa_factor) {
            return invalid(&format!("render.ssaa_factor must be one of {KNOWN_SSAA:?}"));
        }
        if s.render.ssao.sample_count == 0 || s.render.ssao.sample_count > 256 {
            return invalid("render.ssao.sample_count must be in [1, 256]");
        }
        if !(s.render.ssao.radius.is_finite() && s.render.ssao.radius > 0.0) {
            return invalid("render.ssao.radius must be > 0");
        }
        Ok(())
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p polyscope-core view_state -- --nocapture`
Expected: All tests PASS.

- [ ] **Step 4: Clippy + fmt + commit**

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope-core/src/view_state.rs
git commit -m "feat(view_state): ViewState + PartialViewState + validation"
```

---

### Task 6: Public free functions (Context-buffer-backed)

**Files:**
- Modify: `crates/polyscope-core/src/view_state.rs`
- Modify: `crates/polyscope-core/src/lib.rs` (re-exports)

- [ ] **Step 1: Write failing tests**

Append to `view_state.rs` test module:

```rust
    use crate::state::{Context, with_context, with_context_mut};

    fn install_dummy_snapshot() {
        with_context_mut(|ctx| {
            ctx.view_state_snapshot = Some(dummy_view_state());
        });
    }

    fn clear_snapshot() {
        with_context_mut(|ctx| {
            ctx.view_state_snapshot = None;
            ctx.pending_view_apply = None;
        });
    }

    #[test]
    fn test_current_view_state_returns_error_when_no_snapshot() {
        // Caveat: tests share global context; we rely on test isolation guard.
        // Use a unique marker on the snapshot so we know nothing's there.
        clear_snapshot();
        let err = current_view_state().unwrap_err();
        assert!(matches!(err, crate::error::PolyscopeError::NoActiveView));
    }

    #[test]
    fn test_save_view_to_json_roundtrip_via_buffers() {
        install_dummy_snapshot();
        let json = save_view_to_json().unwrap();
        assert!(json.contains("\"version\""));
        let _: PartialViewState = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_load_view_from_json_queues_pending() {
        install_dummy_snapshot();
        let json = save_view_to_json().unwrap();
        load_view_from_json(&json, ViewTransition::Instant).unwrap();
        with_context(|ctx| {
            assert!(ctx.pending_view_apply.is_some());
            let (state, transition) = ctx.pending_view_apply.as_ref().unwrap();
            assert_eq!(*transition, ViewTransition::Instant);
            assert_eq!(state.camera.fov, 0.7854);
        });
        clear_snapshot();
    }
```

These tests touch the **global context** and so are sensitive to test ordering. Add `#[serial_test::serial]` if `serial_test` is available; otherwise rely on the existing convention used elsewhere in the codebase (check how `lib.rs` of polyscope handles this — typically a `setup()` helper is called). For polyscope-core, the simpler approach: tests share Context, but each test that touches buffers begins by clearing them, ensuring deterministic state.

- [ ] **Step 2: Implement the public functions**

Append to `view_state.rs`:

```rust
use crate::state::{with_context, with_context_mut};

/// Returns the latest view-state snapshot from the running App,
/// or an error if no frame has been rendered yet.
pub fn current_view_state() -> Result<ViewState> {
    with_context(|ctx| {
        ctx.view_state_snapshot
            .clone()
            .ok_or(PolyscopeError::NoActiveView)
    })
}

/// Queues a view-state application for the next frame.
pub fn apply_view_state(state: &ViewState, transition: ViewTransition) -> Result<()> {
    ViewState::validate(state)?;
    with_context_mut(|ctx| {
        ctx.pending_view_apply = Some((state.clone(), transition));
    });
    Ok(())
}

/// Serialize the current view state to a JSON string.
pub fn save_view_to_json() -> Result<String> {
    let state = current_view_state()?;
    Ok(serde_json::to_string_pretty(&state)?)
}

/// Save the current view state to a file.
pub fn save_view_to_file(path: impl AsRef<std::path::Path>) -> Result<()> {
    let json = save_view_to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Parse a JSON view-state and queue it for application.
///
/// Missing fields preserve the current snapshot's values. If no snapshot
/// is available yet, missing fields fall back to a built-in default.
pub fn load_view_from_json(json: &str, transition: ViewTransition) -> Result<()> {
    let current = current_view_state().unwrap_or_else(|_| ViewState {
        version: ViewState::CURRENT_VERSION,
        camera: CameraStateOwned {
            position: [0.0, 0.0, 3.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
            near: 0.01,
            far: 1000.0,
            projection_mode: "perspective".to_string(),
            ortho_scale: 1.0,
            navigation_style: "turntable".to_string(),
            up_direction: "pos_y".to_string(),
            front_direction: "neg_z".to_string(),
        },
        render: RenderState {
            background_color: [1.0, 1.0, 1.0],
            ground_plane: GroundPlaneState {
                enabled: true,
                mode: "tile_reflection".to_string(),
                height: 0.0,
            },
            transparency: TransparencyState {
                enabled: true,
                mode: "simple".to_string(),
                render_passes: 8,
            },
            ssao: SsaoConfig::default(),
            ssaa_factor: 1,
        },
    });

    let merged = ViewState::from_json_partial_then_validate(json, &current)?;
    apply_view_state(&merged, transition)
}

/// Load and queue a view state from a file path.
pub fn load_view_from_file(
    path: impl AsRef<std::path::Path>,
    transition: ViewTransition,
) -> Result<()> {
    let json = std::fs::read_to_string(path)?;
    load_view_from_json(&json, transition)
}
```

- [ ] **Step 3: Re-export from `polyscope-core/src/lib.rs`**

Add to `crates/polyscope-core/src/lib.rs`:

```rust
pub use view_state::{
    apply_view_state, current_view_state, load_view_from_file, load_view_from_json,
    save_view_to_file, save_view_to_json, CameraStateOwned, GroundPlaneState, RenderState,
    TransparencyState, ViewState, ViewTransition,
};
```

(Adjust to fit alphabetical or grouped ordering used elsewhere in the file.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p polyscope-core view_state -- --nocapture`
Expected: All PASS. The buffer-sharing tests are sensitive to ordering; if a flake appears, add `#[serial_test::serial]` and the `serial_test` workspace dep.

- [ ] **Step 5: Clippy + fmt + commit**

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope-core/src/view_state.rs crates/polyscope-core/src/lib.rs
git commit -m "feat(view_state): public save/load API operating on Context buffers"
```

---

### Task 7: `App::current_view_state` + `App::apply_view_state`

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs` (add `From` conversions between `CameraState` ↔ `CameraStateOwned` — these traits bridge render-side DTO and core-side DTO)
- Modify: `crates/polyscope/src/app/mod.rs`

- [ ] **Step 1: Add `From` conversions in `polyscope-render/src/camera.rs`**

Since `CameraStateOwned` lives in `polyscope-core` and `CameraState` in `polyscope-render` (which already depends on core), add `From` impls so the App can convert between them:

```rust
impl From<&CameraState> for polyscope_core::view_state::CameraStateOwned {
    fn from(s: &CameraState) -> Self {
        polyscope_core::view_state::CameraStateOwned {
            position: s.position,
            target: s.target,
            up: s.up,
            fov: s.fov,
            near: s.near,
            far: s.far,
            projection_mode: s.projection_mode.clone(),
            ortho_scale: s.ortho_scale,
            navigation_style: s.navigation_style.clone(),
            up_direction: s.up_direction.clone(),
            front_direction: s.front_direction.clone(),
        }
    }
}

impl From<&polyscope_core::view_state::CameraStateOwned> for CameraState {
    fn from(s: &polyscope_core::view_state::CameraStateOwned) -> Self {
        CameraState {
            position: s.position,
            target: s.target,
            up: s.up,
            fov: s.fov,
            near: s.near,
            far: s.far,
            projection_mode: s.projection_mode.clone(),
            ortho_scale: s.ortho_scale,
            navigation_style: s.navigation_style.clone(),
            up_direction: s.up_direction.clone(),
            front_direction: s.front_direction.clone(),
        }
    }
}
```

(The two DTOs are field-identical; trait impls keep the bridge explicit. Alternative considered: make `CameraStateOwned` the only DTO and skip the render-side one. Rejected because `Camera::apply_state` benefits from being adjacent to `Camera` itself, and the render crate already had to grow `serde` for it.)

- [ ] **Step 2: Write failing tests in `app/mod.rs`**

Append to `crates/polyscope/src/app/mod.rs` (in a new `#[cfg(test)] mod app_view_tests` block):

```rust
#[cfg(test)]
mod app_view_tests {
    use super::*;
    use polyscope_core::view_state::ViewTransition;

    fn fresh_app() -> App {
        // Mirror the existing test setup in this crate.
        let _ = polyscope_core::state::ensure_initialized();
        App::new()
    }

    #[test]
    fn test_app_current_view_state_is_consistent() {
        let app = fresh_app();
        // App may not have an engine until rendering; check that
        // current_view_state returns a state with our default background.
        // If engine is None, current_view_state should still produce a
        // best-effort snapshot (camera + the fields owned by App).
        let state = app.current_view_state();
        // Default background is white per App::new
        assert!((state.render.background_color[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_app_apply_view_state_instant_changes_background() {
        let mut app = fresh_app();
        let mut state = app.current_view_state();
        state.render.background_color = [0.5, 0.25, 0.125];
        app.apply_view_state(&state, ViewTransition::Instant);
        assert!((app.background_color.x - 0.5).abs() < 1e-6);
        assert!((app.background_color.y - 0.25).abs() < 1e-6);
        assert!((app.background_color.z - 0.125).abs() < 1e-6);
    }
}
```

If the helper `ensure_initialized` doesn't exist, use whatever setup pattern already exists in `crates/polyscope/src/lib.rs` (look for a `setup()` helper in its `mod tests` block — the prism/pyramid PR used one).

- [ ] **Step 3: Implement `App::current_view_state`**

In `crates/polyscope/src/app/mod.rs`, add an `impl App` block (or extend the existing one):

```rust
impl App {
    /// Snapshot the App's full view state. If a render engine is attached,
    /// camera state is taken from it; otherwise camera fields default to
    /// the values from `Camera::new(1.0)`. Render-look fields are pulled
    /// from this App and the global `Options`.
    pub fn current_view_state(&self) -> polyscope_core::view_state::ViewState {
        use polyscope_core::view_state::{
            CameraStateOwned, GroundPlaneState, RenderState, TransparencyState, ViewState,
        };

        let camera_state: CameraStateOwned = if let Some(engine) = self.engine.as_ref() {
            let cs = polyscope_render::CameraState::from_camera(engine.camera());
            (&cs).into()
        } else {
            let cam = polyscope_render::Camera::new(1.0);
            let cs = polyscope_render::CameraState::from_camera(&cam);
            (&cs).into()
        };

        // Options-owned bits
        let (ssao, transparency_mode, transparency_passes, transparency_enabled, ssaa_factor) =
            polyscope_core::state::with_context(|ctx| {
                (
                    ctx.options.ssao.clone(),
                    ctx.options.transparency_mode,
                    ctx.options.transparency_render_passes,
                    ctx.options.transparency_enabled,
                    ctx.options.ssaa_factor,
                )
            });

        // App-owned bits
        let render = RenderState {
            background_color: [
                self.background_color.x,
                self.background_color.y,
                self.background_color.z,
            ],
            ground_plane: GroundPlaneState {
                enabled: self.ground_plane.enabled,
                mode: polyscope_core::view_state::ground_plane_mode_name(self.ground_plane.mode)
                    .to_owned(),
                height: self.ground_plane.height,
            },
            transparency: TransparencyState {
                enabled: transparency_enabled,
                mode: polyscope_core::view_state::transparency_mode_name(transparency_mode)
                    .to_owned(),
                render_passes: transparency_passes,
            },
            ssao,
            ssaa_factor,
        };

        ViewState {
            version: ViewState::CURRENT_VERSION,
            camera: camera_state,
            render,
        }
    }

    /// Apply a view state to this App. For `Instant`, all fields snap. For
    /// `FlyTo`, the camera animates; non-camera fields apply immediately.
    pub fn apply_view_state(
        &mut self,
        state: &polyscope_core::view_state::ViewState,
        transition: polyscope_core::view_state::ViewTransition,
    ) {
        // Camera (only if an engine exists)
        if let Some(engine) = self.engine.as_mut() {
            let cs: polyscope_render::CameraState = (&state.camera).into();
            cs.apply(engine.camera_mut(), transition);
        }

        // Background
        self.background_color = glam::Vec3::new(
            state.render.background_color[0],
            state.render.background_color[1],
            state.render.background_color[2],
        );

        // Ground plane
        if let Some(mode) =
            polyscope_core::view_state::parse_ground_plane_mode(&state.render.ground_plane.mode)
        {
            self.ground_plane.enabled = state.render.ground_plane.enabled;
            self.ground_plane.mode = mode;
            self.ground_plane.height = state.render.ground_plane.height;
        }

        // Options
        polyscope_core::state::with_context_mut(|ctx| {
            ctx.options.ssao = state.render.ssao.clone();
            ctx.options.ssaa_factor = state.render.ssaa_factor;
            ctx.options.transparency_enabled = state.render.transparency.enabled;
            ctx.options.transparency_render_passes = state.render.transparency.render_passes;
            if let Some(m) =
                polyscope_core::view_state::parse_transparency_mode(&state.render.transparency.mode)
            {
                ctx.options.transparency_mode = m;
            }
        });

        // After applying, mark camera-fitted so the next frame doesn't re-fit.
        self.camera_fitted = true;
    }
}
```

Notes:
- `engine.camera()` / `engine.camera_mut()` may not exist with those exact names. Check `crates/polyscope-render/src/engine/mod.rs:110` for the field. If it's a field rather than a method, use `&engine.camera` / `&mut engine.camera`.
- `App::camera_fitted` was found at `app/render.rs:29` — verify its visibility (probably `pub` or `pub(crate)`). If private, add a setter.

- [ ] **Step 4: Run tests + clippy + fmt + commit**

```bash
cargo test -p polyscope-rs app_view_tests -- --nocapture
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope-render/src/camera.rs crates/polyscope/src/app/mod.rs
git commit -m "feat(view_state): App::current_view_state and apply_view_state"
```

---

### Task 8: Frame-loop wiring — drain pending + publish snapshot

**Files:**
- Modify: `crates/polyscope/src/app/render.rs`

- [ ] **Step 1: Find the frame entry point**

In `crates/polyscope/src/app/render.rs`, identify the function that runs once per frame (it calls `auto_fit_camera` at line 29 per `render_init::auto_fit_camera`). Likely named `render_frame` or `render`. Read the existing function. Drain logic goes at the start (before camera-related work) and publish logic goes at the end (after camera updates).

- [ ] **Step 2: Add drain + publish**

At the top of the per-frame function (before any camera updates):

```rust
// Drain any queued view-state application.
let pending = polyscope_core::state::with_context_mut(|ctx| ctx.pending_view_apply.take());
if let Some((state, transition)) = pending {
    self.apply_view_state(&state, transition);
}
```

At the end of the per-frame function (after camera + options have settled for this frame):

```rust
// Publish a fresh snapshot for save_view_to_json callers.
let snapshot = self.current_view_state();
polyscope_core::state::with_context_mut(|ctx| {
    ctx.view_state_snapshot = Some(snapshot);
});
```

- [ ] **Step 3: Confirm it builds and existing tests still pass**

Run: `cargo test --workspace -- --nocapture`
Expected: All PASS.

Run: `cargo clippy --workspace -- -D warnings`
Expected: Clean.

- [ ] **Step 4: Commit**

```bash
git add crates/polyscope/src/app/render.rs
git commit -m "feat(view_state): wire pending/snapshot buffers into frame loop"
```

---

### Task 9: Headless reproducibility — drain pending + skip auto-fit

**Files:**
- Modify: `crates/polyscope/src/headless.rs`

- [ ] **Step 1: Write the failing integration test**

Create `crates/polyscope/tests/view_state_headless.rs`:

```rust
//! Integration test for view-state reproducibility through headless rendering.

use polyscope_rs::*;
use polyscope_core::view_state::{
    apply_view_state, current_view_state, ViewTransition,
};

#[test]
fn test_load_view_then_headless_uses_loaded_view() {
    // Initialize polyscope
    init().expect("init");

    // Register a tiny scene
    register_point_cloud("p", vec![Vec3::ZERO, Vec3::X, Vec3::Y]);

    // Render one preview frame so a snapshot exists
    let _ = render_to_image(64, 64).expect("first render");
    let baseline = current_view_state().expect("baseline");

    // Construct a view state with a deliberately-non-auto-fit camera
    let mut customised = baseline.clone();
    customised.camera.position = [10.0, 0.0, 0.0];
    customised.camera.target = [0.0, 0.0, 0.0];
    customised.camera.up = [0.0, 1.0, 0.0];

    // Queue the state, then render. Headless must apply it and skip auto-fit.
    apply_view_state(&customised, ViewTransition::Instant).expect("queue");
    let _ = render_to_image(64, 64).expect("second render");

    // After the second render, the published snapshot should match the queued
    // camera position (not auto-fit's value).
    let after = current_view_state().expect("after");
    assert!(
        (after.camera.position[0] - 10.0).abs() < 1e-4,
        "expected position[0] ≈ 10.0 (from loaded view), got {}",
        after.camera.position[0]
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test view_state_headless -- --nocapture`
Expected: FAIL — auto-fit clobbers the queued position.

- [ ] **Step 3: Modify `render_to_image` in headless.rs**

Replace the body of `render_to_image` (currently lines 52-76):

```rust
pub fn render_to_image(width: u32, height: u32) -> Result<Vec<u8>> {
    let mut app = App::new();

    // Create headless render engine
    let engine = RenderEngine::new_headless(width, height)
        .block_on()
        .map_err(|e| {
            crate::PolyscopeError::RenderError(format!("Failed to create headless engine: {e}"))
        })?;
    app.engine = Some(engine);

    // Clear stale GPU resources (as before)
    with_context_mut(|ctx| {
        for structure in ctx.registry.iter_mut() {
            structure.clear_gpu_resources();
        }
    });

    // Drain any queued view-state. If a state is applied here, we mark
    // camera_fitted = true via apply_view_state so the per-frame auto-fit
    // becomes a no-op.
    let pending = with_context_mut(|ctx| ctx.pending_view_apply.take());
    if let Some((state, transition)) = pending {
        app.apply_view_state(&state, transition);
    }

    // Render one frame and capture
    app.render_frame_headless();
    app.capture_to_buffer()
}
```

The single behavioral change: `pending_view_apply` is drained before `render_frame_headless`, and `apply_view_state` sets `camera_fitted = true` (which `auto_fit_camera` checks — see `render_init.rs:12`).

- [ ] **Step 4: Run the test again**

Run: `cargo test --test view_state_headless -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run full workspace tests + clippy + fmt + commit**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope/src/headless.rs crates/polyscope/tests/view_state_headless.rs
git commit -m "feat(view_state): headless reproducibility - drain pending + skip auto-fit"
```

---

### Task 10: `ViewAction` extension + UI buttons + main-crate deps

**Files:**
- Modify: `crates/polyscope/Cargo.toml`
- Modify: `crates/polyscope-ui/src/panels.rs`

- [ ] **Step 1: Add deps to the main crate**

In `crates/polyscope/Cargo.toml`, add to `[dependencies]`:

```toml
serde_json = { workspace = true }
rfd = "0.15"
```

`serde_json` is already a workspace dep.

- [ ] **Step 2: Extend `ViewAction`**

In `crates/polyscope-ui/src/panels.rs`, find `pub enum ViewAction` (around line 303). Extend it:

```rust
pub enum ViewAction {
    None,
    ResetView,
    Screenshot,
    /// User clicked "Save View…". App should open a save dialog after the
    /// egui frame is done and call `save_view_to_file(path)`.
    RequestSaveView,
    /// User clicked "Load View…". Same flow as `RequestSaveView`.
    RequestLoadView,
}
```

(Adjust if existing variants differ — the actual enum may have different variants like `Picking`, etc.)

- [ ] **Step 3: Add buttons in `build_controls_section`**

Find `build_controls_section` (line 613 area). After the Reset View / Screenshot buttons, add:

```rust
        ui.separator();
        if ui.button("Save View…").clicked() {
            action = ViewAction::RequestSaveView;
        }
        if ui.button("Load View…").clicked() {
            action = ViewAction::RequestLoadView;
        }
```

(The actual variable name `action` should match the existing pattern; check the function body.)

- [ ] **Step 4: Build and verify the new variants compile**

Run: `cargo build --workspace`
Expected: Builds (existing match in `render_ui.rs:77` may now be non-exhaustive — fix in Task 11).

If `cargo build` fails with a non-exhaustive match in `render_ui.rs`, that's expected — Task 11 handles it. To keep this task green: add a temporary catch-all `_ => {}` arm to the existing match in `render_ui.rs:77` (will be replaced in Task 11):

```rust
ViewAction::RequestSaveView | ViewAction::RequestLoadView => {
    // Wired up in Task 11
}
```

- [ ] **Step 5: Commit**

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope/Cargo.toml crates/polyscope-ui/src/panels.rs crates/polyscope/src/app/render_ui.rs
git commit -m "feat(view_state): ViewAction variants + UI buttons"
```

---

### Task 11: App-loop dispatch — rfd dialog + save/load wiring

**Files:**
- Modify: `crates/polyscope/src/app/render_ui.rs`

- [ ] **Step 1: Implement the dispatch**

In `crates/polyscope/src/app/render_ui.rs`, replace the temporary catch-all arm (from Task 10) in the `match view_action` block with proper handlers. The dispatch runs **after** the egui frame has been built and laid out (multi-pass complete). Place this right after the existing `match view_action` block, or inside it if the existing match runs post-frame:

```rust
match view_action {
    // ...existing arms...
    ViewAction::RequestSaveView => {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Polyscope View", &["json"])
            .set_file_name("view.json")
            .save_file()
        {
            if let Err(e) = polyscope_core::view_state::save_view_to_file(&path) {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Save View failed")
                    .set_description(&e.to_string())
                    .show();
            }
        }
    }
    ViewAction::RequestLoadView => {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Polyscope View", &["json"])
            .pick_file()
        {
            if let Err(e) = polyscope_core::view_state::load_view_from_file(
                &path,
                polyscope_core::view_state::ViewTransition::FlyTo,
            ) {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Load View failed")
                    .set_description(&e.to_string())
                    .show();
            }
        }
    }
}
```

If the existing `match` runs **inside** the egui frame (during builder), wrap the dispatch with `// Run after egui frame completes` comment and verify it's actually called post-frame by reading the surrounding context. The design requires it: opening rfd inside the egui builder reruns the dialog on every layout pass.

If the match is currently inside the builder, refactor: emit `RequestSaveView`/`RequestLoadView` from the builder, but defer the actual handling to after `ctx.run(...)` completes. The cleanest way is to store the pending request in `App` and process it on the next event-loop iteration.

- [ ] **Step 2: Test manually (the only practical test)**

Run: `cargo run --release --example point_cloud_demo`

In the running window:
1. Click "Save View…" — native save dialog appears.
2. Pick a path like `/tmp/v.json` — file is written.
3. Open `/tmp/v.json` and verify the JSON shape (version, camera, render).
4. Manipulate the camera (drag to rotate).
5. Click "Load View…" — open dialog appears.
6. Pick `/tmp/v.json` — camera flies back to the saved pose.

- [ ] **Step 3: Commit**

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
git add crates/polyscope/src/app/render_ui.rs
git commit -m "feat(view_state): dispatch UI save/load via rfd dialogs"
```

---

### Task 12: Demo example

**Files:**
- Create: `examples/view_state_demo.rs`
- Modify: `crates/polyscope/Cargo.toml` (register the example)

- [ ] **Step 1: Write the example**

Create `examples/view_state_demo.rs`:

```rust
//! Demonstrates the view save/restore API.
//!
//! Run with: `cargo run --release --example view_state_demo`
//!
//! In the window: orbit the camera, then use Save View / Load View buttons
//! in the right panel to write/read `/tmp/polyscope-view.json`.

use polyscope_rs::*;

fn main() -> Result<()> {
    env_logger::init();
    init()?;

    // A small grid of points to look at
    let mut pts = Vec::new();
    for i in -3..=3 {
        for j in -3..=3 {
            for k in -3..=3 {
                pts.push(Vec3::new(i as f32, j as f32, k as f32) * 0.5);
            }
        }
    }
    let pc = register_point_cloud("grid", pts);
    pc.set_radius(0.05);

    println!("Use the Save View / Load View buttons in the right panel.");
    show();
    Ok(())
}
```

- [ ] **Step 2: Register the example**

In `crates/polyscope/Cargo.toml`, after the existing `[[example]]` blocks add:

```toml
[[example]]
name = "view_state_demo"
path = "../../examples/view_state_demo.rs"
```

- [ ] **Step 3: Build + manual verify**

Run: `cargo build --example view_state_demo`
Expected: Builds.

Run: `cargo clippy --example view_state_demo -- -D warnings`
Expected: Clean.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
git add examples/view_state_demo.rs crates/polyscope/Cargo.toml
git commit -m "docs(examples): view_state_demo - save/load view JSON"
```

---

### Task 13: Documentation updates

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/feature-status.md`

- [ ] **Step 1: Update `CHANGELOG.md`**

Find the `## [Unreleased]` section (added in the prism/pyramid PR) and append under `### Added`:

```markdown
- View save/restore: serialize the camera + render-look state (background,
  ground plane, transparency, SSAO, SSAA) to a JSON file and load it back.
  New API: `save_view_to_file`, `load_view_from_file`, `save_view_to_json`,
  `load_view_from_json`, `current_view_state`, `apply_view_state`, plus the
  `ViewTransition` enum (`Instant` / `FlyTo`). Two UI buttons in the View
  Controls section open native file dialogs. JSON format is Rust-native
  (not byte-compatible with C++ Polyscope view files) and versioned.
  Headless rendering honors a loaded view and skips auto-fit. New example
  `view_state_demo`.
```

If `## [Unreleased]` doesn't exist (PR may have merged), add one above the latest version.

- [ ] **Step 2: Update `docs/feature-status.md`**

In the `## Completed Features` section (around line 60), add:

```markdown
- [x] View save/restore JSON — serialize/deserialize camera + render-look state (upstream commit 7570a40, fix #389; Rust-native format, not byte-compatible with C++)
```

Also update the `### Upstream Ports (Medium-Term)` section: **remove** the line:

```markdown
- [ ] View save/restore JSON — serialize/deserialize camera state (upstream 7570a40, d034498)
```

(That item is now done.)

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md docs/feature-status.md
git commit -m "docs(view_state): CHANGELOG + feature-status.md updates"
```

---

## Self-Review

### Spec coverage

| Spec section | Task(s) |
|---|---|
| State ownership (Context buffers) | Task 1 |
| CameraState DTO + Camera::current_state/apply_state | Task 2, 3 |
| RenderState DTO + GroundPlane/Transparency string helpers | Task 4 |
| ViewState + PartialViewState + validation + version | Task 5 |
| Public free functions + error variants | Task 1, 6 |
| App::current_view_state + apply_view_state | Task 7 |
| Frame-loop drain/snapshot | Task 8 |
| Headless reproducibility | Task 9 |
| UI integration (ViewAction extension, buttons, dispatch) | Tasks 10, 11 |
| Demo example | Task 12 |
| Docs | Task 13 |

All spec sections covered.

### Placeholder scan

No "TBD" / "implement later" / "similar to Task N" / vague "handle X appropriately" patterns. Every code-step shows the actual code.

One known soft spot: Task 7 Step 3 says `engine.camera()` / `engine.camera_mut()` "may not exist with those exact names — verify". This is a verification step, not a placeholder; the executing agent will read the engine source and adjust. Similarly Task 10 Step 3 says "if existing variants differ, adjust". These are unavoidable because the spec didn't pin every existing identifier.

### Type consistency

- `ViewState` / `CameraStateOwned` / `RenderState` / `GroundPlaneState` / `TransparencyState` / `ViewTransition` are used consistently across Tasks 1–13.
- `polyscope_render::CameraState` ↔ `polyscope_core::view_state::CameraStateOwned` bridge via `From` impls (Task 7).
- `Camera::apply` takes `polyscope_core::view_state::ViewTransition` consistently.
- `ground_plane_mode_name` / `parse_ground_plane_mode` / `transparency_mode_name` / `parse_transparency_mode` defined in Task 4, used in Tasks 5, 7.
- `ViewState::CURRENT_VERSION = 1` defined in Task 5, referenced everywhere.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-19-view-save-restore-implementation.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
