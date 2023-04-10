cbuffer vertexBuffer : register(b0) {
	float4x4 ProjectionMatrix;
};

struct VS_INPUT {
	float2 pos : POSITION0;
	float2 uv  : TEXCOORD0;

	// Instance data
	float2 sprite_pos : POSITION1;
	float2 sprite_size : TEXCOORD1;
	float sprite_angle : TEXCOORD2;
	int sprite_tex_index : TEXCOORD3;
};

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv  : TEXCOORD0;
};

PS_INPUT main(VS_INPUT input) {
	float pcos = cos(input.sprite_angle - radians(90));
	float psin = sin(input.sprite_angle - radians(90));
	float2 rot_pos = float2(pcos * input.pos.x - psin * input.pos.y, psin * input.pos.x + pcos * input.pos.y);

	PS_INPUT output;
	output.pos = mul(ProjectionMatrix, float4(input.sprite_pos + rot_pos * input.sprite_size, 0.5f, 1.0f));
	output.uv = (float2(input.sprite_tex_index * 16.0f, 80.0f) + input.uv * 16.0f) / 256.0f;
	return output;
}