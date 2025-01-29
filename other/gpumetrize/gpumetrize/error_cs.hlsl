#include "consts.h"

// Based on https://therealmjp.github.io/posts/average-luminance-compute-shader/

// Each thread samples 8x8 texels on the full resolution sprite texture
static const uint group_size = TARGET_WIDTH >> 3;
static const uint total_threads = group_size * group_size;

Texture2D<float4> target_tex : register(t0);
Texture2D<float4> candidate_tex : register(t1);
Texture2D<float4> best_tex : register(t2);
Buffer<float4> bounds : register(t3);
RWTexture2D<float> output_tex : register(u0);
groupshared float3 shared_mem[total_threads];

[numthreads(group_size, group_size, 1)]
void main(uint3 group_id : SV_GroupID, uint3 group_thread_id : SV_GroupThreadID) {
    const float3 color_scale = float3(3.0f, 4.0f, 2.0f);
    
    const uint thread_idx = group_thread_id.y * group_size + group_thread_id.x;
    const float4 bbox = (bounds[group_id.y * GRID_SIZE + group_id.x] + float4(group_id.xy, group_id.xy))
        * float4(TARGET_WIDTH, TARGET_HEIGHT, TARGET_WIDTH, TARGET_HEIGHT);
    
    // Sum up 8x8 texels per thread
    float3 local_sum = float3(0.0f, 0.0f, 0.0f);
    const uint2 local_pos = group_id.xy * group_size * 8;
    [unroll]
    for (uint y = group_thread_id.y; y < TARGET_HEIGHT; y += group_size) {
        [unroll]
        for (uint x = group_thread_id.x; x < TARGET_WIDTH; x += group_size) {
            const uint2 pos = local_pos + uint2(x, y);
            if (pos.x >= bbox.r && pos.y >= bbox.g && pos.x <= bbox.b && pos.y <= bbox.a) {
                const float3 target = target_tex[pos % uint2(TARGET_WIDTH, TARGET_HEIGHT)].rgb;
                const float3 cur_diff = (target - candidate_tex[pos].rgb) * color_scale;
                const float3 best_diff = (target - best_tex[pos % uint2(TARGET_WIDTH, TARGET_HEIGHT)].rgb) * color_scale;
                local_sum += best_diff * best_diff - cur_diff * cur_diff;
            }
        }
    }
    shared_mem[thread_idx] = local_sum;
    GroupMemoryBarrierWithGroupSync();
    
    // Use a parallel reduction to get the total error
    [unroll(total_threads)]
    for (uint s = total_threads >> 1; s > 0; s >>= 1) {
        if (thread_idx < s)
            shared_mem[thread_idx] += shared_mem[thread_idx + s];
        GroupMemoryBarrierWithGroupSync();
    }
    
    if (thread_idx == 0)
        output_tex[group_id.xy] = shared_mem[0].r + shared_mem[0].g + shared_mem[0].b;
}
