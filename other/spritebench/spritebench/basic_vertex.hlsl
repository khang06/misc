cbuffer vertexBuffer : register(b0)  {
    float4x4 ProjectionMatrix;
};

struct VS_INPUT {
	float2 pos : POSITION;
	float2 uv  : TEXCOORD0;
};

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv  : TEXCOORD0;
};

PS_INPUT main(VS_INPUT input) {
	PS_INPUT output;
	output.pos = mul(ProjectionMatrix, float4(input.pos, 0.5f, 1.0f));
	output.uv = input.uv;
	return output;
}