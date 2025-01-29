struct GS_INPUT {
	float2 pos : POSITION0;
	float2 size : TEXCOORD0;
	float angle : TEXCOORD1;
    float alpha : TEXCOORD2;
    uint atlas_id : TEXCOORD3;
};

GS_INPUT main(GS_INPUT input) {
	return input;
}
