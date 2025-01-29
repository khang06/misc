cbuffer vertexBuffer : register(b0) {
	float4x4 mvp;
    float4 fullscreen_uvs;
};

struct PS_INPUT {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD0;
};

PS_INPUT main(uint id : SV_VertexID) {
    PS_INPUT result;

    // https://gist.github.com/rorydriscoll/1495603/56b12e19c62828bc2ecf28e6a90b65108879f461
    const uint2 orig_uv = float2((id << 1) & 2, id & 2);
    result.uv = float2(orig_uv.x ? fullscreen_uvs.b : fullscreen_uvs.r, orig_uv.y ? fullscreen_uvs.a : fullscreen_uvs.g) * 2.0f;
    result.position = float4(orig_uv * float2(2, -2) + float2(-1, 1), 0, 1);

    return result;
}
