// Surface mesh shader with shading modes and wireframe support
// Supports smooth, flat, and tri-flat shading with optional edge rendering

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
    _pad1_0: f32,
    _pad1_1: f32,
    _pad1_2: f32,
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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) edge_real: vec3<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Read position, normal, barycentric, color from storage buffers
    let local_position = positions[vertex_index].xyz;
    let local_normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index];

    // Apply model transform
    let world_position = (mesh_uniforms.model * vec4<f32>(local_position, 1.0)).xyz;
    // Transform normal with upper-left 3x3 of model matrix (assuming uniform scale)
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);

    // Transform position with view_proj matrix
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.barycentric = bary;
    out.vertex_color = color;
    out.edge_real = edge_is_real[vertex_index].xyz;

    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> FragmentOutput {
    // Handle backface culling: if policy==3 and !front_facing, discard
    if (mesh_uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Slice plane culling - discard fragments on negative side of enabled planes
    for (var i = 0u; i < 4u; i = i + 1u) {
        let plane = slice_planes.planes[i];
        if (plane.enabled > 0.5) {
            let dist = dot(in.world_position - plane.origin, plane.normal);
            if (dist < 0.0) {
                discard;
            }
        }
    }

    // Determine base color based on backface policy
    var base_color = mesh_uniforms.surface_color.rgb;
    if (!front_facing) {
        switch (mesh_uniforms.backface_policy) {
            case 0u: {
                // identical - use surface color
                base_color = mesh_uniforms.surface_color.rgb;
            }
            case 1u: {
                // different - darken surface color
                base_color = mesh_uniforms.surface_color.rgb * 0.5;
            }
            case 2u: {
                // custom - use backface color
                base_color = mesh_uniforms.backface_color.rgb;
            }
            default: {
                // fallback (shouldn't reach here due to discard above for case 3)
                base_color = mesh_uniforms.surface_color.rgb;
            }
        }
    }

    // Use per-vertex color if non-zero (for quantities)
    // Check if the color is non-default (not all zeros or very close to it)
    let color_sum = in.vertex_color.r + in.vertex_color.g + in.vertex_color.b;
    if (color_sum > 0.001) {
        base_color = in.vertex_color.rgb;
    }

    // Calculate normal for lighting based on shade_style
    var normal: vec3<f32>;

    if (mesh_uniforms.shade_style == 0u) {
        // Smooth shading: use interpolated vertex normals
        normal = normalize(in.world_normal);
    } else {
        // Flat/TriFlat shading: compute face normal from screen-space derivatives
        let dpdx_pos = dpdx(in.world_position);
        let dpdy_pos = dpdy(in.world_position);
        normal = normalize(cross(dpdx_pos, dpdy_pos));

        // Ensure the flat normal points toward the camera for front faces
        // The cross product direction depends on screen-space winding, so we
        // use the view direction to ensure consistent orientation
        let view_dir = normalize(camera.camera_pos - in.world_position);
        if (dot(normal, view_dir) < 0.0) {
            normal = -normal;
        }
    }

    // Flip normal for backfaces
    if (!front_facing) {
        normal = -normal;
    }

    // Apply lighting: higher ambient (0.5) for brightness from all directions
    // plus diffuse contribution from a directional light
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.5;
    let diffuse = 0.5 * max(dot(normal, light_dir), 0.0);
    let lighting = ambient + diffuse;

    var color = base_color * lighting;

    // Wireframe: if show_edges, mix edge_color based on barycentric distance
    // Only draw edges marked as real (not internal triangulation edges)
    if (mesh_uniforms.show_edges == 1u) {
        let bary = in.barycentric;
        let edge_real = in.edge_real;

        // Compute distance to each edge, but only consider real edges
        // Edge 0: opposite to vertex 0 (barycentric.x), between vertices 1-2
        // Edge 1: opposite to vertex 1 (barycentric.y), between vertices 2-0
        // Edge 2: opposite to vertex 2 (barycentric.z), between vertices 0-1
        var d = 1.0; // start with max distance
        if (edge_real.z > 0.5) { // edge from v0 to v1 (opposite to v2)
            d = min(d, bary.z);
        }
        if (edge_real.x > 0.5) { // edge from v1 to v2 (opposite to v0)
            d = min(d, bary.x);
        }
        if (edge_real.y > 0.5) { // edge from v2 to v0 (opposite to v1)
            d = min(d, bary.y);
        }

        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }

    // Return with transparency applied
    // Note: transparency of 0.0 means fully opaque (alpha = 1.0)
    // transparency of 1.0 means fully transparent (alpha = 0.0)
    let alpha = 1.0 - mesh_uniforms.transparency;

    // Compute view-space normal for SSAO
    let view_normal = (camera.view * vec4<f32>(normal, 0.0)).xyz;

    var out: FragmentOutput;
    out.color = vec4<f32>(color, alpha);
    out.normal = vec4<f32>(view_normal * 0.5 + 0.5, 1.0); // Encode to [0,1] range
    return out;
}
