struct VertexOutput {
    [[location(0)]] tex_coords: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = i32(vertex_index) / 2;
    let y = i32(vertex_index) & 1;
    let tc = vec2<f32>(
        f32(x) * 2.0,
        f32(y) * 2.0
    );
    out.position = vec4<f32>(
        tc.x * 2.0 - 1.0,
        1.0 - tc.y * 2.0,
        0.0, 1.0
    );
    out.tex_coords = tc;
    return out;
}

[[group(0), binding(0)]]
var base_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var base_sampler: sampler;

[[group(1), binding(0)]]
var texture1: texture_2d<f32>;
[[group(1), binding(1)]]
var sampler1: sampler;

[[group(2), binding(0)]]
var texture2: texture_2d<f32>;
[[group(2), binding(1)]]
var sampler2: sampler;

[[group(3), binding(0)]]
var texture3: texture_2d<f32>;
[[group(3), binding(1)]]
var sampler3: sampler;

[[group(4), binding(0)]]
var texture4: texture_2d<f32>;
[[group(4), binding(1)]]
var sampler4: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let base = textureSample(base_texture, base_sampler, in.tex_coords).xyz;
    let blur1 = textureSample(texture1, sampler1, in.tex_coords).xyz;
    let blur2 = textureSample(texture2, sampler2, in.tex_coords).xyz;
    let blur3 = textureSample(texture3, sampler3, in.tex_coords).xyz;
    let blur4 = textureSample(texture4, sampler4, in.tex_coords).xyz;

    return vec4<f32>(0.8843 * base + 0.1 * blur1 + 0.012 * blur2 + 0.0027 * blur3 + 0.001 * blur4, 1.0);
}

