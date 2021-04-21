let PI: f32 = 3.14159265358979323846264;
let INV_SQRT_TWOPI: f32 = 0.3989422804;

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
var texture: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let dims = textureDimensions(texture); // THIS LINE
    var s: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    return vec4<f32>(s, 1.0);
}


