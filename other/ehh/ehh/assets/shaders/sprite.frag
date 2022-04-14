#version 430 core
#extension GL_ARB_gpu_shader_int64 : require

out vec4 color;

flat in uint64_t frag_tex_handle;
in vec3 frag_uvl;
in vec4 frag_color;

void main() {
    color = texture(sampler2DArray(frag_tex_handle), frag_uvl) * frag_color;
}