#include "consts.h"

cbuffer vertexBuffer : register(b0) {
	float4x4 mvp;
    float4 fullscreen_uvs;
};

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv : TEXCOORD0;
    uint idx : TEXCOORD1;
    float alpha : TEXCOORD2;
};

Texture2D tex : register(t0);
Texture2D atlas : register(t1);
SamplerState sam : register(s0);
SamplerState atlas_sam : register(s1);

float4 main(PS_INPUT input) : SV_TARGET {
    const uint cell_x = input.idx % GRID_SIZE;
    const uint cell_y = input.idx / GRID_SIZE;
    const float2 offset = float2(cell_x * (TARGET_WIDTH >> 1), cell_y * (TARGET_HEIGHT >> 1));
    
    if (input.pos.x < offset.x || input.pos.y < offset.y ||
        input.pos.x >= offset.x + (TARGET_WIDTH >> 1) || input.pos.y >= offset.y + (TARGET_HEIGHT >> 1) ||
        atlas.Sample(atlas_sam, input.uv).r < 0.5f)
        discard;
    
    return tex.Sample(sam, input.pos.xy / float2(TARGET_WIDTH >> 1, TARGET_HEIGHT >> 1));
}
