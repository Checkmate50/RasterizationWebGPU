struct VertexOutput {
    [[location(0)]] world_normal: vec3<f32>;
    [[location(1)]] world_position: vec3<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[block]]
struct Camera {
    view: mat4x4<f32>;
    proj: mat4x4<f32>;
};

[[block]]
struct Model {
    model: mat4x4<f32>;
    normal: mat4x4<f32>;
};

[[group(0), binding(0)]]
var cam_mats: Camera;

[[group(1), binding(0)]]
var model_mats: Model;

[[stage(vertex)]]
fn main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    const world_position = model_mats.model * vec4<f32>(position, 1.0);
    out.world_normal = normalize((model_mats.normal * vec4<f32>(normal, 0.0)).xyz);
    out.world_position = world_position.xyz;
    out.position = cam_mats.proj * cam_mats.view * world_position;
    return out;
}
