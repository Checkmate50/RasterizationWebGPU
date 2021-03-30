// shader.vert
#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 v_position;
layout(location = 2) out mat4 view_mat;

layout(set = 0, binding = 0) uniform cam_mats {
    mat4 u_view;
    mat4 u_proj;
};

layout(set = 1, binding = 0) uniform model_mats {
    mat4 u_model;
    mat4 u_normal;
};

void main() {
    v_normal = normalize((u_normal * vec4(a_normal, 0.0)).xyz);
    vec4 world_position = u_model * vec4(a_position, 1.0);
    v_position = world_position.xyz; 
    view_mat = u_view;
    gl_Position = u_proj * u_view * world_position;
}

