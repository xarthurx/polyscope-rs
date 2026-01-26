// Slice plane visualization shader
// Renders a bounded plane quad with a grid pattern (double-sided)

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PlaneUniforms {
    transform: mat4x4<f32>,  // Plane's object transform
    color: vec4<f32>,
    grid_color: vec4<f32>,
    length_scale: f32,
    plane_size: f32,         // Half-extent of the plane quad
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> plane: PlaneUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) plane_uv: vec2<f32>,
}

// Bounded quad geometry (6 vertices, cull_mode: None renders both sides)
// Plane lies in X=0 in local space, with Y and Z as tangent directions
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let size = plane.plane_size;

    // 6 vertices forming 2 triangles (single quad)
    var positions = array<vec3<f32>, 6>(
        vec3<f32>(0.0, -size, -size), // bottom-left
        vec3<f32>(0.0,  size, -size), // top-left
        vec3<f32>(0.0,  size,  size), // top-right
        vec3<f32>(0.0, -size, -size), // bottom-left
        vec3<f32>(0.0,  size,  size), // top-right
        vec3<f32>(0.0, -size,  size), // bottom-right
    );

    let local_pos = positions[vertex_index];
    let world_pos = plane.transform * vec4<f32>(local_pos, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    out.plane_uv = vec2<f32>(local_pos.y, local_pos.z);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Grid pattern based on world-space coordinates
    let grid_size = plane.length_scale * 0.1;

    // Use UV coordinates for grid (they're in local plane space)
    let uv = in.plane_uv / grid_size;

    // Compute grid lines using screen-space derivatives
    let grid = abs(fract(uv - 0.5) - 0.5) / fwidth(uv);
    let line = min(grid.x, grid.y);
    let grid_factor = 1.0 - min(line, 1.0);

    // Mix base color with grid color
    let color = mix(plane.color.rgb, plane.grid_color.rgb, grid_factor * 0.5);

    // Fixed opacity of 0.7
    return vec4<f32>(color, 0.7);
}
