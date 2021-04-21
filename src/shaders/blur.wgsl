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

[[block]]
struct Window {
    size: vec2<f32>;
};

[[group(0), binding(0)]]
var texture: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler: sampler;

[[group(0), binding(2)]]
var window: Window;

[[block]]
struct Blur {
    dir: vec2<f32>;
    stdev: f32;
    radius: i32;
};

[[group(1), binding(0)]]
var blur: Blur;

fn gaussianWeight(r: f32) -> f32 {
    return exp(-r*r/(2.0*blur.stdev*blur.stdev)) * INV_SQRT_TWOPI / blur.stdev;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let inc = vec2<f32>(1.0, 1.0) / vec2<f32>(window.size);
    var s: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var i: i32 = -blur.radius;
    loop {
        if (i > blur.radius) { break; }
        let w = gaussianWeight(f32(i));
        s = s + w * textureSample(texture, sampler, in.tex_coords + inc * f32(i) * blur.dir).xyz;
        i = i + 1;
    }
    return vec4<f32>(s, 1.0);
}
