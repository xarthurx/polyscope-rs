// Ground plane stencil shader
// Writes to stencil buffer to mark ground plane pixels for reflection
// Uses the same geometry as the main ground plane shader

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct GroundUniforms {
    center: vec4<f32>,
    basis_x: vec4<f32>,
    basis_y: vec4<f32>,
    basis_z: vec4<f32>,
    height: f32,
    length_scale: f32,
    camera_height: f32,
    up_sign: f32,
    shadow_darkness: f32,
    shadow_mode: u32,
    is_orthographic: u32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pos_world_homog: vec4<f32>,
}

// Same vertex shader as ground plane (handles both perspective and orthographic)
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Triangle index (0-3) and vertex within triangle (0-2)
    let tri_idx = vertex_index / 3u;
    let vert_idx = vertex_index % 3u;

    // For orthographic mode, use large finite vertices instead of infinite
    let ortho_scale = ground.length_scale * 100.0;

    var center: vec4<f32>;
    var corners: array<vec4<f32>, 4>;

    if (ground.is_orthographic == 1u) {
        // Orthographic: use finite vertices centered on ground.center
        let base = ground.center.xyz + ground.basis_z.xyz * ground.height;
        center = vec4<f32>(base, 1.0);

        let offset_x = ground.basis_x.xyz * ortho_scale;
        let offset_y = ground.basis_y.xyz * ortho_scale;
        corners[0] = vec4<f32>(base + offset_x + offset_y, 1.0);
        corners[1] = vec4<f32>(base - offset_x + offset_y, 1.0);
        corners[2] = vec4<f32>(base - offset_x - offset_y, 1.0);
        corners[3] = vec4<f32>(base + offset_x - offset_y, 1.0);
    } else {
        // Perspective: use original infinite vertex technique
        // Center at ground height (relative to origin, not ground.center)
        center = vec4<f32>(ground.basis_z.xyz * ground.height, 1.0);

        // Corners at infinity (w=0)
        corners[0] = vec4<f32>( ground.basis_x.xyz + ground.basis_y.xyz, 0.0);
        corners[1] = vec4<f32>(-ground.basis_x.xyz + ground.basis_y.xyz, 0.0);
        corners[2] = vec4<f32>(-ground.basis_x.xyz - ground.basis_y.xyz, 0.0);
        corners[3] = vec4<f32>( ground.basis_x.xyz - ground.basis_y.xyz, 0.0);
    }

    // Select vertices for this triangle
    var world_pos: vec4<f32>;
    if (vert_idx == 0u) {
        world_pos = center;
    } else if (vert_idx == 1u) {
        world_pos = corners[(tri_idx + 1u) % 4u];
    } else {
        world_pos = corners[tri_idx];
    }

    // Adjust position by ground height (only for infinite vertices with w=0)
    var adjusted_pos = world_pos;
    if (world_pos.w == 0.0) {
        adjusted_pos = world_pos + vec4<f32>(ground.basis_z.xyz, 0.0) * ground.height;
    }

    out.position = camera.view_proj * adjusted_pos;
    out.pos_world_homog = adjusted_pos;

    return out;
}

// Fragment shader for stencil - applies fade logic but writes no color
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert homogeneous coords to world position
    let world_pos = in.pos_world_homog.xyz / in.pos_world_homog.w;

    // Compute 2D coordinates on ground plane relative to center
    let coord = world_pos - ground.center.xyz;
    let scaled_coord = coord / (ground.length_scale * 0.5);
    let coord_2d = vec2<f32>(
        dot(ground.basis_x.xyz, scaled_coord),
        dot(ground.basis_y.xyz, scaled_coord)
    );

    // Fade off far away (at ~8 length scales from center)
    let dist_from_center = length(coord_2d);
    let dist_fade = 1.0 - smoothstep(8.0, 8.5, dist_from_center);

    // Fade when viewing from below
    let height_diff = ground.up_sign * (ground.camera_height - ground.height) / ground.length_scale;
    let below_fade = smoothstep(0.0, 0.1, height_diff);

    let fade_factor = min(dist_fade, below_fade);
    if (fade_factor <= 0.0) {
        discard;
    }

    // Return transparent - we only care about stencil writes
    // The stencil operation is set in the pipeline state
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
