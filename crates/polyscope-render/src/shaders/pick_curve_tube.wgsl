// Tube-based pick shader for curve networks
// Uses ray-cylinder intersection for accurate picking of tube-rendered curves
// This provides a much larger clickable area compared to 1-pixel line picking

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct PickUniforms {
    structure_id: u32,
    radius: f32,
    // Minimum pick radius - ensures curves are always clickable even when very thin
    min_pick_radius: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: PickUniforms;
@group(0) @binding(2) var<storage, read> edge_vertices: array<vec4<f32>>;

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

// Encode structure_id (12 bits) and element_id (12 bits) into RGB
fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

// Ray-cylinder intersection
fn ray_cylinder_intersect(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    cyl_start: vec3<f32>,
    cyl_end: vec3<f32>,
    cyl_radius: f32,
    t_hit: ptr<function, f32>,
    hit_point: ptr<function, vec3<f32>>
) -> bool {
    let cyl_axis = cyl_end - cyl_start;
    let cyl_length = length(cyl_axis);
    if (cyl_length < 0.0001) {
        return false;
    }
    let cyl_dir = cyl_axis / cyl_length;

    // Vector from cylinder start to ray origin
    let delta = ray_origin - cyl_start;

    // Project ray direction and delta onto plane perpendicular to cylinder
    let ray_dir_perp = ray_dir - dot(ray_dir, cyl_dir) * cyl_dir;
    let delta_perp = delta - dot(delta, cyl_dir) * cyl_dir;

    // Quadratic coefficients for intersection with infinite cylinder
    let a = dot(ray_dir_perp, ray_dir_perp);
    if (a < 0.0001) {
        // Ray parallel to cylinder axis
        return false;
    }
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

    // Allow some tolerance at the ends for easier picking
    let tolerance = cyl_radius * 0.5;
    if (proj < -tolerance || proj > cyl_length + tolerance) {
        return false;
    }

    *t_hit = t;
    *hit_point = p;

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

    // Use at least the minimum pick radius for easier selection
    let radius = max(pick.radius, pick.min_pick_radius);

    // Setup ray from camera through this fragment
    let ray_origin = camera.camera_pos.xyz;
    let ray_dir = normalize(in.world_position - ray_origin);

    // Ray-cylinder intersection
    var t_hit: f32;
    var hit_point: vec3<f32>;

    if (!ray_cylinder_intersect(ray_origin, ray_dir, tail, tip, radius,
                                 &t_hit, &hit_point)) {
        discard;
    }

    // Compute depth
    let clip_pos = camera.view_proj * vec4<f32>(hit_point, 1.0);
    out.depth = clip_pos.z / clip_pos.w;

    // Output encoded pick ID
    let pick_color = encode_pick_id(pick.structure_id, in.edge_id);
    out.color = vec4<f32>(pick_color, 1.0);

    return out;
}
