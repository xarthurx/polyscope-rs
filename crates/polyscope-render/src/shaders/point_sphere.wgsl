// Point sphere impostor shader using instanced rendering
// Each instance is a point rendered as a ray-cast sphere

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

// Slice plane uniforms for fragment-level slicing
struct SlicePlaneUniforms {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
}

struct SlicePlanesArray {
    planes: array<SlicePlaneUniforms, 4>,
}

struct PointUniforms {
    model: mat4x4<f32>,
    point_radius: f32,
    use_per_point_color: u32,  // 0 = base color, 1 = per-point color
    _padding: vec2<f32>,
    base_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> point_uniforms: PointUniforms;
@group(0) @binding(2) var<storage, read> point_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read> point_colors: array<vec3<f32>>;

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

// Matcap textures (Group 2)
@group(2) @binding(0) var matcap_r: texture_2d<f32>;
@group(2) @binding(1) var matcap_g: texture_2d<f32>;
@group(2) @binding(2) var matcap_b: texture_2d<f32>;
@group(2) @binding(3) var matcap_k: texture_2d<f32>;
@group(2) @binding(4) var matcap_sampler: sampler;

fn light_surface_matcap(normal: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    var n = normalize(normal);
    n.y = -n.y;
    n = n * 0.98;
    let uv = n.xy * 0.5 + vec2<f32>(0.5);
    let mat_r = textureSample(matcap_r, matcap_sampler, uv).rgb;
    let mat_g = textureSample(matcap_g, matcap_sampler, uv).rgb;
    let mat_b = textureSample(matcap_b, matcap_sampler, uv).rgb;
    let mat_k = textureSample(matcap_k, matcap_sampler, uv).rgb;
    return color.r * mat_r + color.g * mat_g
         + color.b * mat_b + (1.0 - color.r - color.g - color.b) * mat_k;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) sphere_center_view: vec3<f32>,
    @location(1) quad_pos: vec2<f32>,  // [-1, 1] on billboard quad
    @location(2) point_color: vec3<f32>,
    @location(3) point_radius: f32,
    @location(4) sphere_center_world: vec3<f32>,
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

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Get point position and apply model transform
    let local_pos = point_positions[instance_index];
    let world_pos = (point_uniforms.model * vec4<f32>(local_pos, 1.0)).xyz;
    let view_pos = (camera.view * vec4<f32>(world_pos, 1.0)).xyz;

    // Get quad vertex
    let quad_pos = QUAD_VERTICES[vertex_index];

    // Compute billboard offset in view space (always facing camera)
    let radius = point_uniforms.point_radius;
    let offset = vec3<f32>(quad_pos * radius, 0.0);
    let billboard_pos_view = view_pos + offset;

    // Project to clip space
    out.clip_position = camera.proj * vec4<f32>(billboard_pos_view, 1.0);
    out.sphere_center_view = view_pos;
    out.sphere_center_world = world_pos;
    out.quad_pos = quad_pos;
    out.point_radius = radius;

    // Get color
    if (point_uniforms.use_per_point_color == 1u) {
        out.point_color = point_colors[instance_index];
    } else {
        out.point_color = point_uniforms.base_color.rgb;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Slice plane culling - check sphere center against planes
    for (var i = 0u; i < 4u; i = i + 1u) {
        let plane = slice_planes.planes[i];
        if (plane.enabled > 0.5) {
            let dist = dot(in.sphere_center_world - plane.origin, plane.normal);
            if (dist < 0.0) {
                discard;
            }
        }
    }

    // Ray-sphere intersection in view space
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

    if (discriminant < 0.0) {
        discard;
    }

    let t = (-b - sqrt(discriminant)) / (2.0 * a);
    let hit_point = ray_origin + t * ray_dir;
    let normal = normalize(hit_point - in.sphere_center_view);

    // Matcap lighting: normal is already in view space from ray-sphere intersection
    let color = light_surface_matcap(normal, in.point_color);

    return vec4<f32>(color, 1.0);
}
