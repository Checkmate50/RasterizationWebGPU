#version 450

const float PI = 3.14159265358979323846264;

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_position;
layout(location = 2) in mat4 view_mat;
layout(location = 0) out vec4 f_color;

layout(set = 2, binding = 0) uniform light_uniforms {
  vec3 light_power;
  vec3 light_position;
};

layout(set = 1, binding = 1) uniform material {
  float alpha;
  float k_s;
  float eta;
  vec3 diffuse;
};

layout(set = 0, binding = 0) uniform cam_pos {
    vec3 camera_position;
};

// The Fresnel reflection factor
//   i -- incoming direction
//   m -- microsurface normal
//   eta -- refractive index
float fresnel(vec3 i, vec3 m, float eta) {
  float c = abs(dot(i,m));
  float g = sqrt(eta*eta - 1.0 + c*c);

  float gmc = g-c;
  float gpc = g+c;
  float nom = c*(g+c)-1.0;
  float denom = c*(g-c)+1.0;
  return 0.5*gmc*gmc/gpc/gpc*(1.0 + nom*nom/denom/denom);
}

// The one-sided Smith shadowing/masking function
//   v -- in or out vector
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
float G1(vec3 v, vec3 m, vec3 n, float alpha) {
  float vm = dot(v,m);
  float vn = dot(v,n);
  if (vm*vn > 0.0) {
    float cosThetaV = dot(n,v);
    float sinThetaV2 = 1.0 - cosThetaV*cosThetaV;
    float tanThetaV2 = sinThetaV2 / cosThetaV / cosThetaV;
    return 2.0 / (1.0 + sqrt(1.0 + alpha*alpha*tanThetaV2));
  } else {
    return 0;
  }
}

// The GGX slope distribution function
//   m -- microsurface normal
//   n -- (macro) surface normal
//   alpha -- surface roughness
float D(vec3 m, vec3 n, float alpha) {
  float mn = dot(m,n);
  if (mn > 0.0) {
    float cosThetaM = mn;
    float cosThetaM2 = cosThetaM*cosThetaM;
    float tanThetaM2 = (1.0 - cosThetaM2) / cosThetaM2;
    float cosThetaM4 =  cosThetaM*cosThetaM*cosThetaM*cosThetaM;
    float X = (alpha*alpha + tanThetaM2);
    return alpha*alpha / (PI * cosThetaM4 * X * X);
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
float isotropicMicrofacet(vec3 i, vec3 o, vec3 n, float eta, float alpha) {
  float odotn = dot(o,n);
  vec3 m = normalize(i + o);

  float idotn = dot(i,n);
  if (idotn <= 0.0 || odotn <= 0.0)
      return 0;

  float idotm = dot(i,m);
  float F = (idotm > 0.0) ? fresnel(i,m,eta) : 0.0;
  float G = G1(i,m,n,alpha) * G1(o,m,n,alpha);
  return F * G * D(m,n,alpha) / (4.0*idotn*odotn);
}

void main() {
  vec3 normal = normalize(v_normal);
  // todo: figure out why this works, optimize
  vec3 cam_pos = (inverse(view_mat) * vec4(camera_position, 1.0)).xyz;
  vec3 w_i = light_position - v_position;
  vec3 w_o = normalize(cam_pos - v_position);
  float r2 = dot(w_i, w_i);
  w_i = normalize(w_i);

  float spec = k_s * isotropicMicrofacet(w_i, w_o, normal, eta, alpha);

  vec3 brdf = diffuse / PI + vec3(spec);

  vec3 k_light = light_power * max(dot(normal, w_i), 0.0) / (4 * PI * r2);

  f_color = vec4(brdf * k_light, 1.0);
}

