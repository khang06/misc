#version 430 core
#extension GL_ARB_gpu_shader_int64 : require

layout (location = 0) in vec2 base;

layout (location = 1) in uint64_t tex_handle;
layout (location = 2) in float layer;
layout (location = 3) in uint color;
layout (location = 4) in vec2 position;
layout (location = 5) in vec2 size;
layout (location = 6) in float rot;
layout (location = 7) in float uv[4];

flat out uint64_t frag_tex_handle;
out vec3 frag_uvl;
out vec4 frag_color;

uniform mat4 proj;

// uv: [left, bottom, right, top]
// vertex: [bottom left, bottom right, top right, top left]
const int vertex_to_uv_idx[8] = int[8](
    0, 1,
    2, 1,
    2, 3,
    0, 3
);

void main() {
    float pcos = cos(rot);
    float psin = sin(rot);
    vec2 rot_base = vec2(pcos * base.x - psin * base.y, psin * base.x + pcos * base.y);

    gl_Position = proj * vec4(rot_base * size + position, 0.5, 1.0);
    frag_tex_handle = tex_handle;
    frag_uvl = vec3(uv[vertex_to_uv_idx[gl_VertexID * 2]], uv[vertex_to_uv_idx[gl_VertexID * 2 + 1]], layer);
    frag_color = vec4(float(color & 0xFF) / 255.0f, float((color >> 8) & 0xFF) / 255.0f, float((color >> 16) & 0xFF) / 255.0f, float((color >> 24) & 0xFF) / 255.0f);
}