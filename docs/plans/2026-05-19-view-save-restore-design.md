# View Save / Restore JSON — Design

**Goal:** Add a public API and UI affordance to save the current camera + render-look state to a JSON file and later load it back, reproducing the visual state. Ports upstream C++ Polyscope's `getViewAsJson` / `setViewFromJson` (commit `7570a40` + the FOV fix in PR #389), but with a Rust-native format and idiomatic API.

**Why:** Reproducible figures and bookmarked camera poses are a common ask. Without this, users hand-edit camera fields or screenshot their workflow.

> **Revision note**: V2 of this design. V1 (same file) assumed camera + render-look state lived in `Context::options`; they do not. Camera lives in `RenderEngine::camera`, background/ground-plane in `App` fields, transparency in `AppearanceSettings`. V2 introduces a snapshot/pending buffer in `Context` so free-function APIs work despite the distributed ownership, and updates several other corners flagged in codex review.

---

## Decisions (from brainstorming)

1. **JSON format**: Rust-native (snake_case fields, snake_case enum string values). Not byte-compatible with upstream — files do not roundtrip with C++ Polyscope. Format is documented and versioned.
2. **Scope of state**: Camera pose + the render-look settings that determine what the rendered image looks like (background color, ground plane, SSAO, transparency, SSAA). Excludes scene state (structure visibility, slice planes) and session/runtime fields (`max_fps`, `give_focus_on_show`, `auto_compute_scene_extents`).
3. **API**: Both file-based and string-based, file API as a thin `std::fs` wrapper.
4. **Load transition**: Typed enum `ViewTransition { Instant, FlyTo }`.
5. **UI**: Buttons in the existing **controls** section (next to Reset View / Screenshot — that's where `ViewAction` is already emitted), not the Camera section. Native `rfd` file dialogs. Default UI transition is `FlyTo`. No clipboard, no keyboard shortcuts.

---

## State Ownership (the core mistake in V1)

Live state is split across three places:

| State | Owner | Notes |
|-------|-------|-------|
| `Camera` (position, target, fov, projection, ...) | `RenderEngine::camera` | not in `Context` |
| `App::background_color: Vec3` | `App` field | not in `Options` |
| `App::ground_plane: GroundPlaneConfig` (uses the live `ground_plane::GroundPlaneMode` enum with `TileReflection`) | `App` field | distinct from the stale `Options::ground_plane_mode` enum (has `SolidColor`) |
| Transparency mode | `AppearanceSettings` | not in `Options` |
| SSAA factor | `RenderEngine` (and mirrored in `AppearanceSettings`) | applied via `app/render_ui.rs:575` |
| `Options::ssao` (SsaoConfig) | `Context::options` | the SSAO config is in core Options |

Free-function callers (e.g., a UI button or a script that holds the global context lock) can only reach `Context`. So `save_view_to_json()` needs the App to publish its view state into `Context` each frame, and `load_view_from_json()` needs to queue a pending state for the App to consume next frame.

**Mechanism**: two new `pub(crate)` fields in `Context`:

```rust
pub struct Context {
    // ...existing fields...

    /// Live snapshot of the App's view state, refreshed each frame.
    /// `None` until the first frame renders (also `None` between Apps).
    pub(crate) view_state_snapshot: Option<ViewState>,

    /// Pending view state to apply on the next frame.
    /// Consumed (taken) by the App at frame start.
    pub(crate) pending_view_apply: Option<(ViewState, ViewTransition)>,
}
```

App lifecycle:
- **Frame start (after handling events, before rendering)**: `if let Some((state, t)) = ctx.pending_view_apply.take() { self.apply_view_state(&state, t); }`.
- **Frame end (after camera + options finalized)**: `ctx.view_state_snapshot = Some(self.current_view_state());`.

Trade-off: a single `Clone` of `ViewState` per frame. The struct is ~250 bytes of POD (vecs of fixed length, enums) — negligible.

For **headless rendering**: `App::new_headless` drains `pending_view_apply` once at construction. If a state was queued, headless rendering **skips auto-fit** (otherwise the loaded view is immediately clobbered by `auto_fit_camera()`). This is the only way scripted reproducibility works.

---

## Data Model

A new `ViewState` struct composes `CameraState` (defined in `polyscope-render`) and `RenderState` (defined in `polyscope-core`). Each piece is `Serialize`/`Deserialize` in its own crate; the top-level wrapper lives in the main `polyscope` crate.

JSON shape (v1):

```json
{
  "version": 1,
  "camera": {
    "position": [0.0, 0.0, 3.0],
    "target": [0.0, 0.0, 0.0],
    "up": [0.0, 1.0, 0.0],
    "fov": 0.7854,
    "near": 0.01,
    "far": 1000.0,
    "projection_mode": "perspective",
    "ortho_scale": 1.0,
    "navigation_style": "turntable",
    "up_direction": "pos_y",
    "front_direction": "neg_z"
  },
  "render": {
    "background_color": [1.0, 1.0, 1.0],
    "ground_plane": {
      "enabled": true,
      "mode": "tile_reflection",
      "height": 0.0
    },
    "transparency": {
      "enabled": true,
      "mode": "simple",
      "render_passes": 8
    },
    "ssao": {
      "enabled": true,
      "radius": 0.5,
      "bias": 0.025,
      "intensity": 1.0,
      "sample_count": 16
    },
    "ssaa_factor": 1
  }
}
```

Key choices:
- **Camera is stored as position / target / up** (each a `Vec3`), not as a view matrix. Storing the basis vectors directly: (a) avoids the view-matrix-without-target problem (a view matrix doesn't encode the orbit target distance); (b) makes the JSON human-readable; (c) eliminates the inversion/reconstruction step. The view matrix is reconstructed at render time from these by existing code (`Camera::view_matrix()`).
- `aspect_ratio` and window size are **not** serialized — loading fits the current window.
- `version: 1` enables forward-compat. Unknown version → `PolyscopeError::InvalidViewState(...)`.
- `render.ground_plane.mode` uses the **live** `ground_plane::GroundPlaneMode` enum (`TileReflection`/`ShadowOnly`/`Tile`/`None`), not the stale `Options::GroundPlaneMode` with `SolidColor`. V2 cleanup of the duplicate enum is out of scope for this PR (separate issue).
- `ssao.sample_count` included (suggestion from review #1 — it affects the rendered look).
- **Enum serde renames are DTO-local**, not applied to the public enums. CameraState defines wrapper Serialize impls (or uses `#[serde(rename_all = "snake_case")]` only on private DTO types). No JSON-format change to the public enum derives. (Suggestion from review #3.)

### Partial deserialization

A separate `PartialViewState` DTO with `Option<T>` fields handles the "missing fields preserve current" requirement:

```rust
#[derive(Deserialize)]
struct PartialViewState {
    version: u32, // required
    camera: Option<PartialCameraState>,
    render: Option<PartialRenderState>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
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
// ...similar for PartialRenderState
```

Required: `version` only. Everything else falls back to the current value when missing. Loaded enum strings parse via a small helper that returns `InvalidViewState` on unknown values.

### Validation on load

Before applying, validate each present field:

- All float fields finite (`f32::is_finite`).
- `fov ∈ (0, π)`.
- `near > 0`, `near < far`.
- `ortho_scale > 0`.
- `ssaa_factor` ∈ `{1, 2, 4, 8}` (or whatever the renderer accepts — verify at impl time).
- `transparency.render_passes ≥ 1`.
- `ssao.sample_count` ∈ `[1, 256]`.
- `up`, `target - position` non-collinear and non-zero.

Failure → `PolyscopeError::InvalidViewState(reason: String)`.

---

## Public API

A new `view_state` module in `polyscope-core` exports `ViewState`, `CameraState`, `RenderState`, `PartialViewState`, `ViewTransition`, the file/string functions, and the validation function:

```rust
/// Whether to animate the camera transition when loading a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewTransition {
    /// Snap immediately to the loaded pose.
    #[default]
    Instant,
    /// Animate to the loaded pose using the existing camera flight (~0.4 s).
    FlyTo,
}

// ─── Snapshot / queue API (operate on the global Context) ───

/// Returns the latest view-state snapshot from the running App, or an error
/// if no frame has been rendered yet.
pub fn current_view_state() -> Result<ViewState>;

/// Queues a view-state application for the next frame.
pub fn apply_view_state(state: &ViewState, transition: ViewTransition) -> Result<()>;

// ─── String API ───
pub fn save_view_to_json() -> Result<String>;
pub fn load_view_from_json(json: &str, transition: ViewTransition) -> Result<()>;

// ─── File API ───
pub fn save_view_to_file(path: impl AsRef<Path>) -> Result<()>;
pub fn load_view_from_file(path: impl AsRef<Path>, transition: ViewTransition) -> Result<()>;
```

Behavior:
- `current_view_state` / `save_view_to_*` read `Context::view_state_snapshot`. Returns `PolyscopeError::NoActiveView` if the App hasn't rendered a frame yet.
- `apply_view_state` / `load_view_from_*` parse + validate, then store into `Context::pending_view_apply`. The App consumes and applies on next frame.
- `save_view_to_json` uses `serde_json::to_string_pretty`.
- `load_view_from_*` deserializes into `PartialViewState`, validates, merges with current snapshot to fill missing fields, produces a full `ViewState`, then queues.
- `apply_view_state(.., FlyTo)` animates only the camera (~0.4 s). Render-look changes apply immediately on the first frame — you don't want a 0.4 s SSAO fade-in.
- For **headless**: `App::new_headless` drains the pending queue if present and skips auto-fit. This means scripts can do `load_view_from_file(...)?; render_to_file(...)?` and get a reproducible view.

### Error model

Reuse existing `PolyscopeError` variants:

- `PolyscopeError::IoError(io::Error)` — already exists with `#[from]`. Used for file read/write.
- `PolyscopeError::JsonError(serde_json::Error)` — already exists with `#[from]`. Used for JSON parse failures.

Add one new variant:

- `PolyscopeError::InvalidViewState(String)` — version mismatch, validation failure, unknown enum value. Carries a human-readable reason.

No `ViewStateIo` / `ViewStateJson` — those would defeat `?`-composition with existing `IoError`/`JsonError`.

### Optional: `NoActiveView` variant

A `PolyscopeError::NoActiveView` (no message; static) for the "save called before any frame rendered" case. Or reuse `InvalidViewState("no view available yet")`. Decided at impl time — leaning toward a distinct variant for clearer error messages.

---

## UI Integration

Buttons go in `build_controls_section` (`crates/polyscope-ui/src/panels.rs:613`) — that's the section that already returns `ViewAction` and contains Reset View / Screenshot. Adding Save/Load there matches the existing layering and avoids changing `build_camera_settings_section`'s signature.

Extend `ViewAction`:

```rust
pub enum ViewAction {
    None,
    ResetView,
    Screenshot,
    // new:
    RequestSaveView, // open save dialog after egui frame
    RequestLoadView, // open load dialog after egui frame
    ShowError(String), // error to surface in a MessageDialog
}
```

**Why `RequestSaveView` / `RequestLoadView` instead of `SaveView(PathBuf)`**: egui runs the UI builder in a multi-pass layout (`app/render_ui.rs:61`). Opening rfd inside the panel builder would reopen the dialog on every layout pass. Solution: emit an intent; the app loop opens the dialog **after** the egui frame is done.

App-loop dispatch (after egui frame):

```rust
match view_action {
    ViewAction::RequestSaveView => {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Polyscope View", &["json"])
            .set_file_name("view.json")
            .save_file()
        {
            if let Err(e) = save_view_to_file(&path) {
                show_error_dialog(format!("Save failed: {e}"));
            }
        }
    }
    ViewAction::RequestLoadView => {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Polyscope View", &["json"])
            .pick_file()
        {
            if let Err(e) = load_view_from_file(&path, ViewTransition::FlyTo) {
                show_error_dialog(format!("Load failed: {e}"));
            }
        }
    }
    // ...existing variants
}
```

`show_error_dialog` is a small helper using `rfd::MessageDialog::new().set_level(MessageLevel::Error).show()`.

**rfd lives in the main `polyscope` crate**, not `polyscope-ui` — the dialog runs outside the egui frame, in the app loop. `polyscope-ui` just emits the request enum. (Fixes review #7 properly.)

---

## Test Plan

TDD, bottom-up:

| # | Test | Crate | Type |
|---|------|-------|------|
| 1 | `CameraState` roundtrip via `serde_json::Value` (tolerant float compare to 1e-5) | `polyscope-render` | unit |
| 2 | DTO-local enum rename: `serde_json::to_value(&CameraState{...})["navigation_style"] == "turntable"` | `polyscope-render` | unit |
| 3 | `RenderState` roundtrip | `polyscope-core` | unit |
| 4 | `PartialViewState` deserialize with only `version` → all fields `None` | `polyscope-core` | unit |
| 5 | Validation: `fov = 0` → `InvalidViewState`; `near > far` → `InvalidViewState`; non-finite floats → `InvalidViewState` | `polyscope-core` | unit |
| 6 | Unknown `version` → `InvalidViewState("unsupported version 2")` | `polyscope-core` | unit |
| 7 | Unknown enum string (`navigation_style: "xyz"`) → `InvalidViewState` | `polyscope-core` | unit |
| 8 | Field-level `serde_json::Value` comparison of `ViewState` against a fixed reference JSON (avoids float pretty-print fragility) | `polyscope-core` | unit |
| 9 | `apply_view_state(Instant)` then `current_view_state` returns the applied values (with a stub App that drains pending and writes snapshot) | `polyscope-core` | integration |
| 10 | `load_view_from_json(.., FlyTo)` triggers `Camera::flight = Some(_)` after one frame | `polyscope` | integration |
| 11 | `save_view_to_file` + `load_view_from_file` via `tempfile` | `polyscope` | integration |
| 12 | **Headless reproducibility**: queue view → `render_to_image()` → output pixel hash matches a reference (or at minimum, camera position matches the queued state, not the auto-fit) | `polyscope` | integration |

Not testable in CI: rfd dialog UX, visual FlyTo animation quality. Visual checks deferred to the PR.

---

## Files Touched

| Location | Change |
|----------|--------|
| `crates/polyscope-render/Cargo.toml` | Add `serde` dep. |
| `crates/polyscope-render/src/camera.rs` | New `CameraState` DTO with `#[serde(rename_all = "snake_case")]` on its enum-string fields. `Camera::current_state(&self) -> CameraState`. `Camera::apply_state(&mut self, &CameraState, ViewTransition)`. Existing flight code reused for FlyTo. |
| `crates/polyscope-core/Cargo.toml` | Already has serde + serde_json. |
| `crates/polyscope-core/src/state.rs` | Add `view_state_snapshot` + `pending_view_apply` fields to `Context`. |
| `crates/polyscope-core/src/view_state.rs` (new) | `ViewState`, `RenderState`, `PartialViewState`, `ViewTransition`, validation, the 6 public functions. |
| `crates/polyscope-core/src/error.rs` | Add `InvalidViewState(String)` and `NoActiveView` variants. |
| `crates/polyscope-core/src/options.rs` | Verify `SsaoConfig` field naming (sample_count etc.). No global enum renames. |
| `crates/polyscope/Cargo.toml` | Add `serde_json` direct dep; add `rfd` dep. |
| `crates/polyscope/src/app/mod.rs` | `App::current_view_state(&self) -> ViewState` (gathers camera + own fields + Options into a `ViewState`). `App::apply_view_state(&mut self, &ViewState, ViewTransition)`. Frame-start drains `pending_view_apply`; frame-end writes `view_state_snapshot`. |
| `crates/polyscope/src/app/render_ui.rs` | Dispatch `ViewAction::RequestSaveView` / `RequestLoadView` / `ShowError`. |
| `crates/polyscope/src/headless.rs` | Drain `pending_view_apply` in `new_headless`; skip auto-fit if a view was applied. |
| `crates/polyscope/src/lib.rs` | Re-export `view_state::*`. |
| `crates/polyscope-ui/src/panels.rs` | Two buttons in `build_controls_section`. |
| `crates/polyscope-ui/src/lib.rs` | Extend `ViewAction`. |
| `CHANGELOG.md`, `docs/feature-status.md` | New feature entry. No "Changed" entry — public enum serialization is unchanged. |

**Estimated size:** ~700 lines across ~12 files, ~10 commits. Larger than V1's estimate; the App-side plumbing and partial-DTO are the deltas.

---

## Risks

1. **Camera state reconstruction**: position/target/up + fov/near/far + projection_mode + ortho_scale fully reconstitutes the camera. No view-matrix inversion needed. Validated by test #1.
2. **App-Context sync timing**: `view_state_snapshot` is one frame stale by definition. UI button click → save reads the snapshot that was written at the end of the previous frame. This is the right semantic — the user clicked while looking at frame N, so they save frame N's view.
3. **Headless drain ordering**: pending state must be applied **before** auto-fit's first call, and auto-fit must be **skipped** if a state was applied. Pinned by test #12.
4. **Two `GroundPlaneMode` enums**: V2 deliberately targets only the live one (`ground_plane::GroundPlaneMode`). The stale `Options::GroundPlaneMode` is not touched. Unifying the two enums is a separate cleanup.
5. **`rfd` blocking the app loop**: ~100 ms stall after egui frame is rendered. Acceptable for explicit user action. If issues arise, swap to `AsyncFileDialog` later.

---

## Out of Scope

- Byte-compat with upstream C++ Polyscope JSON (chose Rust-native).
- Scene-state save (structure visibility, group state, slice planes) — separate larger feature.
- Unifying the two `GroundPlaneMode` enums — separate cleanup.
- View-state migration (only v1 exists).
- Clipboard copy/paste, keyboard shortcuts, drag-and-drop JSON loading.
- Web/wasm target support.
