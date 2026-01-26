// Curve network tube shader - renders cylinders via ray casting
// Vertex shader passes through bounding box geometry
// Fragment shader performs ray-cylinder intersection

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
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

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> edge_colors: array<vec4<f32>>;

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) edge_id_and_vertex_id: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) @interpolate(flat) edge_id: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.world_position = in.position.xyz;
    out.clip_position = camera.view_proj * in.position;
    out.edge_id = in.edge_id_and_vertex_id.x;

    return out;
}

// Ray-cylinder intersection
// Returns true if ray hits cylinder, outputs t_hit, hit_point, hit_normal
fn ray_cylinder_intersect(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    cyl_start: vec3<f32>,
    cyl_end: vec3<f32>,
    cyl_radius: f32,
    t_hit: ptr<function, f32>,
    hit_point: ptr<function, vec3<f32>>,
    hit_normal: ptr<function, vec3<f32>>
) -> bool {
    let cyl_axis = cyl_end - cyl_start;
    let cyl_length = length(cyl_axis);
    let cyl_dir = cyl_axis / cyl_length;

    // Vector from cylinder start to ray origin
    let delta = ray_origin - cyl_start;

    // Project ray direction and delta onto plane perpendicular to cylinder
    let ray_dir_perp = ray_dir - dot(ray_dir, cyl_dir) * cyl_dir;
    let delta_perp = delta - dot(delta, cyl_dir) * cyl_dir;

    // Quadratic coefficients for intersection with infinite cylinder
    let a = dot(ray_dir_perp, ray_dir_perp);
    let b = 2.0 * dot(ray_dir_perp, delta_perp);
    let c = dot(delta_perp, delta_perp) - cyl_radius * cyl_radius;

    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return false;
    }

    let sqrt_disc = sqrt(discriminant);
    var t = (-b - sqrt_disc) / (2.0 * a);

    // If t is negative, try the other intersection
    if (t < 0.001) {
        t = (-b + sqrt_disc) / (2.0 * a);
        if (t < 0.001) {
            return false;
        }
    }

    // Check if intersection is within cylinder bounds
    let p = ray_origin + t * ray_dir;
    let proj = dot(p - cyl_start, cyl_dir);

    if (proj < 0.0 || proj > cyl_length) {
        // Try cap intersection (simplified - just check if inside caps)
        // For now, just reject points outside the cylinder body
        return false;
    }

    // Compute normal (pointing outward from axis)
    let closest_on_axis = cyl_start + proj * cyl_dir;
    let normal = normalize(p - closest_on_axis);

    *t_hit = t;
    *hit_point = p;
    *hit_normal = normal;

    return true;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    // Get cylinder data
    let tail = edge_vertices[in.edge_id * 2u].xyz;
    let tip = edge_vertices[in.edge_id * 2u + 1u].xyz;
    let radius = uniforms.radius;

    // Setup ray from camera through this fragment
    let ray_origin = camera.camera_pos.xyz;
    let ray_dir = normalize(in.world_position - ray_origin);

    // Ray-cylinder intersection
    var t_hit: f32;
    var hit_point: vec3<f32>;
    var hit_normal: vec3<f32>;

    if (!ray_cylinder_intersect(ray_origin, ray_dir, tail, tip, radius,
                                 &t_hit, &hit_point, &hit_normal)) {
        discard;
    }

    // Slice plane culling - check the actual hit point
    for (var i = 0u; i < 4u; i = i + 1u) {
        let plane = slice_planes.planes[i];
        if (plane.enabled > 0.5) {
            let dist = dot(hit_point - plane.origin, plane.normal);
            if (dist < 0.0) {
                discard;
            }
        }
    }

    // Compute depth
    let clip_pos = camera.view_proj * vec4<f32>(hit_point, 1.0);
    out.depth = clip_pos.z / clip_pos.w;

    // Get color
    let ec = edge_colors[in.edge_id];
    var base_color: vec3<f32>;
    if (ec.r + ec.g + ec.b > 0.001) {
        base_color = ec.rgb;
    } else {
        base_color = uniforms.color.rgb;
    }

    // Simple lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let view_dir = -ray_dir;

    // Ambient
    let ambient = 0.3;

    // Diffuse
    let n_dot_l = max(dot(hit_normal, light_dir), 0.0);
    let diffuse = 0.6 * n_dot_l;

    // Specular
    let half_vec = normalize(light_dir + view_dir);
    let n_dot_h = max(dot(hit_normal, half_vec), 0.0);
    let specular = 0.3 * pow(n_dot_h, 32.0);

    let lit_color = base_color * (ambient + diffuse) + vec3<f32>(1.0) * specular;

    out.color = vec4<f32>(lit_color, 1.0);

    return out;
}
