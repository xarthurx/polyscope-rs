// Slice plane visualization shader
// Renders an infinite plane with a grid pattern

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
    transparency: f32,
    length_scale: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> plane: PlaneUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) plane_uv: vec2<f32>,
}

// Use points at infinity technique like C++ polyscope
// Plane lies in X=0 in local space, with Y and Z as tangent directions
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Plane geometry using homogeneous coordinates
    // 4 triangles forming a quad with vertices at infinity
    // Each triangle has one vertex at origin and two at infinity
    var positions = array<vec4<f32>, 12>(
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 1.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 0.0, -1.0, 0.0), vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, -1.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, -1.0, 0.0, 0.0),
    );

    let pos = positions[vertex_index];
    let world_pos = plane.transform * pos;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_pos;

    // Compute world position (for non-infinite vertices)
    if (world_pos.w > 0.001) {
        out.world_position = world_pos.xyz / world_pos.w;
    } else {
        out.world_position = world_pos.xyz * 1000.0; // Large value for infinite vertices
    }

    // Compute UV coordinates in plane's local space
    // Extract tangent vectors from transform matrix
    let tangent_y = (plane.transform * vec4<f32>(0.0, 1.0, 0.0, 0.0)).xyz;
    let tangent_z = (plane.transform * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz;
    out.plane_uv = vec2<f32>(dot(out.world_position, tangent_y), dot(out.world_position, tangent_z));

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Grid pattern
    let grid_size = plane.length_scale * 0.1;
    let uv = in.plane_uv / grid_size;

    // Compute grid lines using screen-space derivatives
    let grid = abs(fract(uv - 0.5) - 0.5) / fwidth(uv);
    let line = min(grid.x, grid.y);
    let grid_factor = 1.0 - min(line, 1.0);

    // Mix base color with grid color
    let color = mix(plane.color.rgb, plane.grid_color.rgb, grid_factor * 0.5);

    return vec4<f32>(color, plane.transparency);
}
