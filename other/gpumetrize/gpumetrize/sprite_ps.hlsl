#include "consts.h"

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv : TEXCOORD0;
    uint idx : TEXCOORD1;
    float alpha : TEXCOORD2;
};

Texture2D color : register(t0);
Texture2D atlas : register(t1);
SamplerState atlas_sam : register(s0);

float4 main(PS_INPUT input) : SV_TARGET {
    const uint cell_x = input.idx % GRID_SIZE;
    const uint cell_y = input.idx / GRID_SIZE;
    const float2 offset = float2(cell_x * TARGET_WIDTH, cell_y * TARGET_HEIGHT);
    
    if (input.pos.x < offset.x || input.pos.y < offset.y ||
        input.pos.x >= offset.x + TARGET_WIDTH || input.pos.y >= offset.y + TARGET_HEIGHT ||
        atlas.Sample(atlas_sam, input.uv).r < 0.5f)
        discard;
    
    return float4(color.Load(int3(cell_x, cell_y, 0)).xyz, input.alpha);
}
