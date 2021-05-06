struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] world_normal: vec3<f32>;
};

[[block]]
struct Camera {
    proj: mat4x4<f32>;
    view: mat4x4<f32>;
};

[[group(0), binding(0)]]
var cam_mats: Camera;

[[block]]
struct Model {
    model: mat4x4<f32>;
    normal: mat4x4<f32>;
};

[[group(1), binding(0)]]
var model_mats: Model;

[[block]]
struct Joints {
    mats: array<mat4x4<f32>>;
};

[[group(1), binding(2)]]
var joint_mats: [[access(read)]] Joints;

fn add_mats(m0: mat4x4<f32>, m1: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(m0.x + m1.x, m0.y + m1.y, m0.z + m1.z, m0.w + m1.w);
}

fn mul_scalar_mat(scalar: f32, mat: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(mat.x * scalar, mat.y * scalar, mat.z * scalar, mat.w * scalar);
}

fn mat4tomat3(m: mat4x4<f32>) -> mat3x3<f32> {
    return mat3x3<f32>(m.x.xyz, m.y.xyz, m.z.xyz);
}

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
    [[location(2)]] weights: vec4<f32>,
    [[location(3)]] joints: vec4<u32>,
) -> VertexOutput {
    let bones_mat = add_mats(
      add_mats(
          mul_scalar_mat(weights.x, joint_mats.mats[joints.x]),
          mul_scalar_mat(weights.y, joint_mats.mats[joints.y]),
      ),
      add_mats(
          mul_scalar_mat(weights.z, joint_mats.mats[joints.z]),
          mul_scalar_mat(weights.w, joint_mats.mats[joints.w])
      )
    );

    var out: VertexOutput;
    out.world_normal = normalize((model_mats.normal * vec4<f32>(mat4tomat3(bones_mat) * normal, 0.0)).xyz);
    out.position = cam_mats.proj * cam_mats.view * model_mats.model * bones_mat * vec4<f32>(position, 1.0);
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

