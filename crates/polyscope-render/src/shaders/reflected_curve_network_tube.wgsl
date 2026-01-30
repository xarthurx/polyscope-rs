// Reflected curve network tube shader
// Renders cylinders via ray casting with reflection matrix applied

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

struct ReflectionUniforms {
    reflection_matrix: mat4x4<f32>,
    intensity: f32,
    ground_height: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> edge_colors: array<vec4<f32>>;
@group(1) @binding(0) var<uniform> reflection: ReflectionUniforms;

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

    // Apply reflection matrix to the bounding box vertex
    let reflected_pos = (reflection.reflection_matrix * in.position).xyz;

    out.world_position = reflected_pos;
    out.clip_position = camera.view_proj * vec4<f32>(reflected_pos, 1.0);
    out.edge_id = in.edge_id_and_vertex_id.x;

    return out;
}

// Ray-cylinder intersection
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

    let delta = ray_origin - cyl_start;

    let ray_dir_perp = ray_dir - dot(ray_dir, cyl_dir) * cyl_dir;
    let delta_perp = delta - dot(delta, cyl_dir) * cyl_dir;

    let a = dot(ray_dir_perp, ray_dir_perp);
    let b = 2.0 * dot(ray_dir_perp, delta_perp);
    let c = dot(delta_perp, delta_perp) - cyl_radius * cyl_radius;

    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return false;
    }

    let sqrt_disc = sqrt(discriminant);
    var t = (-b - sqrt_disc) / (2.0 * a);

    if (t < 0.001) {
        t = (-b + sqrt_disc) / (2.0 * a);
        if (t < 0.001) {
            return false;
        }
    }

    let p = ray_origin + t * ray_dir;
    let proj = dot(p - cyl_start, cyl_dir);

    if (proj < 0.0 || proj > cyl_length) {
        return false;
    }

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

    // Get cylinder data and apply reflection
    let tail_orig = edge_vertices[in.edge_id * 2u].xyz;
    let tip_orig = edge_vertices[in.edge_id * 2u + 1u].xyz;
    let tail = (reflection.reflection_matrix * vec4<f32>(tail_orig, 1.0)).xyz;
    let tip = (reflection.reflection_matrix * vec4<f32>(tip_orig, 1.0)).xyz;
    let radius = uniforms.radius;

    // Clip check - if both endpoints are above ground, skip
    if (tail.y > reflection.ground_height && tip.y > reflection.ground_height) {
        discard;
    }

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

    // Clip hit point above ground plane
    if (hit_point.y > reflection.ground_height) {
        discard;
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

    // Matcap lighting: transform world-space normal to view space
    let view_normal = normalize((camera.view * vec4<f32>(hit_normal, 0.0)).xyz);
    let lit_color = light_surface_matcap(view_normal, base_color);

    // Output with reflection intensity as alpha
    out.color = vec4<f32>(lit_color, reflection.intensity);

    return out;
}
