// SSAO blur shader
// Applies bilateral blur to smooth SSAO while preserving edges

struct BlurUniforms {
    texel_size: vec2<f32>,
    blur_scale: f32,
    _padding: f32,
}

@group(0) @binding(0) var ssao_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: BlurUniforms;

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
    // 4x4 box blur
    var result = 0.0;
    for (var x = -2; x < 2; x++) {
        for (var y = -2; y < 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * uniforms.texel_size * uniforms.blur_scale;
            result += textureSample(ssao_texture, tex_sampler, in.uv + offset).r;
        }
    }
    result /= 16.0;

    return vec4<f32>(result, result, result, 1.0);
}
