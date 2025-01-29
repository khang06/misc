#include "consts.h"

cbuffer vertexBuffer : register(b0) {
	float4x4 mvp;
    float4 fullscreen_uvs;
};

struct GS_INPUT {
	float2 pos  : POSITION0;
	float2 size : TEXCOORD0;
	float angle : TEXCOORD1;
    float alpha : TEXCOORD2;
    uint atlas_id : TEXCOORD3;
};

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv : TEXCOORD0;
    uint idx : TEXCOORD1;
    float alpha : TEXCOORD2;
};

[maxvertexcount(4)]
void main(point GS_INPUT input[1], inout TriangleStream<PS_INPUT> output, uint idx : SV_PrimitiveID) {
	PS_INPUT vertices[4];
    const float pcos = cos(input[0].angle);
    const float psin = sin(input[0].angle);
    const float2 offset = float2(idx % GRID_SIZE, idx / GRID_SIZE);
    const float2 scale = float2(TARGET_WIDTH, TARGET_HEIGHT);

	const float2 base_pos[4] = {
		float2(-0.5f, -0.5f) * input[0].size,
		float2( 0.5f, -0.5f) * input[0].size,
		float2( 0.5f,  0.5f) * input[0].size,
		float2(-0.5f,  0.5f) * input[0].size,
	};

    const float2 atlas_uv = float2(input[0].atlas_id % 8 / 8.0f, input[0].atlas_id / 8 / 8.0f);
	const float2 base_uv[4] = {
        atlas_uv,
		atlas_uv + float2(1.0 / 8.0f, 0.0f),
		atlas_uv + float2(1.0f / 8.0f, 1.0f / 8.0f),
		atlas_uv + float2(0.0f, 1.0f / 8.0f),
    };

	[unroll(4)]
	for (int i = 0; i < 4; i++) {
		float2 rot_pos = float2(pcos * base_pos[i].x - psin * base_pos[i].y, psin * base_pos[i].x + pcos * base_pos[i].y);
        vertices[i].pos = mul(mvp, float4(((input[0].pos + rot_pos) / float2(ASPECT_RATIO, 1.0f) + offset) * scale, 0.5f, 1.0f));
        vertices[i].uv = base_uv[i];
        vertices[i].idx = idx;
        vertices[i].alpha = input[0].alpha;
    }

	output.Append(vertices[0]);
	output.Append(vertices[1]);
	output.Append(vertices[3]);
	output.Append(vertices[2]);
}