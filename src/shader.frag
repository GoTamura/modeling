#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in vec3 v_position;
layout(location=2) in vec3 v_light_position;
layout(location=3) in vec3 v_view_position;
//layout(location=2) in mat3 v_tangent_matrix;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;
layout(set = 0, binding = 2) uniform texture2D t_normal;
layout(set = 0, binding = 3) uniform sampler s_normal;
layout(set = 0, binding = 4) uniform texture2D t_specular;
layout(set = 0, binding = 5) uniform sampler s_specular;

layout(set=2, binding=0)
uniform Light {
    vec3 light_position;
    vec3 light_color;
};

void main() {
    vec4 object_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    vec4 object_normal = texture(sampler2D(t_normal, s_normal), v_tex_coords);
    vec4 object_specular = texture(sampler2D(t_specular, s_specular), v_tex_coords);

    float ambient_strength = 0.1;
    vec3 ambient_color = light_color * ambient_strength;

    vec3 normal = normalize(object_normal.rgb * 2.0 - 1.0);
    vec3 light_dir = normalize(v_light_position - v_position);

    float diffuse_strength = max(dot(normal, light_dir), 0.);
    vec3 diffuse_color = light_color * diffuse_strength;

    vec3 view_dir = normalize(v_view_position - v_position);
    vec3 half_dir = normalize(view_dir + light_dir);
    float specular_strength = pow(max(dot(normal, half_dir), 0.0), 32) * smoothstep(-.01,.01, diffuse_strength);
    vec3 specular_color = object_specular.rgb * specular_strength * light_color;

    vec3 color = (ambient_color + diffuse_color) * object_color.xyz + specular_color;
    // vec3 color = ambient_color * object_color.xyz;
    // vec3 color = diffuse_color * object_color.xyz;
    // vec3 color = specular_color;
    // vec3 color = normal;
    // vec3 color =  vec3(v_tex_coords, 0.0);

    f_color = vec4(color, object_color.a);
}