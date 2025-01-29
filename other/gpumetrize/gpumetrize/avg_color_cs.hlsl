#include "consts.h"

// Based on https://therealmjp.github.io/posts/average-luminance-compute-shader/

// Each thread samples 4x4 texels on the half resolution stencil texture
static const uint group_size = TARGET_WIDTH >> 3;
static const uint total_threads = group_size * group_size;

Texture2D<float4> input_tex : register(t0);
Buffer<float4> input_bounds : register(t1);
RWTexture2D<float4> output_tex : register(u0);
groupshared float4 shared_mem[total_threads];

[numthreads(group_size, group_size, 1)]
void main(uint3 group_id : SV_GroupID, uint3 group_thread_id : SV_GroupThreadID) {
    const uint thread_idx = group_thread_id.y * group_size + group_thread_id.x;
    
    // Sum up 4x4 texels per thread
    float4 local_sum = float4(0.0f, 0.0f, 0.0f, 0.0f);
    const uint2 local_pos = group_id.xy * group_size * 4;
    const float4 bbox = (input_bounds[group_id.y * group_size + group_id.x] + float4(group_id.xy, group_id.xy))
        * float4(TARGET_WIDTH >> 1, TARGET_HEIGHT >> 1, TARGET_WIDTH >> 1, TARGET_HEIGHT >> 1);
    [unroll]
    for (uint y = group_thread_id.y; y < (TARGET_HEIGHT >> 1); y += group_size) {
        [unroll]
        for (uint x = group_thread_id.x; x < (TARGET_WIDTH >> 1); x += group_size) {
            uint2 pos = local_pos + uint2(x, y);
            if (pos.x >= bbox.r && pos.y >= bbox.g && pos.x <= bbox.b && pos.y <= bbox.a)
                local_sum += float4(pow(input_tex[pos].rgb, 1.0f / 2.2f), input_tex[pos].a);
        }
    }
    shared_mem[thread_idx] = local_sum;
    GroupMemoryBarrierWithGroupSync();
    
    // Use a parallel reduction to get the average color
    [unroll(total_threads)]
    for (uint s = total_threads >> 1; s > 0; s >>= 1) {
        if (thread_idx < s)
            shared_mem[thread_idx] += shared_mem[thread_idx + s];
        GroupMemoryBarrierWithGroupSync();
    }
    
    if (thread_idx == 0)
        output_tex[group_id.xy] = (shared_mem[0].a == 0.0f) ? 0.0f : float4(pow(shared_mem[0].rgb / shared_mem[0].a, 2.2f), 1.0f);
}
