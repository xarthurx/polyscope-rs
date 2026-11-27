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
    global_start: u32,
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

// Encode a flat 24-bit global index into RGB color
fn index_to_color(index: u32) -> vec3<f32> {
    let r = f32((index >> 16u) & 0xFFu) / 255.0;
    let g = f32((index >> 8u) & 0xFFu) / 255.0;
    let b = f32(index & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

// Encode the global pick index `start + local` into RGB color. An index past
// the 24 bits of the pick buffer draws as the background, also when the sum
// overflows `u32`, so it cannot select a different element.
fn encode_pick_index(start: u32, local: u32) -> vec3<f32> {
    if (start >= 0x1000000u || local >= 0x1000000u - start) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    return index_to_color(start + local);
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

    // Parallel-ray case: ray parallel to cylinder axis (e.g. ortho viewing
    // straight down a tube). Quadratic degenerates; intersect with end caps.
    // Without this, ortho-mode picks miss tubes viewed end-on.
    if (a < 1e-8) {
        if (dot(delta_perp, delta_perp) > cyl_radius * cyl_radius) {
            return false;
        }
        let ray_dot_cyl = dot(ray_dir, cyl_dir);
        if (abs(ray_dot_cyl) < 1e-8) {
            return false;
        }
        let t_start = dot(cyl_start - ray_origin, cyl_dir) / ray_dot_cyl;
        let t_end = dot(cyl_end - ray_origin, cyl_dir) / ray_dot_cyl;
        var t_cap = min(t_start, t_end);
        if (t_cap < 0.001) {
            t_cap = max(t_start, t_end);
            if (t_cap < 0.001) {
                return false;
            }
        }
        *t_hit = t_cap;
        *hit_point = ray_origin + t_cap * ray_dir;
        return true;
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

// Returns the point of the segment from `a` to `b` that is nearest to the
// ray. `ray_dir` has unit length.
//
// The parameter comes from the parts of the segment ends across the ray, so
// it stays accurate for a segment that is almost parallel to the ray. A
// segment parallel to the ray covers one point of the image, and its nearer
// end is in front there. The result does not depend on the order of the ends.
fn segment_point_nearest_ray(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    a: vec3<f32>,
    b: vec3<f32>,
) -> vec3<f32> {
    let a_rel = a - ray_origin;
    let b_rel = b - ray_origin;
    let a_along = dot(a_rel, ray_dir);
    let b_along = dot(b_rel, ray_dir);
    let a_across = a_rel - a_along * ray_dir;
    let v_across = (b_rel - b_along * ray_dir) - a_across;
    let vv_across = dot(v_across, v_across);
    let v = b - a;
    if (vv_across <= 1e-12 * dot(v, v)) {
        return select(a, b, b_along < a_along);
    }
    let s = clamp(-dot(a_across, v_across) / vv_across, 0.0, 1.0);
    return a + s * v;
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

    // Setup ray. Perspective: from camera through fragment. Orthographic: parallel
    // along world-space view forward, pushed back behind the cylinder so t > 0.
    var ray_origin: vec3<f32>;
    var ray_dir: vec3<f32>;
    if (camera.camera_pos.w > 0.5) {
        ray_dir = -vec3<f32>(camera.view[0].z, camera.view[1].z, camera.view[2].z);
        let cyl_extent = length(tip - tail) + 2.0 * radius;
        ray_origin = in.world_position - cyl_extent * ray_dir;
    } else {
        ray_origin = camera.camera_pos.xyz;
        ray_dir = normalize(in.world_position - ray_origin);
    }

    // Ray-cylinder intersection
    var t_hit: f32;
    var hit_point: vec3<f32>;

    if (!ray_cylinder_intersect(ray_origin, ray_dir, tail, tip, radius,
                                 &t_hit, &hit_point)) {
        discard;
    }

    // The depth is the depth of what the scene draws: the tube surface where
    // the ray hits the drawn tube, else the curve line (`pick.radius` is 0 for
    // a curve that draws lines). Thus the wider pick cylinder does not cover
    // nearer geometry in the pick buffer.
    var depth_point = segment_point_nearest_ray(ray_origin, ray_dir, tail, tip);
    var t_drawn: f32;
    var drawn_point: vec3<f32>;
    if (pick.radius > 0.0
        && ray_cylinder_intersect(ray_origin, ray_dir, tail, tip, pick.radius,
                                  &t_drawn, &drawn_point)) {
        depth_point = drawn_point;
    }
    let clip_pos = camera.view_proj * vec4<f32>(depth_point, 1.0);
    out.depth = clip_pos.z / clip_pos.w;

    // Output encoded pick ID
    let pick_color = encode_pick_index(pick.global_start, in.edge_id);
    out.color = vec4<f32>(pick_color, 1.0);

    return out;
}
