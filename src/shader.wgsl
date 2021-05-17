struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(5)]] model_matrix1: vec4<f32>;
    [[location(6)]] model_matrix2: vec4<f32>;
    [[location(7)]] model_matrix3: vec4<f32>;
    [[location(8)]] model_matrix4: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] w_position: vec3<f32>;
};


fn inverse(model: mat4x4<f32>) -> mat4x4<f32> {
    var m: mat4x4<f32> = model;
    //let a00 = m[1];// let a01 = m[0][1]; let a02 = m[0][2]; let a04 = m[0][3];
    return m;
    //let a10 = m[1][0]; let a11 = m[1][1]; let a12 = m[1][2]; let a13 = m[1][3];
    //let a20 = m[2][0]; let a21 = m[2][1]; let a22 = m[2][2]; let a23 = m[2][3];
    //let a30 = m[3][0]; let a31 = m[3][1]; let a32 = m[3][2]; let a33 = m[3][3];

    //let b00 = a00 * a11 - a01 * a10;
    //let b01 = a00 * a12 - a02 * a10;
    //let b02 = a00 * a13 - a03 * a10;
    //let b03 = a01 * a12 - a02 * a11;
    //let b04 = a01 * a13 - a03 * a11;
    //let b05 = a02 * a13 - a03 * a12;
    //let b06 = a20 * a31 - a21 * a30;
    //let b07 = a20 * a32 - a22 * a30;
    //let b08 = a20 * a33 - a23 * a30;
    //let b09 = a21 * a32 - a22 * a31;
    //let b10 = a21 * a33 - a23 * a31;
    //let b11 = a22 * a33 - a23 * a32;

    //let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;

    //return mat4x4<f32>(
    //    a11 * b11 - a12 * b10 + a13 * b09,
    //    a02 * b10 - a01 * b11 - a03 * b09,
    //    a31 * b05 - a32 * b04 + a33 * b03,
    //    a22 * b04 - a21 * b05 - a23 * b03,
    //    a12 * b08 - a10 * b11 - a13 * b07,
    //    a00 * b11 - a02 * b08 + a03 * b07,
    //    a32 * b02 - a30 * b05 - a33 * b01,
    //    a20 * b05 - a22 * b02 + a23 * b01,
    //    a10 * b10 - a11 * b08 + a13 * b06,
    //    a01 * b08 - a00 * b10 - a03 * b06,
    //    a30 * b04 - a31 * b02 + a33 * b00,
    //    a21 * b02 - a20 * b04 - a23 * b00,
    //    a11 * b07 - a10 * b09 - a12 * b06,
    //    a00 * b09 - a01 * b07 + a02 * b06,
    //    a31 * b01 - a30 * b03 - a32 * b00,
    //    a20 * b03 - a21 * b01 + a22 * b00) / det;
}

[[block]]
struct Uniforms {
    view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    let model_matrix: mat4x4<f32> = mat4x4<f32>(in.model_matrix1, in.model_matrix2, in.model_matrix3, in.model_matrix4);
    let normal_matrix = transpose(inverse(model_matrix));
    var out: VertexOutput;

    out.tex_coords = in.tex_coords;
    out.normal = (normal_matrix * vec4<f32>(in.normal, 1.0)).xyz;

    let model_space = model_matrix * vec4<f32>(in.normal, 1.0);
    out.w_position = model_space.xyz;
    out.position = uniforms.view_proj * model_matrix * vec4<f32>(in.position, 1.0);
    return out;
}

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[block]]
struct Light {
    position: vec3<f32>;
    color: vec3<f32>;
};
[[group(2), binding(0)]]
var<uniform> light: Light;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let normal = normalize(in.normal);
    let light_dir = normalize(light.position - in.w_position);

    let diffuse_strength = max(dot(normal, light_dir), 0.);
    let diffuse_color = light.color * diffuse_strength;

    let color = (ambient_color + diffuse_color) * object_color.xyz;
    let f_color = vec4<f32>(color, object_color.a);
    return f_color; 
}