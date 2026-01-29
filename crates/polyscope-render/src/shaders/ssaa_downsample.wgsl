// SSAA (Supersampling Anti-Aliasing) downsample shader
// Downsamples a high-resolution texture to screen resolution using box filtering

struct DownsampleUniforms {
    ssaa_factor: u32,
    _padding: vec3<u32>,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: DownsampleUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let factor = f32(uniforms.ssaa_factor);
    let dims = vec2<f32>(textureDimensions(input_texture));
    let texel_size = 1.0 / dims;

    // Box filter: average all samples in the SSAA grid
    // For 2x SSAA: 4 samples, for 4x SSAA: 16 samples
    var color = vec4<f32>(0.0);
    let sample_count = uniforms.ssaa_factor * uniforms.ssaa_factor;

    // Calculate the center of the first sample in the grid
    // We want to sample at the centers of the high-res pixels that map to this output pixel
    let base_uv = in.uv;

    for (var y = 0u; y < uniforms.ssaa_factor; y++) {
        for (var x = 0u; x < uniforms.ssaa_factor; x++) {
            // Offset within the SSAA grid (0.5 centers the sample in each sub-pixel)
            let offset = (vec2<f32>(f32(x), f32(y)) + 0.5) / factor - 0.5;
            let sample_uv = base_uv + offset * texel_size * factor;
            color += textureSample(input_texture, tex_sampler, sample_uv);
        }
    }

    return color / f32(sample_count);
}
