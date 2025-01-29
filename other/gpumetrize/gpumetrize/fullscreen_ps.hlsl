struct PS_INPUT {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD0;
};

Texture2D tex : register(t0);
SamplerState tex_sampler : register(s0);

float4 main(PS_INPUT input) : SV_TARGET {
    return tex.Sample(tex_sampler, input.uv);
}
