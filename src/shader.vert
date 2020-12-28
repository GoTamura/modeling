#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_tex_coords;

layout(location=0) out vec3 v_tex_coords;

layout(location=5) in mat4 model_matrix;

layout(set=1, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    v_tex_coords = a_tex_coords;
    gl_Position = u_view_proj * model_matrix * vec4(a_position, 1.);
}