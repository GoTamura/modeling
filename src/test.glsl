#ifndef TEST_GLSL
#define TEST_GLSL
float diffuse_s(vec3 normal, vec3 light_dir) {
    return max(dot(normal, light_dir), 0.);
}
#endif