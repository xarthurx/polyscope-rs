// Ground plane shader using vertices at infinity
// Matches the original C++ Polyscope implementation
// Uses screen-space planar shadow sampling (like C++ Polyscope)

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct GroundUniforms {
    center: vec4<f32>,        // Scene center (xyz) + padding
    basis_x: vec4<f32>,       // Forward direction on ground plane
    basis_y: vec4<f32>,       // Right direction on ground plane
    basis_z: vec4<f32>,       // Up direction (normal to ground)
    height: f32,              // Ground plane height
    length_scale: f32,        // Scene length scale for tiling
    camera_height: f32,       // Camera height for fade calculation
    up_sign: f32,             // +1 or -1 depending on up direction
    shadow_darkness: f32,     // Shadow darkness (0.0 = no shadow, 1.0 = full black)
    shadow_mode: u32,         // 0=none, 1=shadow_only, 2=tile_with_shadow
    is_orthographic: u32,     // 0=perspective, 1=orthographic
    reflection_intensity: f32, // Reflection intensity (0=opaque, 1=mirror)
    viewport_dim: vec2<f32>,  // Viewport dimensions for screen-space sampling
    _padding: vec2<f32>,
}

// Note: LightUniforms kept for backward compatibility but not used for planar shadows
struct LightUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;
@group(0) @binding(2) var<uniform> light: LightUniforms;
// Shadow texture is now a regular texture (blurred shadow mask from planar projection)
@group(0) @binding(3) var shadow_texture: texture_2d<f32>;
@group(0) @binding(4) var shadow_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pos_world_homog: vec4<f32>,
}

// Screen-space shadow sampling (like C++ Polyscope)
// The shadow texture contains a blurred mask where:
// - 1.0 = shadow (geometry was projected here)
// - 0.0 = no shadow
fn calculate_shadow_screen_space(screen_pos: vec4<f32>) -> f32 {
    // Convert screen position to UV coordinates
    // screen_pos.xy is in pixels, viewport_dim gives us the conversion
    let screen_uv = vec2<f32>(
        screen_pos.x / ground.viewport_dim.x,
        screen_pos.y / ground.viewport_dim.y
    );

    // Sample the shadow texture
    let shadow_val = textureSample(shadow_texture, shadow_sampler, screen_uv).r;

    // Apply power function to soften edges (like C++ Polyscope)
    return pow(clamp(shadow_val, 0.0, 1.0), 0.25);
}

// Ground plane geometry: center vertex + 4 vertices at infinity (perspective)
// or large finite vertices (orthographic)
// Forms 4 triangles covering the entire plane
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
        corners[0] = vec4<f32>(base + offset_x + offset_y, 1.0);  // +X +Y
        corners[1] = vec4<f32>(base - offset_x + offset_y, 1.0);  // -X +Y
        corners[2] = vec4<f32>(base - offset_x - offset_y, 1.0);  // -X -Y
        corners[3] = vec4<f32>(base + offset_x - offset_y, 1.0);  // +X -Y
    } else {
        // Perspective: use original infinite vertex technique
        // Center at ground height relative to scene center (consistent with orthographic)
        let base = ground.center.xyz + ground.basis_z.xyz * ground.height;
        center = vec4<f32>(base, 1.0);

        // Corners at infinity (w=0)
        corners[0] = vec4<f32>( ground.basis_x.xyz + ground.basis_y.xyz, 0.0);  // +X +Y
        corners[1] = vec4<f32>(-ground.basis_x.xyz + ground.basis_y.xyz, 0.0);  // -X +Y
        corners[2] = vec4<f32>(-ground.basis_x.xyz - ground.basis_y.xyz, 0.0);  // -X -Y
        corners[3] = vec4<f32>( ground.basis_x.xyz - ground.basis_y.xyz, 0.0);  // +X -Y
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

    // Both perspective and orthographic vertices are already positioned at the correct height
    // (center at ground.center + height, corners either at infinity or at large offsets)
    let adjusted_pos = world_pos;

    out.position = camera.view_proj * adjusted_pos;
    out.pos_world_homog = adjusted_pos;

    return out;
}

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

    // Calculate shadow using screen-space sampling if shadow mode is enabled
    var shadow_val = 0.0;
    if (ground.shadow_mode > 0u) {
        shadow_val = calculate_shadow_screen_space(in.position);
    }

    // Shadow-only mode: just draw the shadow as a transparent overlay
    if (ground.shadow_mode == 1u) {
        // Calculate fade
        let dist_from_center = length(coord_2d);
        let dist_fade = 1.0 - smoothstep(8.0, 8.5, dist_from_center);
        let height_diff = ground.up_sign * (ground.camera_height - ground.height) / ground.length_scale;
        let below_fade = smoothstep(0.0, 0.1, height_diff);
        let fade_factor = min(dist_fade, below_fade);

        if (fade_factor <= 0.0) {
            discard;
        }

        // shadow_val is 1.0 where shadow, 0.0 where no shadow
        let shadow_amount = shadow_val * ground.shadow_darkness * fade_factor;

        if (shadow_amount < 0.01) {
            discard;
        }

        // Draw shadow as semi-transparent black
        return vec4<f32>(0.0, 0.0, 0.0, shadow_amount);
    }

    // Tile mode: draw the full ground plane with optional shadows

    // Checker stripe pattern (subtle lines between tiles)
    let mod_dist = min(
        min(fract(coord_2d.x), fract(coord_2d.y)),
        min(fract(-coord_2d.x), fract(-coord_2d.y))
    );
    let stripe_blend = smoothstep(0.005, 0.01, mod_dist);

    // Base grey color with darker stripes
    let base_color = vec3<f32>(0.75, 0.75, 0.75);
    let stripe_color = base_color * 0.2;
    let ground_color = mix(stripe_color, base_color, stripe_blend);

    // Simple lighting
    let view_pos = (camera.view * in.pos_world_homog).xyz / (camera.view * in.pos_world_homog).w;
    let normal_camera = (camera.view * vec4<f32>(ground.basis_z.xyz, 0.0)).xyz;
    let light_pos = vec3<f32>(5.0, 5.0, -5.0) * ground.length_scale;
    let light_dir = normalize(light_pos - view_pos);
    let eye_dir = normalize(-view_pos);

    // Diffuse lighting (simplified Oren-Nayar approximation)
    let n_dot_l = max(dot(normal_camera, light_dir), 0.0);
    let diffuse = 1.2 * n_dot_l + 0.3;

    // Specular
    let half_vec = normalize(light_dir + eye_dir);
    let n_dot_h = max(dot(normal_camera, half_vec), 0.0);
    let specular = 0.25 * pow(n_dot_h, 12.0);

    // Compute shadow factor from shadow_val
    // shadow_val is 1.0 where shadow, 0.0 where no shadow
    // shadow_factor should be 1.0 where lit, (1.0 - darkness) where shadowed
    let shadow_factor = mix(1.0, 1.0 - ground.shadow_darkness, shadow_val);

    // Apply lighting and shadow
    var lit_color = ground_color * diffuse * shadow_factor + vec3<f32>(1.0, 1.0, 1.0) * specular * shadow_factor;

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

    // When reflection is enabled, reduce ground plane opacity to let reflection show through
    // reflection_intensity=0 -> full opacity, reflection_intensity=1 -> very transparent
    let reflection_transparency = 1.0 - ground.reflection_intensity * 0.7;
    let final_alpha = fade_factor * reflection_transparency;

    // Premultiplied alpha output
    lit_color *= final_alpha;
    return vec4<f32>(lit_color, final_alpha);
}
