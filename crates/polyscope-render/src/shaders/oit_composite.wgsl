// OIT Composite Shader
// Combines accumulated weighted transparent fragments with the opaque scene.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var accum_texture: texture_2d<f32>;
@group(0) @binding(1) var reveal_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

// Fullscreen triangle vertices
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let accum = textureSample(accum_texture, texture_sampler, in.uv);
    let reveal = textureSample(reveal_texture, texture_sampler, in.uv).r;

    // If reveal is 1.0, nothing was drawn (fully transparent)
    if (reveal >= 1.0) {
        discard;
    }

    // Weighted average color
    let avg_color = accum.rgb / max(accum.a, 0.0001);

    // Final alpha is 1 - reveal (how much of the background is occluded)
    let alpha = 1.0 - reveal;

    return vec4<f32>(avg_color, alpha);
}
