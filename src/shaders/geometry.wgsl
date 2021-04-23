struct VertexOutput {
    [[location(0)]] world_normal: vec3<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[block]]
struct Camera {
    proj: mat4x4<f32>;
    view: mat4x4<f32>;
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
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.world_normal = normalize((model_mats.normal * vec4<f32>(normal, 0.0)).xyz);
    out.position = cam_mats.proj * cam_mats.view * model_mats.model * vec4<f32>(position, 1.0);
    return out;
}

struct FragmentOutput {
    [[location(0)]] diffuse: vec4<f32>;
    [[location(1)]] material: vec4<f32>;
    [[location(2)]] normal: vec4<f32>;
};

[[block]]
struct Material {
    alpha: f32;
    k_s: f32;
    eta: f32;
    diffuse: vec3<f32>;
};

[[group(1), binding(1)]]
var material: Material;

[[stage(fragment)]]
fn fs_main(
    in: VertexOutput,
    [[builtin(front_facing)]] front_facing: bool,
    ) -> FragmentOutput {
    var out: FragmentOutput;
    out.normal = vec4<f32>(normalize(in.world_normal), 0.0);
    if (!front_facing) {
      out.normal = -out.normal;
    }
    out.material = vec4<f32>(material.alpha, material.k_s, material.eta, 1.0);
    out.diffuse = vec4<f32>(material.diffuse, 1.0);
    return out;
}

