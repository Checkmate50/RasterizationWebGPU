let PI: f32 = 3.14159265358979323846264;

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
struct Light {
    radiance: vec3<f32>;
    range: f32;
};

[[group(0), binding(0)]]
var<uniform> light: Light;

[[block]]
struct Camera {
    proj: mat4x4<f32>;
    view: mat4x4<f32>;
};

[[block]]
struct InvCamera {
    inv_proj: mat4x4<f32>;
    inv_view: mat4x4<f32>;
    position: vec3<f32>;
};

[[group(1), binding(0)]]
var<uniform> camera: Camera;

[[group(1), binding(1)]]
var<uniform> inv_camera: InvCamera;

[[group(2), binding(0)]]
var diffuse_texture: texture_2d<f32>;
[[group(2), binding(1)]]
var diffuse_sampler: sampler;

[[group(3), binding(0)]]
var normal_texture: texture_2d<f32>;
[[group(3), binding(1)]]
var normal_sampler: sampler;

[[group(4), binding(0)]]
var depth_texture: texture_depth_2d;
[[group(4), binding(1)]]
var depth_sampler: sampler;

[[block]]
struct Sky {
    A: vec3<f32>;
    B: vec3<f32>;
    C: vec3<f32>;
    D: vec3<f32>;
    E: vec3<f32>;
    zenith: vec3<f32>;
    theta_sun: f32;
};

let sky_scale: f32 = 0.06;
let ground_radiance: vec3<f32> = vec3<f32>(0.5, 0.5, 0.5);
let solar_disc_radiance: vec3<f32> = vec3<f32>(10000.0, 10000.0, 10000.0);
let sun_angular_radius: f32 = 0.00872664625;
let phi_sun: f32 = PI;

[[group(5), binding(0)]]
var<uniform> sky: Sky;

fn random(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co.xy,vec2<f32>(12.9898,78.233))) * 43758.5453);
}

fn make_tbn(normal: vec3<f32>) -> mat3x3<f32> {
    let normal_abs = abs(normal);
    let min_norm_dir = min(min(normal_abs.x, normal_abs.y), normal_abs.z);
    var nonparallel: vec3<f32> = vec3<f32>(min_norm_dir, 0.0, 0.0);
    if (min_norm_dir == normal_abs.y) {
      nonparallel = vec3<f32>(0.0, min_norm_dir, 0.0);
    }
    if (min_norm_dir == normal_abs.z) {
      nonparallel = vec3<f32>(0.0, 0.0, min_norm_dir);
    }
    if (min_norm_dir == 0.0) {
      nonparallel = vec3<f32>(0.5, 0.5, 0.5);
    }
    let tangent = normalize(cross(normal, nonparallel));
    let bitangent = normalize(cross(normal, tangent));

    return mat3x3<f32>(tangent, bitangent, normal);
}

fn get_world_position(tex_coords: vec2<f32>) -> vec3<f32> {
    let depth = textureSample(depth_texture, depth_sampler, tex_coords);
    let coords_ndc = vec2<f32>(tex_coords.x * 2.0 - 1.0, (1.0 - tex_coords.y) * 2.0 - 1.0);
    let view_position_tmp = inv_camera.inv_proj * vec4<f32>(coords_ndc, depth, 1.0);
    let view_position = view_position_tmp.xyz * (1.0 / view_position_tmp.w);
    return (inv_camera.inv_view * vec4<f32>(view_position, 1.0)).xyz;
}

fn square_to_uniform_disk_concentric(sample: vec2<f32>) -> vec2<f32> {
    let r1 = 2.0 * sample.x - 1.0;
    let r2 = 2.0 * sample.y - 1.0;

    var phi: f32 = 0.0;
    var r: f32 = 0.0;
    if (r1 == 0.0 && r2 == 0.0) {
        r = 0.0;
        phi = 0.0;
    } elseif (r1*r1 > r2*r2) {
        r = r1;
        phi = (PI/4.0) * (r2/r1);
    } else {
        r = r2;
        phi = (PI/2.0) - (r1/r2) * (PI/4.0);
    }

    let sin_phi = sin(phi);
    let cos_phi = cos(phi);

    return vec2<f32>(r * cos_phi, r * sin_phi);
}

fn square_to_cosine_hemisphere(sample: vec2<f32>) -> vec3<f32> {
    let p = square_to_uniform_disk_concentric(sample);
    var z: f32 = sqrt(max(1.0 - p.x * p.x - p.y * p.y, 0.0));

    if (z == 0.0) {
        z = 0.00001;
    }

    return vec3<f32>(p.x, p.y, z);
}

