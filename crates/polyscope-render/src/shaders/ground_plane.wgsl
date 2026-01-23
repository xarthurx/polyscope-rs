// Ground plane shader using vertices at infinity
// Matches the original C++ Polyscope implementation

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
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pos_world_homog: vec4<f32>,
}

// Ground plane geometry: center vertex + 4 vertices at infinity
// Forms 4 triangles covering the entire infinite plane
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Triangle index (0-3) and vertex within triangle (0-2)
    let tri_idx = vertex_index / 3u;
    let vert_idx = vertex_index % 3u;

    // Center vertex at ground height
    let center = vec4<f32>(ground.basis_z.xyz * ground.height, 1.0);

    // Four corner vertices at infinity (w=0)
    // Using basis_x (forward) and basis_y (right) directions
    var corners: array<vec4<f32>, 4>;
    corners[0] = vec4<f32>( ground.basis_x.xyz + ground.basis_y.xyz, 0.0);  // +X +Y
    corners[1] = vec4<f32>(-ground.basis_x.xyz + ground.basis_y.xyz, 0.0);  // -X +Y
    corners[2] = vec4<f32>(-ground.basis_x.xyz - ground.basis_y.xyz, 0.0);  // -X -Y
    corners[3] = vec4<f32>( ground.basis_x.xyz - ground.basis_y.xyz, 0.0);  // +X -Y

    // Select vertices for this triangle
    var world_pos: vec4<f32>;
    if (vert_idx == 0u) {
        world_pos = center;
    } else if (vert_idx == 1u) {
        world_pos = corners[(tri_idx + 1u) % 4u];
    } else {
        world_pos = corners[tri_idx];
    }

    // Adjust position by ground height (for finite vertices)
    let adjusted_pos = world_pos + vec4<f32>(ground.basis_z.xyz, 0.0) * ground.height * world_pos.w;

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

    // Apply lighting
    var lit_color = ground_color * diffuse + vec3<f32>(1.0, 1.0, 1.0) * specular;

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

    // Premultiplied alpha output
    lit_color *= fade_factor;
    return vec4<f32>(lit_color, fade_factor);
}
