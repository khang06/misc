#pragma once

#include <Windows.h>
#include <wrl/client.h>
#include <dxgi1_2.h>
#include <d3d11_1.h>
#include <vector>
#include <memory>
#include "D3D11SpriteRenderers.h"

using Microsoft::WRL::ComPtr;

struct ConstantBuffer {
	float mvp[4][4];
};

enum class SpriteRendererType {
	CPUTransform,
	Instance,
	GeometryShader,
};

class D3D11Renderer {
public:
	D3D11Renderer(HWND window, SpriteRendererType renderer);

	void Draw(std::vector<SpriteInstance>& sprites);
private:
	HWND m_hWnd;

	std::unique_ptr<D3D11SpriteRenderer> m_pSpriteRenderer;
	
	ComPtr<IDXGIAdapter> m_pAdapter;
	ComPtr<ID3D11Device1> m_pDevice;
	ComPtr<ID3D11DeviceContext> m_pContext;
	ComPtr<IDXGISwapChain1> m_pSwapChain;
	ComPtr<ID3D11RenderTargetView> m_pRTV;
	ComPtr<ID3D11BlendState> m_pBlendState;
	ComPtr<ID3D11Texture2D> m_pEtamaTex;
	ComPtr<ID3D11ShaderResourceView> m_pEtamaSRV;
	ComPtr<ID3D11SamplerState> m_pEtamaSampler;
	ComPtr<ID3D11Buffer> m_pConstantBuffer;
};