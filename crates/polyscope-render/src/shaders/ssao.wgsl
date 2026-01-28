// SSAO (Screen Space Ambient Occlusion) shader
// Samples depth buffer in hemisphere around each pixel to estimate occlusion

struct SsaoUniforms {
    // Projection matrix for depth reconstruction
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    // SSAO parameters
    radius: f32,
    bias: f32,
    intensity: f32,
    sample_count: u32,
    // Screen dimensions for noise tiling
    screen_width: f32,
    screen_height: f32,
    _padding: vec2<f32>,
}

// Hemisphere sample kernel (precomputed, oriented along +Z)
// Well-distributed samples using cosine-weighted hemisphere sampling
// More samples near the center (higher weight areas) for quality
const KERNEL_SIZE: u32 = 64u;
var<private> kernel: array<vec3<f32>, 64> = array<vec3<f32>, 64>(
    // Ring 1 - close samples (scale 0.1-0.2)
    vec3<f32>(0.08, 0.00, 0.10),
    vec3<f32>(0.00, 0.08, 0.10),
    vec3<f32>(-0.08, 0.00, 0.10),
    vec3<f32>(0.00, -0.08, 0.10),
    vec3<f32>(0.06, 0.06, 0.12),
    vec3<f32>(-0.06, 0.06, 0.12),
    vec3<f32>(0.06, -0.06, 0.12),
    vec3<f32>(-0.06, -0.06, 0.12),
    // Ring 2 - medium-close samples (scale 0.2-0.3)
    vec3<f32>(0.15, 0.00, 0.15),
    vec3<f32>(0.00, 0.15, 0.15),
    vec3<f32>(-0.15, 0.00, 0.15),
    vec3<f32>(0.00, -0.15, 0.15),
    vec3<f32>(0.12, 0.12, 0.18),
    vec3<f32>(-0.12, 0.12, 0.18),
    vec3<f32>(0.12, -0.12, 0.18),
    vec3<f32>(-0.12, -0.12, 0.18),
    // Ring 3 - medium samples (scale 0.3-0.4)
    vec3<f32>(0.25, 0.00, 0.20),
    vec3<f32>(0.18, 0.18, 0.22),
    vec3<f32>(0.00, 0.25, 0.20),
    vec3<f32>(-0.18, 0.18, 0.22),
    vec3<f32>(-0.25, 0.00, 0.20),
    vec3<f32>(-0.18, -0.18, 0.22),
    vec3<f32>(0.00, -0.25, 0.20),
    vec3<f32>(0.18, -0.18, 0.22),
    // Ring 4 - medium-far samples (scale 0.4-0.5)
    vec3<f32>(0.30, 0.00, 0.25),
    vec3<f32>(0.22, 0.22, 0.28),
    vec3<f32>(0.00, 0.30, 0.25),
    vec3<f32>(-0.22, 0.22, 0.28),
    vec3<f32>(-0.30, 0.00, 0.25),
    vec3<f32>(-0.22, -0.22, 0.28),
    vec3<f32>(0.00, -0.30, 0.25),
    vec3<f32>(0.22, -0.22, 0.28),
    // Ring 5 - far samples (scale 0.5-0.6)
    vec3<f32>(0.38, 0.00, 0.30),
    vec3<f32>(0.27, 0.27, 0.32),
    vec3<f32>(0.00, 0.38, 0.30),
    vec3<f32>(-0.27, 0.27, 0.32),
    vec3<f32>(-0.38, 0.00, 0.30),
    vec3<f32>(-0.27, -0.27, 0.32),
    vec3<f32>(0.00, -0.38, 0.30),
    vec3<f32>(0.27, -0.27, 0.32),
    // Ring 6 - farther samples (scale 0.6-0.7)
    vec3<f32>(0.45, 0.00, 0.35),
    vec3<f32>(0.32, 0.32, 0.38),
    vec3<f32>(0.00, 0.45, 0.35),
    vec3<f32>(-0.32, 0.32, 0.38),
    vec3<f32>(-0.45, 0.00, 0.35),
    vec3<f32>(-0.32, -0.32, 0.38),
    vec3<f32>(0.00, -0.45, 0.35),
    vec3<f32>(0.32, -0.32, 0.38),
    // Ring 7 - outer samples (scale 0.7-0.85)
    vec3<f32>(0.52, 0.00, 0.40),
    vec3<f32>(0.37, 0.37, 0.42),
    vec3<f32>(0.00, 0.52, 0.40),
    vec3<f32>(-0.37, 0.37, 0.42),
    vec3<f32>(-0.52, 0.00, 0.40),
    vec3<f32>(-0.37, -0.37, 0.42),
    vec3<f32>(0.00, -0.52, 0.40),
    vec3<f32>(0.37, -0.37, 0.42),
    // Ring 8 - outermost samples (scale 0.85-1.0)
    vec3<f32>(0.60, 0.00, 0.45),
    vec3<f32>(0.42, 0.42, 0.48),
    vec3<f32>(0.00, 0.60, 0.45),
    vec3<f32>(-0.42, 0.42, 0.48),
    vec3<f32>(-0.60, 0.00, 0.45),
    vec3<f32>(-0.42, -0.42, 0.48),
    vec3<f32>(0.00, -0.60, 0.45),
    vec3<f32>(0.42, -0.42, 0.48),
);

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var noise_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> uniforms: SsaoUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Reconstruct view-space position from depth and UV
fn view_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Convert UV to clip space
    let clip = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    // Unproject to view space
    let view = uniforms.inv_proj * clip;
    return view.xyz / view.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = vec2<f32>(1.0 / uniforms.screen_width, 1.0 / uniforms.screen_height);

    // Sample depth and normal
    let depth = textureSample(depth_texture, tex_sampler, in.uv);
    if (depth >= 1.0) {
        // Sky/background - no occlusion
        return vec4<f32>(1.0);
    }

    let normal_sample = textureSample(normal_texture, tex_sampler, in.uv);

    // Skip pixels without valid geometry (alpha=0 means ground plane or other non-mesh surfaces)
    if (normal_sample.a < 0.5) {
        return vec4<f32>(1.0);
    }

    let normal = normalize(normal_sample.xyz * 2.0 - 1.0); // Decode from [0,1] to [-1,1]

    // Reconstruct view-space position
    let frag_pos = view_pos_from_depth(in.uv, depth);

    // Sample noise for random rotation (tile across screen)
    let noise_scale = vec2<f32>(uniforms.screen_width / 4.0, uniforms.screen_height / 4.0);
    let noise_uv = in.uv * noise_scale;
    let random_vec = textureSample(noise_texture, tex_sampler, noise_uv).xyz * 2.0 - 1.0;

    // Create TBN matrix to orient hemisphere along normal
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    // Accumulate occlusion
    var occlusion = 0.0;
    let sample_count = min(uniforms.sample_count, KERNEL_SIZE);

    // Depth discontinuity threshold - reject samples that cross geometric edges
    // Using a relative threshold based on radius to handle varying scene scales
    let depth_threshold = uniforms.radius * 3.0;

    for (var i = 0u; i < sample_count; i++) {
        // Get sample position in view space
        let sample_dir = tbn * kernel[i];
        let sample_pos = frag_pos + sample_dir * uniforms.radius;

        // Project sample to screen space
        let offset = uniforms.proj * vec4<f32>(sample_pos, 1.0);
        let offset_uv = (offset.xy / offset.w) * 0.5 + 0.5;

        // Sample depth at this position
        let sample_depth = textureSample(depth_texture, tex_sampler, vec2<f32>(offset_uv.x, 1.0 - offset_uv.y));
        let sample_view_pos = view_pos_from_depth(vec2<f32>(offset_uv.x, 1.0 - offset_uv.y), sample_depth);

        // Depth difference between center and sample
        let depth_diff = abs(frag_pos.z - sample_view_pos.z);

        // Skip samples that cross geometric edges (large depth discontinuity)
        // This prevents false occlusion at sharp corners and edges
        if (depth_diff > depth_threshold) {
            continue;
        }

        // Range check with quadratic falloff - more aggressive rejection of distant samples
        // This reduces artifacts where samples graze geometry edges
        let range_factor = depth_diff / uniforms.radius;
        let range_check = 1.0 - clamp(range_factor * range_factor, 0.0, 1.0);

        // Only count occlusion if sample is behind the expected position
        // Using angle-dependent bias: steeper angles need more bias
        let view_dir = normalize(-frag_pos);
        let angle_factor = 1.0 - abs(dot(normal, view_dir));
        let adaptive_bias = uniforms.bias * (1.0 + angle_factor * 2.0);

        if (sample_view_pos.z >= sample_pos.z + adaptive_bias) {
            occlusion += range_check;
        }
    }

    // Average and invert
    occlusion = 1.0 - (occlusion / f32(sample_count));

    // Apply intensity with softer curve to prevent over-darkening
    occlusion = pow(occlusion, uniforms.intensity * 0.8 + 0.2);

    return vec4<f32>(occlusion, occlusion, occlusion, 1.0);
}
