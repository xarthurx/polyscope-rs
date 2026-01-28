// SSAO bilateral blur shader
// Uses depth-aware weighting to preserve edges while smoothing noise

struct BlurUniforms {
    texel_size: vec2<f32>,
    blur_scale: f32,
    blur_sharpness: f32,
}

@group(0) @binding(0) var ssao_texture: texture_2d<f32>;
@group(0) @binding(1) var depth_texture: texture_depth_2d;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> uniforms: BlurUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center_ao = textureSample(ssao_texture, tex_sampler, in.uv).r;
    let center_depth = textureSample(depth_texture, tex_sampler, in.uv);

    // Skip background
    if (center_depth >= 1.0) {
        return vec4<f32>(1.0);
    }

    // 4x4 blur kernel (good balance of smoothing vs detail preservation)
    var result = 0.0;
    var total_weight = 0.0;

    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * uniforms.texel_size * uniforms.blur_scale;
            let sample_uv = in.uv + offset;

            // Bounds check
            if (sample_uv.x < 0.0 || sample_uv.x > 1.0 ||
                sample_uv.y < 0.0 || sample_uv.y > 1.0) {
                continue;
            }

            let sample_ao = textureSample(ssao_texture, tex_sampler, sample_uv).r;
            let sample_depth = textureSample(depth_texture, tex_sampler, sample_uv);

            // Spatial weight (simple box filter falloff)
            let dist = length(vec2<f32>(f32(x), f32(y)));
            let spatial_weight = 1.0 / (1.0 + dist * 0.5);

            // Depth weight - reduce contribution from pixels at different depths
            let depth_diff = abs(center_depth - sample_depth);
            let depth_weight = exp(-depth_diff * uniforms.blur_sharpness * 100.0);

            let weight = spatial_weight * depth_weight;
            result += sample_ao * weight;
            total_weight += weight;
        }
    }

    if (total_weight > 0.001) {
        result /= total_weight;
    } else {
        result = center_ao;
    }

    return vec4<f32>(result, result, result, 1.0);
}
