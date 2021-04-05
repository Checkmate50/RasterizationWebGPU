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
fn vs_main(
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

const PI: f32 = 3.14159265358979323846264;

[[block]]
struct Light {
    power: vec3<f32>;
    position: vec3<f32>;
};

[[group(2), binding(0)]]
var light: Light;

[[block]]
struct Material {
    alpha: f32;
    k_s: f32;
    eta: f32;
    diffuse: vec3<f32>;
};

[[group(1), binding(1)]]
var material: Material;

[[block]]
struct Camera_Pos {
    position: vec3<f32>;
};

[[group(0), binding(1)]]
var camera: Camera_Pos;

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
    if (vm * vn > 0.0) {
        const cosThetaV = dot(n, v);
        const sinThetaV2 = 1.0 - cosThetaV * cosThetaV;
        const tanThetaV2 = sinThetaV2 / cosThetaV / cosThetaV;
        return 2.0 / (1.0 + sqrt(1.0 + alpha * alpha * tanThetaV2));
    } else {
        return 0.0;
    }
}

// The GGX slope distribution function
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
fn D(m: vec3<f32>, n: vec3<f32>, alpha: f32) -> f32 {
    const mn = dot(m, n);
    if (mn > 0.0) {
        const cosThetaM = mn;
        const cosThetaM2 = cosThetaM * cosThetaM;
        const tanThetaM2 = (1.0 - cosThetaM2) / cosThetaM2;
        const cosThetaM4 =  cosThetaM * cosThetaM * cosThetaM * cosThetaM;
        const X = (alpha * alpha + tanThetaM2);
        return alpha * alpha / (PI * cosThetaM4 * X * X);
    } else {
        return 0.0;
    }
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
    const normal = normalize(in.world_normal);
    var w_i: vec3<f32> = light.position - in.world_position;
    const w_o = normalize(camera.position - in.world_position);
    const r2 = dot(w_i, w_i);
    w_i = normalize(w_i);

    const spec = material.k_s * isotropicMicrofacet(w_i, w_o, normal, material.eta, material.alpha);

    const brdf = material.diffuse * (1.0 / PI) + vec3<f32>(spec, spec, spec);

    const k_light = light.power * max(dot(normal, w_i), 0.0) * (1.0 / (4.0 * PI * r2));

    return vec4<f32>(brdf * k_light, 1.0);
}

