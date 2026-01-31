// Pick buffer shader - renders elements with unique colors encoding their ID
//
// This shader is used for element selection. Each element (point, face, etc.)
// is rendered with a unique color that encodes its global index.
// When the user clicks, we read the pixel at that position and decode the color
// to find which structure and element was clicked.

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    global_start: u32,
    point_radius: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick_uniforms: PickUniforms;
@group(0) @binding(2) var<storage, read> point_positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pick_color: vec3<f32>,
    @location(1) sphere_center_view: vec3<f32>,
    @location(2) quad_pos: vec2<f32>,
    @location(3) point_radius: f32,
}

// Billboard quad vertices (two triangles)
const QUAD_VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

// Encode a flat 24-bit global index into RGB color
fn index_to_color(index: u32) -> vec3<f32> {
    let r = f32((index >> 16u) & 0xFFu) / 255.0;
    let g = f32((index >> 8u) & 0xFFu) / 255.0;
    let b = f32(index & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Get point position (stored as vec4, using xyz)
    let world_pos = point_positions[instance_index].xyz;
    let view_pos = (camera.view * vec4<f32>(world_pos, 1.0)).xyz;

    // Get quad vertex
    let quad_pos = QUAD_VERTICES[vertex_index];

    // Compute billboard offset in view space (always facing camera)
    let radius = pick_uniforms.point_radius;
    let offset = vec3<f32>(quad_pos * radius, 0.0);
    let billboard_pos_view = view_pos + offset;

    // Project to clip space
    out.clip_position = camera.proj * vec4<f32>(billboard_pos_view, 1.0);

    // Encode the pick color from global_start + instance_index
    out.pick_color = index_to_color(pick_uniforms.global_start + instance_index);

    // Pass through for ray-sphere intersection
    out.sphere_center_view = view_pos;
    out.quad_pos = quad_pos;
    out.point_radius = radius;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ray-sphere intersection in view space (same as main shader)
    // Ray starts at fragment position on billboard, goes toward -Z (into screen)
    let ray_origin = vec3<f32>(
        in.sphere_center_view.xy + in.quad_pos * in.point_radius,
        in.sphere_center_view.z
    );
    let ray_dir = vec3<f32>(0.0, 0.0, -1.0);

    // Sphere at sphere_center_view with radius point_radius
    let oc = ray_origin - in.sphere_center_view;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - in.point_radius * in.point_radius;
    let discriminant = b * b - 4.0 * a * c;

    // Discard if ray misses sphere
    if (discriminant < 0.0) {
        discard;
    }

    // Output the pick color (no lighting needed for picking)
    return vec4<f32>(in.pick_color, 1.0);
}
