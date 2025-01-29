#pragma once

#include <Windows.h>
#include <wrl/client.h>
#include <dxgi1_2.h>
#include <d3d11_1.h>
#include <vector>
#include <memory>
#include "consts.h"
#include "util.h"

using Microsoft::WRL::ComPtr;

struct SpriteInstance {
	float x;
	float y;
	float width;
	float height;
	float angle;
	float alpha;
	uint32_t atlas_id;
};

struct BasicVertex {
	float x;
	float y;
	float u;
	float v;
};

struct ConstantBuffer {
	float mvp[4][4];
	float fullscreen_uvs[4];
};

class Texture {
public:
	Texture() {};
	Texture(ID3D11Device1* device, const char* type, DXGI_FORMAT format, UINT width, UINT height, bool with_uav = false, bool filtering = false, const void* data = nullptr);

	ComPtr<ID3D11Texture2D> tex;
	ComPtr<ID3D11ShaderResourceView> srv;
	ComPtr<ID3D11SamplerState> sampler;
	ComPtr<ID3D11RenderTargetView> rtv;
	ComPtr<ID3D11UnorderedAccessView> uav;
};

class Geometrizer {
public:
	Geometrizer(HWND window, const char* image);
	~Geometrizer() {
		m_bExitRequested = true;
		if (m_hThread)
			WaitForSingleObject(m_hThread, INFINITE);
	}

	void SpawnThread() {
		m_hThread = CreateThread(NULL, 0, &ThreadProc, this, 0, 0);
		if (!m_hThread)
			printf("Failed to create geometrizer thread: 0x%lX", GetLastError());
	}
private:
	inline void UpdateConstantBuffer() {
		D3D11_MAPPED_SUBRESOURCE resource;
		DWORD res = m_pContext->Map(m_pConstantBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
		if (FAILED(res))
			Panic("Failed to map the constant buffer: 0x%X", res);
		memcpy(resource.pData, &m_pConstantBufferCPU, sizeof(ConstantBuffer));
		m_pContext->Unmap(m_pConstantBuffer.Get(), 0);
	}

	static DWORD __stdcall ThreadProc(void*);
	void ThreadProcInner();

	HWND m_hWnd = NULL;
	HANDLE m_hThread = NULL;
	bool m_bExitRequested = false;

	ComPtr<IDXGIAdapter> m_pAdapter;
	ComPtr<ID3D11Device1> m_pDevice;
	ComPtr<ID3D11DeviceContext> m_pContext;
	ComPtr<IDXGISwapChain1> m_pSwapChain;
	ComPtr<ID3D11RenderTargetView> m_pRTV;
	ComPtr<ID3D11BlendState> m_pBlendState;

	Texture m_TargetTex;
	Texture m_StencilTex;
	Texture m_CurAvgColorTex;
	Texture m_BestAvgColorTex;
	Texture m_CurCandidatesTex;
	Texture m_LocalBestCandidatesTex;
	Texture m_GlobalBestCandidateTex;
	Texture m_ErrorTex;
	Texture m_BO3Atlas;
	ComPtr<ID3D11Texture2D> m_pErrorStagingTex;
	ComPtr<ID3D11Texture2D> m_pAvgColorStagingTex;

	ComPtr<ID3D11Buffer> m_pConstantBuffer;
	ConstantBuffer m_pConstantBufferCPU;
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11Buffer> m_pBoundsBuffer;
	ComPtr<ID3D11ShaderResourceView> m_pBoundsSRV;

	ComPtr<ID3D11VertexShader> m_pMainVertexShader;
	ComPtr<ID3D11InputLayout> m_pMainVertexLayout;
	ComPtr<ID3D11GeometryShader> m_pMainGeometryShader;
	ComPtr<ID3D11PixelShader> m_pStencilPixelShader;
	ComPtr<ID3D11PixelShader> m_pSpritePixelShader;
	ComPtr<ID3D11ComputeShader> m_pAvgColorComputeShader;
	ComPtr<ID3D11ComputeShader> m_pErrorComputeShader;
	ComPtr<ID3D11VertexShader> m_pFullscreenVertexShader;
	ComPtr<ID3D11PixelShader> m_pFullscreenPixelShader;

	SpriteInstance m_BestInstances[GRID_SIZE * GRID_SIZE];
	float m_BestScores[GRID_SIZE * GRID_SIZE];
	SpriteInstance m_CurInstances[GRID_SIZE * GRID_SIZE];
	float m_CurBounds[GRID_SIZE * GRID_SIZE * 4];
	float m_CurScores[GRID_SIZE * GRID_SIZE];
	std::pair<size_t, float> m_PruneScores[GRID_SIZE * GRID_SIZE];

	std::vector<SpriteInstance> m_ChosenInstances;
	std::vector<uint32_t> m_ChosenColors;
};
