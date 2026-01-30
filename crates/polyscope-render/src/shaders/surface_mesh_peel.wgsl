// Surface mesh depth-peeling shader
// Same as surface_mesh.wgsl but discards fragments at or in front of the previous peel depth.

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

struct MeshUniforms {
    model: mat4x4<f32>,
    shade_style: u32,      // 0 = smooth, 1 = flat, 2 = tri-flat
    show_edges: u32,       // 0 = off, 1 = on
    edge_width: f32,
    transparency: f32,
    surface_color: vec4<f32>,
    edge_color: vec4<f32>,
    backface_policy: u32,  // 0 = identical, 1 = different, 2 = custom, 3 = cull
    slice_planes_enabled: u32, // 0 = off, 1 = on
    use_vertex_color: u32, // 0 = surface_color, 1 = per-vertex color
    _pad1: f32,
    _pad2_0: f32,
    _pad2_1: f32,
    _pad2_2: f32,
    _pad3: f32,
    backface_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> mesh_uniforms: MeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> barycentrics: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> colors: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read> edge_is_real: array<vec4<f32>>;

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

// Matcap textures (Group 2)
@group(2) @binding(0) var matcap_r: texture_2d<f32>;
@group(2) @binding(1) var matcap_g: texture_2d<f32>;
@group(2) @binding(2) var matcap_b: texture_2d<f32>;
@group(2) @binding(3) var matcap_k: texture_2d<f32>;
@group(2) @binding(4) var matcap_sampler: sampler;

// Depth peeling resources (Group 3)
@group(3) @binding(0) var t_min_depth: texture_2d<f32>;
@group(3) @binding(1) var peel_sampler: sampler;

// Matcap lighting: view-space normal -> UV -> 4-channel weighted blend
fn light_surface_matcap(normal: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    var n = normalize(normal);
    n.y = -n.y; // flip Y for camera convention
    n = n * 0.98; // scale to avoid edge artifacts
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
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) edge_real: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let local_position = positions[vertex_index].xyz;
    let local_normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index];

    let world_position = (mesh_uniforms.model * vec4<f32>(local_position, 1.0)).xyz;
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);

    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.barycentric = bary;
    out.vertex_color = color;
    out.edge_real = edge_is_real[vertex_index].xyz;

    return out;
}

struct PeelOutput {
    @location(0) color: vec4<f32>,
    @location(1) depth_out: vec4<f32>,
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> PeelOutput {
    // --- Depth peeling: discard if at or in front of previous peel depth ---
    let depth = in.clip_position.z;
    let viewport_dim = vec2<f32>(textureDimensions(t_min_depth));
    let peel_uv = in.clip_position.xy / viewport_dim;
    let min_depth = textureSample(t_min_depth, peel_sampler, peel_uv).r;
    // Use epsilon large enough to account for f16 quantization in the min_depth texture.
    // The min_depth is stored in Rgba16Float (half precision, ~10-bit mantissa).
    // At depth 1.0, f16 precision is ~0.001, so we need epsilon > 0.001 to reliably
    // distinguish already-peeled layers from new ones.
    if (depth <= min_depth + 2e-3) {
        discard;
    }

    // Handle backface culling: if policy==3 and !front_facing, discard
    if (mesh_uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Slice plane culling
    if (mesh_uniforms.slice_planes_enabled != 0u) {
        for (var i = 0u; i < 4u; i = i + 1u) {
            let plane = slice_planes.planes[i];
            if (plane.enabled > 0.5) {
                let dist = dot(in.world_position - plane.origin, plane.normal);
                if (dist < 0.0) {
                    discard;
                }
            }
        }
    }

    // Determine base color based on backface policy
    var base_color = mesh_uniforms.surface_color.rgb;
    if (!front_facing) {
        switch (mesh_uniforms.backface_policy) {
            case 0u: {
                base_color = mesh_uniforms.surface_color.rgb;
            }
            case 1u: {
                base_color = mesh_uniforms.surface_color.rgb * 0.5;
            }
            case 2u: {
                base_color = mesh_uniforms.backface_color.rgb;
            }
            default: {
                base_color = mesh_uniforms.surface_color.rgb;
            }
        }
    }

    var per_element_alpha = 1.0;
    if (mesh_uniforms.use_vertex_color == 1u) {
        base_color = in.vertex_color.rgb;
        per_element_alpha = in.vertex_color.w;
    }

    // Calculate normal for lighting based on shade_style
    var normal: vec3<f32>;

    if (mesh_uniforms.shade_style == 0u) {
        normal = normalize(in.world_normal);
    } else {
        let dpdx_pos = dpdx(in.world_position);
        let dpdy_pos = dpdy(in.world_position);
        normal = normalize(cross(dpdx_pos, dpdy_pos));

        let view_dir = normalize(camera.camera_pos - in.world_position);
        if (dot(normal, view_dir) < 0.0) {
            normal = -normal;
        }
    }

    if (!front_facing) {
        normal = -normal;
    }

    // Apply matcap lighting
    let view_normal_for_matcap = normalize((camera.view * vec4<f32>(normal, 0.0)).xyz);
    var color = light_surface_matcap(view_normal_for_matcap, base_color);

    // Wireframe
    if (mesh_uniforms.show_edges == 1u) {
        let bary = in.barycentric;
        let edge_real = in.edge_real;

        var d = 1.0;
        if (edge_real.z > 0.5) {
            d = min(d, bary.z);
        }
        if (edge_real.x > 0.5) {
            d = min(d, bary.x);
        }
        if (edge_real.y > 0.5) {
            d = min(d, bary.y);
        }

        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }

    // Compute alpha
    let alpha = (1.0 - mesh_uniforms.transparency) * per_element_alpha;

    if (alpha <= 0.0) {
        discard;
    }

    var out: PeelOutput;
    out.color = vec4<f32>(color * alpha, alpha);
    out.depth_out = vec4<f32>(depth, 0.0, 0.0, 1.0);
    return out;
}