fn is_occluded(world_position: vec3<f32>, direction: vec3<f32>, range: f32) -> f32 {
    // compute position in proj space
    let view_pos = camera.view * vec4<f32>(world_position + (direction * range), 1.0);
    let view_pos_frag = camera.view * vec4<f32>(world_position, 1.0);
    let proj_pos = camera.proj * view_pos;

    // vulkan's coordinate system is in [1, -1], [-1, 1], [0, 1] so we account for that
    let flip = vec3<f32>(0.5, -0.5, 1.0);
    let shadow_coords = proj_pos.xyz * flip * (1.0 / proj_pos.w) + vec3<f32>(0.5, 0.5, 0.0);

    // get texture with comparison sampler
    let depth = textureSample(depth_texture, depth_sampler, shadow_coords.xy);

    // honestly I'm not 100% sure what I'm doing here
    // but through various degeneracy it appears to work
    let pos = vec4<f32>(shadow_coords.x * 2.0 - 1.0, (1.0 - shadow_coords.y) * 2.0 - 1.0, depth, 1.0);
    let pos_vs = inv_camera.inv_proj * pos;
    let recon_pos = pos_vs.xyz * (1.0 / pos_vs.w);
    var range_check: f32 = 0.0;
    if (abs(recon_pos.z - view_pos.z) < range * 4.0) {
      range_check = 1.0;
    }
    var result: f32 = 1.0;
    if (recon_pos.z + 0.00001 <= view_pos.z) {
      result = 0.0;
    }
    return result * range_check;
}

fn perez(theta: f32, gamma: f32) -> vec3<f32> {
    return (vec3<f32>(1.0, 1.0, 1.0) + sky.A * exp(sky.B * (1.0 / cos(theta)))) * (vec3<f32>(1.0, 1.0, 1.0) + sky.C * exp(sky.D * gamma) + sky.E * pow(cos(gamma), 2.0));
}

fn sun_radiance(dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = vec3<f32>(sin(sky.theta_sun) * cos(phi_sun), cos(sky.theta_sun), sin(sky.theta_sun) * sin(phi_sun));
    if (dot(dir, sun_dir) > cos(sun_angular_radius)) {
        return solar_disc_radiance;
    }
    return vec3<f32>(0.0, 0.0, 0.0);
}

let XYZ2RGB: mat3x3<f32> = mat3x3<f32>(
   vec3<f32>(3.2404542, -0.969266, 0.0556434),
   vec3<f32>(-1.5371385, 1.8760108, -0.2040259),
   vec3<f32>(-0.4985314, 0.041556, 1.0572252)
);

fn sky_radiance(dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = vec3<f32>(sin(sky.theta_sun) * cos(phi_sun), cos(sky.theta_sun), sin(sky.theta_sun) * sin(phi_sun));
    let gamma = acos(min(1.0, dot(dir, sun_dir)));
    if (dir.y > 0.0) {
      let theta = acos(dir.y);
      let Yxy = sky.zenith * perez(theta, gamma) / perez(0.0, sky.theta_sun);
      return XYZ2RGB * vec3<f32>(Yxy[1] * (Yxy[0]/Yxy[2]), Yxy[0], (1.0 - Yxy[1] - Yxy[2])*(Yxy[0]/Yxy[2])) * sky_scale;
    }
    return ground_radiance;
}

fn sunsky_radiance(dir: vec3<f32>) -> vec3<f32> {
    return sun_radiance(dir) + sky_radiance(dir);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let diffuse = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords).xyz;
    let normal = textureSample(normal_texture, normal_sampler, in.tex_coords).xyz;
    let position = get_world_position(in.tex_coords);
    if (normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0) {
        let result = max(sunsky_radiance(normalize(position - inv_camera.position)), vec3<f32>(0.0, 0.0, 0.0));
        return vec4<f32>(result, 1.0);
    }
    let TBN = make_tbn(normal);
    var i: f32 = 0.0;
    let max_iter = 32.0;
    var total_shadowed: f32 = max_iter;
    loop {
      if (i >= max_iter) { break; }
      let rand_vec = vec2<f32>(random(in.tex_coords + vec2<f32>(0.0, i)), random(in.tex_coords + vec2<f32>(i, 0.0)));
      let r = sqrt(random(in.tex_coords + vec2<f32>(i, i)));
      let uniform_dir = square_to_cosine_hemisphere(rand_vec);
      let point = TBN * r * uniform_dir;
      let shadow = is_occluded(position, point, light.range);
      total_shadowed = total_shadowed - shadow;
      i = i + 1.0;
    }
    let shadow_percent = total_shadowed / max_iter;
    return vec4<f32>(light.radiance * diffuse * shadow_percent, 1.0);
}

