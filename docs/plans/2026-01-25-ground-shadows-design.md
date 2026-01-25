# Ground Shadows Implementation Design

**Date:** 2026-01-25

**Goal:** Render scene objects to a shadow map from the light's perspective, enabling the ground plane to display real shadows.

## Current State

The shadow infrastructure exists but is not connected:
- `ShadowMapPass` with 2048x2048 depth texture
- Light uniform buffer and comparison sampler
- `begin_shadow_pass()` and `compute_light_matrix()` methods
- Ground plane shader samples shadow map with PCF

**Missing:** No shadow pipeline, no shadow pass in render loop, shadow map always empty.

## Architecture

### Render Loop (Updated)

```
1. Shadow pass (objects → shadow map depth texture)  ← NEW
2. Main render pass (objects → HDR texture)
3. Ground plane pass (samples shadow map)
4. Tone mapping pass
```

### Shadow Pipeline

- Shader: `shadow_map.wgsl` (depth-only, no color output)
- Depth format: Depth32Float
- No multisampling
- TriangleList topology

### Shadow Bind Group Layout

```
Binding 0: Light uniforms (view_proj, light_dir)
Binding 1: Vertex positions (storage buffer)
```

### Shadow-Casting Structures

- SurfaceMesh: Yes (primary target)
- PointCloud: No (impostors don't work well)
- CurveNetwork: No (minimal shadow impact)
- VolumeMesh: Future enhancement
- VolumeGrid: No (wireframe only)

## Files to Modify

| File | Changes |
|------|---------|
| `engine.rs` | Add shadow pipeline, bind group layout |
| `surface_mesh_render.rs` | Add shadow bind group |
| `surface_mesh/mod.rs` | Add shadow resource init |
| `app.rs` | Add shadow pass to render loop |

## Success Criteria

- Shadows appear on ground plane under meshes
- Shadow darkness adjustable via UI
- No performance regression
