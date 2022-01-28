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
var<uniform> light: Light;

[[group(1), binding(0)]]
var<uniform> model_mats: Model;

[[block]]
struct MatArray {
    mats: array<mat4x4<f32>>;
};

[[group(1), binding(2)]]
var<storage, read> joint_mats: MatArray;

fn add_mats(m0: mat4x4<f32>, m1: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(m0.x + m1.x, m0.y + m1.y, m0.z + m1.z, m0.w + m1.w);
}

fn mul_scalar_mat(scalar: f32, mat: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(mat.x * scalar, mat.y * scalar, mat.z * scalar, mat.w * scalar);
}

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
    [[location(2)]] weights: vec4<f32>,
    [[location(3)]] joints: vec4<u32>,
) -> [[builtin(position)]] vec4<f32> {
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

    return light.proj * light.view * model_mats.model * bones_mat * vec4<f32>(position, 1.0);
}

