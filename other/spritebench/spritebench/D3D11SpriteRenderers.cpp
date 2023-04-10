#define _USE_MATH_DEFINES
#include <math.h>
#include "Common.h"
#include "D3D11Renderer.h"
#include "BasicVertexShader.h"
#include "BasicPixelShader.h"
#include "InstanceVertexShader.h"
#include "GeometryVertexShader.h"
#include "GeometryShader.h"

D3D11SpriteRenderer::D3D11SpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context) {
	m_pDevice = device;
	m_pContext = context;
}

CPUTransformSpriteRenderer::CPUTransformSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context)
	: D3D11SpriteRenderer(device, context) {
	// Create a dynamic vertex buffer that'll hold the data to draw
	D3D11_BUFFER_DESC vertex_desc = {
		.ByteWidth = sizeof(BasicVertex) * MAX_SPRITES * 4,
		.Usage = D3D11_USAGE_DYNAMIC,
		.BindFlags = D3D11_BIND_VERTEX_BUFFER,
		.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};

	HRESULT res = m_pDevice->CreateBuffer(&vertex_desc, NULL, &m_pVertexBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (vertex) failed: 0x%X", res);

	// Create an index buffer
	D3D11_BUFFER_DESC index_desc = {
		.ByteWidth = MAX_SPRITES * 6 * 4,
		.Usage = D3D11_USAGE_IMMUTABLE,
		.BindFlags = D3D11_BIND_INDEX_BUFFER,
		.CPUAccessFlags = 0,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};

	std::vector<uint32_t> index_vec;
	index_vec.resize(MAX_SPRITES * 6);
	for (int i = 0; i < MAX_SPRITES; i++) {
		index_vec[i * 6] = i * 4;
		index_vec[i * 6 + 1] = i * 4 + 1;
		index_vec[i * 6 + 2] = i * 4 + 2;
		index_vec[i * 6 + 3] = i * 4;
		index_vec[i * 6 + 4] = i * 4 + 2;
		index_vec[i * 6 + 5] = i * 4 + 3;
	}
	D3D11_SUBRESOURCE_DATA index_init = {
		.pSysMem = index_vec.data(),
	};
	res = m_pDevice->CreateBuffer(&index_desc, &index_init, &m_pIndexBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (index) failed: 0x%X", res);

	// Load the vertex shader
	res = m_pDevice->CreateVertexShader(g_pBasicVertexShader, sizeof(g_pBasicVertexShader), nullptr, &m_pVertexShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);

	// Define the input layout
	D3D11_INPUT_ELEMENT_DESC input_layout[] = {
		{ "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 8, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	};
	res = m_pDevice->CreateInputLayout(input_layout, _countof(input_layout), g_pBasicVertexShader, sizeof(g_pBasicVertexShader), &m_pInputLayout);
	if (FAILED(res))
		Panic("CreateInputLayout failed: 0x%X", res);

	// Load the pixel shader
	res = m_pDevice->CreatePixelShader(g_pBasicPixelShader, sizeof(g_pBasicPixelShader), nullptr, &m_pPixelShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);
}

void CPUTransformSpriteRenderer::Draw(std::vector<SpriteInstance>& sprites) {
	// Generate the vertices
	auto count = min(sprites.size(), MAX_SPRITES);
	D3D11_MAPPED_SUBRESOURCE resource;
	HRESULT res = m_pContext->Map(m_pVertexBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
	if (FAILED(res))
		Panic("Failed to map the vertex buffer: 0x%X", res);
	auto vertices = (BasicVertex*)resource.pData;
	for (int i = 0; i < count; i++) {
		float s = sinf(sprites[i].angle - M_PI_2);
		float c = cosf(sprites[i].angle - M_PI_2);

		auto rotx = [=](float x, float y) {
			return x * c - y * s;
		};
		auto roty = [=](float x, float y) {
			return x * s + y * c;
		};

		float left = -sprites[i].width / 2.0f;
		float right = -left;
		float top = -sprites[i].height / 2.0f;
		float bottom = -top;

		float tex_left = ((float)sprites[i].tex_index * 16.0f) / 256.0f;
		float tex_right = tex_left + (sprites[i].width / 256.0f);
		float tex_top = 80.0f / 256.0f;
		float tex_bottom = tex_top + (sprites[i].height / 256.0f);

		vertices[i * 4 + 0] = {
			.x = sprites[i].x + rotx(left, top),
			.y = sprites[i].y + roty(left, top),
			.u = tex_left,
			.v = tex_top,
		};
		vertices[i * 4 + 1] = {
			.x = sprites[i].x + rotx(right, top),
			.y = sprites[i].y + roty(right, top),
			.u = tex_right,
			.v = tex_top,
		};
		vertices[i * 4 + 2] = {
			.x = sprites[i].x + rotx(right, bottom),
			.y = sprites[i].y + roty(right, bottom),
			.u = tex_right,
			.v = tex_bottom,
		};
		vertices[i * 4 + 3] = {
			.x = sprites[i].x + rotx(left, bottom),
			.y = sprites[i].y + roty(left, bottom),
			.u = tex_left,
			.v = tex_bottom,
		};
	}
	m_pContext->Unmap(m_pVertexBuffer.Get(), 0);

	// Set up render state
	UINT stride = sizeof(BasicVertex);
	UINT offset = 0;
	m_pContext->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);
	m_pContext->IASetIndexBuffer(m_pIndexBuffer.Get(), DXGI_FORMAT_R32_UINT, 0);
	m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
	m_pContext->IASetInputLayout(m_pInputLayout.Get());
	m_pContext->VSSetShader(m_pVertexShader.Get(), nullptr, 0);
	m_pContext->PSSetShader(m_pPixelShader.Get(), nullptr, 0);

	// Draw it
	m_pContext->DrawIndexed(count * 6, 0, 0);
}

InstanceSpriteRenderer::InstanceSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context)
	: D3D11SpriteRenderer(device, context) {
	// Create an immutable vertex buffer that holds a single quad
	D3D11_BUFFER_DESC vertex_desc = {
		.ByteWidth = sizeof(BasicVertex) * 4,
		.Usage = D3D11_USAGE_IMMUTABLE,
		.BindFlags = D3D11_BIND_VERTEX_BUFFER,
		.CPUAccessFlags = 0,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};
	BasicVertex vertices[] = {
		{ -0.5f, -0.5f, 0.0f, 0.0f },
		{  0.5f, -0.5f, 1.0f, 0.0f },
		{  0.5f,  0.5f, 1.0f, 1.0f },
		{ -0.5f,  0.5f, 0.0f, 1.0f },
	};
	D3D11_SUBRESOURCE_DATA vertex_data = {
		.pSysMem = vertices,
	};

	HRESULT res = m_pDevice->CreateBuffer(&vertex_desc, &vertex_data, &m_pVertexBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (vertex) failed: 0x%X", res);

	// Create an index buffer
	D3D11_BUFFER_DESC index_desc = {
		.ByteWidth = MAX_SPRITES * 6 * 4,
		.Usage = D3D11_USAGE_IMMUTABLE,
		.BindFlags = D3D11_BIND_INDEX_BUFFER,
		.CPUAccessFlags = 0,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};

	std::vector<uint32_t> index_vec;
	index_vec.resize(MAX_SPRITES * 6);
	for (int i = 0; i < MAX_SPRITES; i++) {
		index_vec[i * 6] = i * 4;
		index_vec[i * 6 + 1] = i * 4 + 1;
		index_vec[i * 6 + 2] = i * 4 + 2;
		index_vec[i * 6 + 3] = i * 4;
		index_vec[i * 6 + 4] = i * 4 + 2;
		index_vec[i * 6 + 5] = i * 4 + 3;
	}
	D3D11_SUBRESOURCE_DATA index_init = {
		.pSysMem = index_vec.data(),
	};
	res = m_pDevice->CreateBuffer(&index_desc, &index_init, &m_pIndexBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (index) failed: 0x%X", res);

	// Create the instance buffer
	D3D11_BUFFER_DESC instance_desc = {
		.ByteWidth = sizeof(SpriteInstance) * MAX_SPRITES,
		.Usage = D3D11_USAGE_DYNAMIC,
		.BindFlags = D3D11_BIND_VERTEX_BUFFER,
		.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};
	res = m_pDevice->CreateBuffer(&instance_desc, NULL, &m_pInstanceBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (instance) failed: 0x%X", res);

	// Load the vertex shader
	res = m_pDevice->CreateVertexShader(g_pInstanceVertexShader, sizeof(g_pInstanceVertexShader), nullptr, &m_pVertexShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);

	// Define the input layout
	D3D11_INPUT_ELEMENT_DESC input_layout[] = {
		{ "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "POSITION", 1, DXGI_FORMAT_R32G32_FLOAT, 1, 0, D3D11_INPUT_PER_INSTANCE_DATA, 1 },
		{ "TEXCOORD", 1, DXGI_FORMAT_R32G32_FLOAT, 1, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_INSTANCE_DATA, 1 },
		{ "TEXCOORD", 2, DXGI_FORMAT_R32_FLOAT, 1, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_INSTANCE_DATA, 1 },
		{ "TEXCOORD", 3, DXGI_FORMAT_R32_SINT, 1, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_INSTANCE_DATA, 1 },
	};
	res = m_pDevice->CreateInputLayout(input_layout, _countof(input_layout), g_pInstanceVertexShader, sizeof(g_pInstanceVertexShader), &m_pInputLayout);
	if (FAILED(res))
		Panic("CreateInputLayout failed: 0x%X", res);

	// Load the pixel shader
	res = m_pDevice->CreatePixelShader(g_pBasicPixelShader, sizeof(g_pBasicPixelShader), nullptr, &m_pPixelShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);
}

void InstanceSpriteRenderer::Draw(std::vector<SpriteInstance>& sprites) {
	// Update the instance data
	auto count = min(sprites.size(), MAX_SPRITES);
	D3D11_MAPPED_SUBRESOURCE resource;
	HRESULT res = m_pContext->Map(m_pInstanceBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
	if (FAILED(res))
		Panic("Failed to map the instance buffer: 0x%X", res);
	memcpy(resource.pData, sprites.data(), min(sprites.size(), MAX_SPRITES) * sizeof(SpriteInstance));
	m_pContext->Unmap(m_pInstanceBuffer.Get(), 0);

	// Set up render state
	ID3D11Buffer* buffers[] = { m_pVertexBuffer.Get(), m_pInstanceBuffer.Get() };
	UINT strides[] = { sizeof(BasicVertex), sizeof(SpriteInstance) };
	UINT offsets[] = { 0, 0 };
	m_pContext->IASetVertexBuffers(0, 2, buffers, strides, offsets);
	m_pContext->IASetIndexBuffer(m_pIndexBuffer.Get(), DXGI_FORMAT_R32_UINT, 0);
	m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
	m_pContext->IASetInputLayout(m_pInputLayout.Get());
	m_pContext->VSSetShader(m_pVertexShader.Get(), nullptr, 0);
	m_pContext->PSSetShader(m_pPixelShader.Get(), nullptr, 0);

	// Draw it
	m_pContext->DrawIndexedInstanced(6, count, 0, 0, 0);
}

GeometryShaderSpriteRenderer::GeometryShaderSpriteRenderer(ComPtr<ID3D11Device> device, ComPtr<ID3D11DeviceContext> context)
	: D3D11SpriteRenderer(device, context) {
	// Create a vertex buffer that holds the sprite data
	D3D11_BUFFER_DESC vertex_desc = {
		.ByteWidth = sizeof(SpriteInstance) * MAX_SPRITES,
		.Usage = D3D11_USAGE_DYNAMIC,
		.BindFlags = D3D11_BIND_VERTEX_BUFFER,
		.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
		.MiscFlags = 0,
		.StructureByteStride = 0,
	};

	HRESULT res = m_pDevice->CreateBuffer(&vertex_desc, NULL, &m_pVertexBuffer);
	if (FAILED(res))
		Panic("CreateBuffer (vertex) failed: 0x%X", res);

	// Load the vertex shader
	res = m_pDevice->CreateVertexShader(g_pGeometryVertexShader, sizeof(g_pGeometryVertexShader), nullptr, &m_pVertexShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);

	// Define the input layout
	D3D11_INPUT_ELEMENT_DESC input_layout[] = {
		{ "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "TEXCOORD", 1, DXGI_FORMAT_R32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
		{ "TEXCOORD", 2, DXGI_FORMAT_R32_SINT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
	};
	res = m_pDevice->CreateInputLayout(input_layout, _countof(input_layout), g_pGeometryVertexShader, sizeof(g_pGeometryVertexShader), &m_pInputLayout);
	if (FAILED(res))
		Panic("CreateInputLayout failed: 0x%X", res);

	// Load the pixel shader
	res = m_pDevice->CreatePixelShader(g_pBasicPixelShader, sizeof(g_pBasicPixelShader), nullptr, &m_pPixelShader);
	if (FAILED(res))
		Panic("CreateVertexShader failed: 0x%X", res);

	// Load the geometry shader
	res = m_pDevice->CreateGeometryShader(g_pGeometryShader, sizeof(g_pGeometryShader), nullptr, &m_pGeometryShader);
	if (FAILED(res))
		Panic("CreateGeometryShader failed: 0x%X", res);
}

void GeometryShaderSpriteRenderer::Draw(std::vector<SpriteInstance>& sprites) {
	// Update the vertex data
	auto count = min(sprites.size(), MAX_SPRITES);
	D3D11_MAPPED_SUBRESOURCE resource;
	HRESULT res = m_pContext->Map(m_pVertexBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
	if (FAILED(res))
		Panic("Failed to map the vertex buffer: 0x%X", res);
	memcpy(resource.pData, sprites.data(), min(sprites.size(), MAX_SPRITES) * sizeof(SpriteInstance));
	m_pContext->Unmap(m_pVertexBuffer.Get(), 0);

	// Set up render state
	UINT stride = sizeof(SpriteInstance);
	UINT offset = 0;
	m_pContext->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);
	m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_POINTLIST);
	m_pContext->IASetInputLayout(m_pInputLayout.Get());
	m_pContext->VSSetShader(m_pVertexShader.Get(), nullptr, 0);
	m_pContext->PSSetShader(m_pPixelShader.Get(), nullptr, 0);
	m_pContext->GSSetShader(m_pGeometryShader.Get(), nullptr, 0);

	// Draw it
	m_pContext->Draw(count, 0);
}