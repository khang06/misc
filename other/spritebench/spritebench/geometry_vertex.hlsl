struct GS_INPUT {
	float2 pos : POSITION0;
	float2 size : TEXCOORD0;
	float angle : TEXCOORD1;
	int tex_index : TEXCOORD2;
};

GS_INPUT main(GS_INPUT input) {
	input.angle -= radians(90);
	return input;
}