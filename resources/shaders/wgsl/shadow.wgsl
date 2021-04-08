[[block]]
struct Light {
    proj: mat4x4<f32>;
    view: mat4x4<f32>;
};

[[block]]
struct Model {
    model: mat4x4<f32>;
    normal: mat4x4<f32>;
};

[[group(0), binding(0)]]
var light: Light;

[[group(1), binding(0)]]
var model_mats: Model;

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
) -> [[builtin(position)]] vec4<f32> {
    return light.proj * light.view * model_mats.model * vec4<f32>(position, 1.0);
}

