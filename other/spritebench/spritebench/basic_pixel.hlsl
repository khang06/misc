struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv  : TEXCOORD0;
};

Texture2D tex : register(t0);
SamplerState sam : register(s0);

float4 main(PS_INPUT input) : SV_TARGET {
	return tex.Sample(sam, input.uv);
}