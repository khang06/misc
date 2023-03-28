#include <Windows.h>
#include "$bc7e_ispc_avx2.h"

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

extern "C" __declspec(dllexport) void bc7e_init() {
    ispc::bc7e_compress_block_init();
}

extern "C" __declspec(dllexport) void bc7e_init_params(int level, bool perceptual, ispc::bc7e_compress_block_params* pack_params) {
	memset(pack_params, 0, sizeof(ispc::bc7e_compress_block_params));
	switch (level)
	{
	case 0:
		ispc::bc7e_compress_block_params_init_ultrafast(pack_params, perceptual);
		break;
	case 1:
		ispc::bc7e_compress_block_params_init_veryfast(pack_params, perceptual);
		break;
	case 2:
		ispc::bc7e_compress_block_params_init_fast(pack_params, perceptual);
		break;
	case 3:
		ispc::bc7e_compress_block_params_init_basic(pack_params, perceptual);
		break;
	case 4:
		ispc::bc7e_compress_block_params_init_slow(pack_params, perceptual);
		break;
	case 5:
		ispc::bc7e_compress_block_params_init_veryslow(pack_params, perceptual);
		break;
	case 6:
	default:
		ispc::bc7e_compress_block_params_init_slowest(pack_params, perceptual);
		break;
	}
}

extern "C" __declspec(dllexport) void bc7e_compress(uint32_t num_blocks, uint64_t* pBlocks, const uint32_t* pPixelsRGBA, const struct ispc::bc7e_compress_block_params* pComp_params) {
	ispc::bc7e_compress_blocks(num_blocks, pBlocks, pPixelsRGBA, pComp_params);
}