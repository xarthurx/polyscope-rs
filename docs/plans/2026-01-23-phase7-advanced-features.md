# Phase 7: Advanced Features Implementation Plan

**Date**: 2026-01-23
**Status**: In Progress

---

## Overview

Phase 7 adds advanced features to polyscope-rs:
1. ~~Ground plane~~ (Already implemented)
2. Screenshots
3. Materials system enhancement
4. Groups
5. Slice planes
6. Transformation gizmos

---

## Implementation Order (by complexity/dependencies)

### Task 1: Screenshots
**Complexity**: Low
**Dependencies**: None

Screenshots capture the rendered frame to an image file.

**Files to create/modify:**
- `crates/polyscope-render/src/screenshot.rs` - New file
- `crates/polyscope-render/src/lib.rs` - Export
- `crates/polyscope/src/lib.rs` - Public API

**Implementation:**
1. Add `screenshot()` function that reads back the render buffer
2. Support PNG and JPG formats via `image` crate
3. Options: filename, transparent background, include UI
4. Add `screenshot_to_buffer()` for programmatic use

**API:**
```rust
pub fn screenshot(filename: &str);
pub fn screenshot_with_options(filename: &str, options: ScreenshotOptions);
pub fn screenshot_to_buffer(options: ScreenshotOptions) -> Vec<u8>;
```

---

### Task 2: Materials System Enhancement
**Complexity**: Medium
**Dependencies**: None

Enhance the existing basic material system with proper material properties.

**Files to modify:**
- `crates/polyscope-render/src/materials.rs` - Add properties
- `crates/polyscope-render/src/shaders/` - Material-aware shaders
- Surface mesh uniforms - Add material parameters

**Implementation:**
1. Add material properties: specular, roughness, metallic, ambient
2. Create material presets matching C++ Polyscope (clay, wax, candy, etc.)
3. Add material selection to structure UI
4. Pass material uniforms to shaders

**Material Properties:**
```rust
pub struct Material {
    pub name: String,
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub roughness: f32,
}
```

---

### Task 3: Groups
**Complexity**: Medium
**Dependencies**: Registry modifications

Groups organize structures hierarchically.

**Files to create/modify:**
- `crates/polyscope-core/src/group.rs` - New file
- `crates/polyscope-core/src/registry.rs` - Group support
- `crates/polyscope-ui/src/lib.rs` - Group UI
- `crates/polyscope/src/lib.rs` - Public API

**Implementation:**
1. Create `Group` struct with name, parent, children
2. Groups can contain structures and other groups
3. Enable/disable propagates to children
4. UI shows groups as collapsible tree nodes

**API:**
```rust
pub fn create_group(name: &str) -> GroupHandle;
pub fn get_group(name: &str) -> Option<GroupHandle>;
pub fn remove_group(name: &str);

impl GroupHandle {
    pub fn add_child_structure(&self, structure_name: &str);
    pub fn add_child_group(&self, group_name: &str);
    pub fn set_enabled(&self, enabled: bool);
}
```

---

### Task 4: Slice Planes
**Complexity**: High
**Dependencies**: Shader modifications, all structures

Slice planes cut through geometry to reveal interior.

**Files to create/modify:**
- `crates/polyscope-core/src/slice_plane.rs` - New file
- `crates/polyscope-render/src/engine.rs` - Slice plane rendering
- `crates/polyscope-render/src/shaders/*.wgsl` - Discard logic
- `crates/polyscope-ui/src/lib.rs` - Slice plane UI
- `crates/polyscope/src/lib.rs` - Public API

**Implementation:**
1. SlicePlane struct: position, normal, enabled, draw_plane, draw_widget
2. Pass slice plane uniforms to all structure shaders
3. In fragment shaders: discard fragments on negative side of plane
4. Render visual plane representation
5. UI for adding/removing/configuring slice planes

**Shader Logic:**
```wgsl
// In fragment shader
if (dot(world_pos - slice_plane_origin, slice_plane_normal) < 0.0) {
    discard;
}
```

**API:**
```rust
pub fn add_slice_plane(name: &str) -> SlicePlaneHandle;
pub fn remove_slice_plane(name: &str);

impl SlicePlaneHandle {
    pub fn set_pose(&self, origin: Vec3, normal: Vec3);
    pub fn set_enabled(&self, enabled: bool);
    pub fn set_draw_plane(&self, draw: bool);
}
```

---

### Task 5: Transformation Gizmos
**Complexity**: High
**Dependencies**: UI integration, structure transforms

Interactive gizmos for translate/rotate/scale.

**Note**: The C++ version uses ImGuizmo. We'll implement a simpler version or integrate egui-gizmo if available.

**Files to create/modify:**
- `crates/polyscope-ui/src/gizmo.rs` - New file (or use external crate)
- `crates/polyscope-core/src/structure.rs` - Transform methods
- `crates/polyscope/src/app.rs` - Gizmo rendering

**Implementation:**
1. Gizmo rendering (translate: arrows, rotate: circles, scale: boxes)
2. Mouse interaction for manipulation
3. Apply transforms to selected structure
4. UI toggle for gizmo mode (translate/rotate/scale)

**Options:**
- Use `egui-gizmo` crate if compatible
- Or implement custom gizmo with basic translation support

**API:**
```rust
impl StructureHandle {
    pub fn set_transform(&self, transform: Mat4);
    pub fn get_transform(&self) -> Mat4;
}

// In UI, gizmo appears when structure is selected
```

---

## Summary

| Task | Complexity | Est. Effort | Dependencies |
|------|------------|-------------|--------------|
| Screenshots | Low | Small | None |
| Materials | Medium | Medium | None |
| Groups | Medium | Medium | Registry |
| Slice Planes | High | Large | Shaders |
| Gizmos | High | Large | UI |

**Recommended order**: 1 → 2 → 3 → 4 → 5

---

## Testing Strategy

1. **Screenshots**: Verify image output matches rendered view
2. **Materials**: Visual comparison with C++ Polyscope
3. **Groups**: Unit tests for hierarchy, enable/disable propagation
4. **Slice planes**: Visual tests with known geometry
5. **Gizmos**: Interactive testing required
