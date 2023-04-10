cbuffer vertexBuffer : register(b0) {
	float4x4 ProjectionMatrix;
};

struct GS_INPUT {
	float2 pos : POSITION0;
	float2 size : TEXCOORD0;
	float angle : TEXCOORD1;
	int tex_index : TEXCOORD2;
};

struct PS_INPUT {
	float4 pos : SV_POSITION;
	float2 uv  : TEXCOORD0;
};

[maxvertexcount(4)]
void main(point GS_INPUT input[1], inout TriangleStream<PS_INPUT> output) {
	PS_INPUT vertices[4];
	float pcos = cos(input[0].angle);
	float psin = sin(input[0].angle);

	float2 base_pos[4] = {
		float2(-0.5f, -0.5f),
		float2( 0.5f, -0.5f),
		float2( 0.5f,  0.5f),
		float2(-0.5f,  0.5f),
	};

	float2 base_uv[4] = {
		float2(0.0, 0.0),
		float2(1.0, 0.0),
		float2(1.0, 1.0),
		float2(0.0, 1.0),
	};

	[unroll(4)]
	for (int i = 0; i < 4; i++) {
		float2 rot_pos = float2(pcos * base_pos[i].x - psin * base_pos[i].y, psin * base_pos[i].x + pcos * base_pos[i].y);
		vertices[i].pos = mul(ProjectionMatrix, float4(input[0].pos + rot_pos * input[0].size, 0.5f, 1.0f));
		vertices[i].uv = (float2(input[0].tex_index * 16.0f, 80.0f) + base_uv[i] * 16.0f) / 256.0f;
	}

	/*
	output.Append(vertices[0]);
	output.Append(vertices[1]);
	output.Append(vertices[2]);
	output.RestartStrip();

	output.Append(vertices[0]);
	output.Append(vertices[2]);
	output.Append(vertices[3]);
	*/

	output.Append(vertices[0]);
	output.Append(vertices[1]);
	output.Append(vertices[3]);
	output.Append(vertices[2]);
}