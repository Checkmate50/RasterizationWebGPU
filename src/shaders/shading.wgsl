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

let PI: f32 = 3.14159265358979323846264;

[[block]]
struct Light {
    proj: mat4x4<f32>;
    view: mat4x4<f32>;
    power: vec3<f32>;
    position: vec3<f32>;
};

[[group(0), binding(0)]]
var<uniform> light: Light;

[[block]]
struct Camera_Pos {
    inv_proj: mat4x4<f32>;
    inv_view: mat4x4<f32>;
    position: vec3<f32>;
};

[[group(1), binding(1)]]
var<uniform> camera: Camera_Pos;

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

[[group(5), binding(0)]]
var material_texture: texture_2d<f32>;
[[group(5), binding(1)]]
var material_sampler: sampler;

[[group(6), binding(0)]]
var light_depth_texture: texture_depth_2d;
[[group(6), binding(1)]]
var light_depth_sampler: sampler_comparison;

// The Fresnel reflection factor
//   i -- incoming direction
//   m -- microsurface normal
//   eta -- refractive index
fn fresnel(i: vec3<f32>, m: vec3<f32>, eta: f32) -> f32 {
    let c = abs(dot(i, m));
    let g = sqrt(eta * eta - 1.0 + c * c);

    let gmc = g - c;
    let gpc = g + c;
    let nom = c * (g + c) - 1.0;
    let denom = c * (g - c) + 1.0;
    return 0.5 * gmc * gmc / gpc / gpc * (1.0 + nom * nom / denom / denom);
}

// The one-sided Smith shadowing/masking function
//   v -- in or out vector
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
fn G1(v: vec3<f32>, m: vec3<f32>, n: vec3<f32>, alpha: f32) -> f32 {
    let vm = dot(v, m);
    let vn = dot(v, n);
    var result: f32 = 0.0;
    if (vm * vn > 0.0) {
        let cosThetaV = dot(n, v);
        let sinThetaV2 = 1.0 - cosThetaV * cosThetaV;
        let tanThetaV2 = sinThetaV2 / cosThetaV / cosThetaV;
        result = 2.0 / (1.0 + sqrt(1.0 + alpha * alpha * tanThetaV2));
    }
    return result;
}

// The GGX slope distribution function
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
fn D(m: vec3<f32>, n: vec3<f32>, alpha: f32) -> f32 {
    let mn = dot(m, n);
    var result: f32 = 0.0;
    if (mn > 0.0) {
        let cosThetaM = mn;
        let cosThetaM2 = cosThetaM * cosThetaM;
        let tanThetaM2 = (1.0 - cosThetaM2) / cosThetaM2;
        let cosThetaM4 =  cosThetaM * cosThetaM * cosThetaM * cosThetaM;
        let X = (alpha * alpha + tanThetaM2);
        result = alpha * alpha / (PI * cosThetaM4 * X * X);
    }
    return result;
}

// Evalutate the Microfacet BRDF (GGX variant) for the paramters:
//   i -- incoming direction (unit vector, pointing away from surface)
//   o -- outgoing direction (unit vector, pointing away from surface)
//   n -- outward pointing surface normal vector
//   eta -- refractive index
//   alpha -- surface roughness
// return: scalar BRDF value
fn isotropic_microfacet(i: vec3<f32>, o: vec3<f32>, n: vec3<f32>, eta: f32, alpha: f32) -> f32 {
    let odotn = dot(o, n);
    let m = normalize(i + o);

    let idotn = dot(i,n);
    if (idotn <= 0.0 || odotn <= 0.0) {
        return 0.0;
    }

    let idotm = dot(i, m);
    var F: f32 = 0.0;
    if (idotm > 0.0) {
        F = fresnel(i,m,eta);
    }
    let G = G1(i, m, n, alpha) * G1(o, m, n, alpha);
    return F * G * D(m, n, alpha) / (4.0 * idotn * odotn);
}

fn get_world_position(tex_coords: vec2<f32>) -> vec3<f32> {
    let depth = textureSample(depth_texture, depth_sampler, tex_coords);
    let coords_ndc = vec2<f32>(tex_coords.x * 2.0 - 1.0, (1.0 - tex_coords.y) * 2.0 - 1.0);
    let view_position_tmp = camera.inv_proj * vec4<f32>(coords_ndc, depth, 1.0);
    let view_position = view_position_tmp.xyz * (1.0 / view_position_tmp.w);
    return (camera.inv_view * vec4<f32>(view_position, 1.0)).xyz;
}

fn not_occluded(position: vec3<f32>) -> f32 {
    // compute position in light space
    let light_pos = light.proj * light.view * vec4<f32>(position, 1.0);

    // vulkan's coordinate system is in [1, -1], [-1, 1], [0, 1] so we account for that
    let flip = vec3<f32>(0.5, -0.5, 1.0);
    let shadow_coords = light_pos.xyz * flip * (1.0 / light_pos.w) + vec3<f32>(0.5, 0.5, 0.0);

    // get texture with comparison sampler
    return textureSampleCompare(light_depth_texture, light_depth_sampler, shadow_coords.xy, shadow_coords.z);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let diffuse = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords).xyz;
    let normal = textureSample(normal_texture, normal_sampler, in.tex_coords).xyz;
    let material = textureSample(material_texture, material_sampler, in.tex_coords).xyz;
    let position = get_world_position(in.tex_coords);
    let shadow = not_occluded(position);
    var result: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if (shadow == 1.0) {
      var w_i: vec3<f32> = light.position - position;
      let w_o = normalize(camera.position - position);
      let r2 = dot(w_i, w_i);
      w_i = normalize(w_i);

      let spec = material.y * isotropic_microfacet(w_i, w_o, normal, material.z, material.x);

      let brdf = diffuse * (1.0 / PI) + vec3<f32>(spec, spec, spec);

      let k_light = light.power * max(dot(normal, w_i), 0.0) * (1.0 / (4.0 * PI * r2));

      result = vec4<f32>(brdf * k_light, 1.0);
    }
    return result;
}

