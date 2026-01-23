// Ground plane shader with checker pattern
// Uses ray-plane intersection to render infinite ground

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct GroundUniforms {
    color1: vec4<f32>,
    color2: vec4<f32>,
    height: f32,
    tile_size: f32,
    transparency: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> ground: GroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_ray_dir: vec3<f32>,
}

// Fullscreen triangle vertices
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle (covers [-1,1] clip space)
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.999, 1.0);

    // Compute world-space ray direction from camera through this pixel
    let clip_pos = vec4<f32>(x, y, 1.0, 1.0);
    let world_pos = camera.inv_view_proj * clip_pos;
    let world_pos3 = world_pos.xyz / world_pos.w;
    out.world_ray_dir = normalize(world_pos3 - camera.camera_pos.xyz);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_origin = camera.camera_pos.xyz;
    let ray_dir = normalize(in.world_ray_dir);

    // Intersect ray with horizontal plane at y = height
    // Ray: P = O + t*D
    // Plane: y = height
    // Solve: O.y + t*D.y = height => t = (height - O.y) / D.y

    let denom = ray_dir.y;
    if (abs(denom) < 0.0001) {
        // Ray parallel to plane
        discard;
    }

    let t = (ground.height - ray_origin.y) / denom;
    if (t < 0.0) {
        // Intersection behind camera
        discard;
    }

    let hit_point = ray_origin + t * ray_dir;

    // Checker pattern
    let grid_x = floor(hit_point.x / ground.tile_size);
    let grid_z = floor(hit_point.z / ground.tile_size);
    let checker = ((i32(grid_x) + i32(grid_z)) % 2 + 2) % 2; // Handle negative

    var color: vec3<f32>;
    if (checker == 0) {
        color = ground.color1.rgb;
    } else {
        color = ground.color2.rgb;
    }

    // Fade out with distance for anti-aliasing at horizon
    let dist = length(hit_point - ray_origin);
    let fade = exp(-dist * 0.001);
    let alpha = (1.0 - ground.transparency) * fade;

    return vec4<f32>(color, alpha);
}
