#pragma once

#include <wrl/client.h>
#include <d3d11_1.h>
#include <vector>
#include "D3D11Renderer.h"

using Microsoft::WRL::ComPtr;

struct SpriteInstance {
	float x;
	float y;
	float width;
	float height;
	float angle;
	int tex_index;
};

struct BasicVertex {
	float x;
	float y;
	float u;
	float v;
};

// Generic interface for various sprite rendering methods
class D3D11SpriteRenderer {
public:
	D3D11SpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context);
	virtual ~D3D11SpriteRenderer() = default;
	virtual void Draw(std::vector<SpriteInstance>& sprites) = 0;

protected:
	ComPtr<ID3D11Device> m_pDevice;
	ComPtr<ID3D11DeviceContext> m_pContext;
};

// Perform vertex transformations on the CPU and copy to a dynamic vertex buffer
class CPUTransformSpriteRenderer : public D3D11SpriteRenderer {
public:
	CPUTransformSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context);
	void Draw(std::vector<SpriteInstance>& sprites);

private:
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11Buffer> m_pIndexBuffer;
	ComPtr<ID3D11VertexShader> m_pVertexShader;
	ComPtr<ID3D11PixelShader> m_pPixelShader;
	ComPtr<ID3D11InputLayout> m_pInputLayout;
};

// Send transform data to the GPU and render sprites using instancing
class InstanceSpriteRenderer : public D3D11SpriteRenderer {
public:
	InstanceSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context);
	void Draw(std::vector<SpriteInstance>& sprites);

private:
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11Buffer> m_pIndexBuffer;
	ComPtr<ID3D11Buffer> m_pInstanceBuffer;
	ComPtr<ID3D11VertexShader> m_pVertexShader;
	ComPtr<ID3D11PixelShader> m_pPixelShader;
	ComPtr<ID3D11InputLayout> m_pInputLayout;
};

// Send transform data to the GPU and render sprites using geometry shaders
class GeometryShaderSpriteRenderer : public D3D11SpriteRenderer {
public:
	GeometryShaderSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context);
	void Draw(std::vector<SpriteInstance>& sprites);

private:
	ComPtr<ID3D11Buffer> m_pVertexBuffer;
	ComPtr<ID3D11VertexShader> m_pVertexShader;
	ComPtr<ID3D11PixelShader> m_pPixelShader;
	ComPtr<ID3D11GeometryShader> m_pGeometryShader;
	ComPtr<ID3D11InputLayout> m_pInputLayout;
};