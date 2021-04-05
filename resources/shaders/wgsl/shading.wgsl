struct VertexOutput {
    [[location(0)]] tex_coords: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    const x = i32(vertex_index) / 2;
    const y = i32(vertex_index) & 1;
    const tc = vec2<f32>(
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

const PI: f32 = 3.14159265358979323846264;

[[block]]
struct Camera_Pos {
    inv_proj: mat4x4<f32>;
    inv_view: mat4x4<f32>;
    position: vec3<f32>;
};

[[group(0), binding(1)]]
var camera: Camera_Pos;

[[block]]
struct Light {
    power: vec3<f32>;
    position: vec3<f32>;
};

[[group(1), binding(0)]]
var light: Light;

[[group(2), binding(0)]]
var diffuse_texture: texture_2d<f32>;
[[group(2), binding(1)]]
var diffuse_sampler: sampler;

[[group(3), binding(0)]]
var normal_texture: texture_2d<f32>;
[[group(3), binding(1)]]
var normal_sampler: sampler;

[[group(4), binding(0)]]
var material_texture: texture_2d<f32>;
[[group(4), binding(1)]]
var material_sampler: sampler;

[[group(5), binding(0)]]
var depth_texture: texture_depth_2d;
[[group(5), binding(1)]]
var depth_sampler: sampler;

// The Fresnel reflection factor
//   i -- incoming direction
//   m -- microsurface normal
//   eta -- refractive index
fn fresnel(i: vec3<f32>, m: vec3<f32>, eta: f32) -> f32 {
    const c = abs(dot(i, m));
    const g = sqrt(eta * eta - 1.0 + c * c);

    const gmc = g - c;
    const gpc = g + c;
    const nom = c * (g + c) - 1.0;
    const denom = c * (g - c) + 1.0;
    return 0.5 * gmc * gmc / gpc / gpc * (1.0 + nom * nom / denom / denom);
}

// The one-sided Smith shadowing/masking function
//   v -- in or out vector
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
fn G1(v: vec3<f32>, m: vec3<f32>, n: vec3<f32>, alpha: f32) -> f32 {
    const vm = dot(v, m);
    const vn = dot(v, n);
    var result: f32 = 0.0;
    if (vm * vn > 0.0) {
        const cosThetaV = dot(n, v);
        const sinThetaV2 = 1.0 - cosThetaV * cosThetaV;
        const tanThetaV2 = sinThetaV2 / cosThetaV / cosThetaV;
        result = 2.0 / (1.0 + sqrt(1.0 + alpha * alpha * tanThetaV2));
    }
    return result;
}

// The GGX slope distribution function
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
fn D(m: vec3<f32>, n: vec3<f32>, alpha: f32) -> f32 {
    const mn = dot(m, n);
    var result: f32 = 0.0;
    if (mn > 0.0) {
        const cosThetaM = mn;
        const cosThetaM2 = cosThetaM * cosThetaM;
        const tanThetaM2 = (1.0 - cosThetaM2) / cosThetaM2;
        const cosThetaM4 =  cosThetaM * cosThetaM * cosThetaM * cosThetaM;
        const X = (alpha * alpha + tanThetaM2);
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
fn isotropicMicrofacet(i: vec3<f32>, o: vec3<f32>, n: vec3<f32>, eta: f32, alpha: f32) -> f32 {
    const odotn = dot(o, n);
    const m = normalize(i + o);

    const idotn = dot(i,n);
    if (idotn <= 0.0 || odotn <= 0.0) {
        return 0.0;
    }

    const idotm = dot(i, m);
    var F: f32 = 0.0;
    if (idotm > 0.0) {
        F = fresnel(i,m,eta);
    }
    const G = G1(i, m, n, alpha) * G1(o, m, n, alpha);
    return F * G * D(m, n, alpha) / (4.0 * idotn * odotn);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    const diffuse = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords).xyz;
    const normal = textureSample(normal_texture, normal_sampler, in.tex_coords).xyz;
    const material = textureSample(material_texture, material_sampler, in.tex_coords).xyz;
    const depth = textureSample(depth_texture, depth_sampler, in.tex_coords);
    const view_position_tmp = camera.inv_proj * vec4<f32>(in.tex_coords.x * 2.0 - 1.0, (1.0 - in.tex_coords.y) * 2.0 - 1.0, depth, 1.0);
    const view_position = view_position_tmp.xyz * (1.0 / view_position_tmp.w);
    const position = (camera.inv_view * vec4<f32>(view_position, 1.0)).xyz;
    var w_i: vec3<f32> = light.position - position;
    const w_o = normalize(camera.position - position);
    const r2 = dot(w_i, w_i);
    w_i = normalize(w_i);

    const spec = material.y * isotropicMicrofacet(w_i, w_o, normal, material.z, material.x);

    const brdf = diffuse * (1.0 / PI) + vec3<f32>(spec, spec, spec);

    const k_light = light.power * max(dot(normal, w_i), 0.0) * (1.0 / (4.0 * PI * r2));

    return vec4<f32>(k_light * brdf, 1.0);
}

