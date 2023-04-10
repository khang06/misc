#include "Common.h"
#include "etama3.h"
#include "D3D11Renderer.h"

D3D11Renderer::D3D11Renderer(HWND hwnd, SpriteRendererType renderer) {
	m_hWnd = hwnd;

	// Create device
    D3D_FEATURE_LEVEL feature_levels[] = {
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
    };
    ComPtr<ID3D11Device> device;
    D3D_FEATURE_LEVEL feature_level = D3D_FEATURE_LEVEL_11_1;
    UINT flags = 0;
#ifdef _DEBUG
    flags = D3D11_CREATE_DEVICE_DEBUG;
#endif
    HRESULT res = D3D11CreateDevice(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, flags, feature_levels, _countof(feature_levels),
        D3D11_SDK_VERSION, &device, &feature_level, &m_pContext);
    if (FAILED(res))
        Panic("D3D11CreateDevice failed: 0x%X", res);
    res = device.As(&m_pDevice);
    if (FAILED(res))
        Panic("Casting to ID3D11Device1 failed: 0x%X", res);

    // Obtain DXGI factory from device (since we used nullptr for pAdapter above)
    ComPtr<IDXGIFactory2> dxgi_factory = nullptr;
    {
        ComPtr<IDXGIDevice> dxgi_device = nullptr;
        res = m_pDevice.As(&dxgi_device);
        if (FAILED(res))
            Panic("Casting to IDXGIDevice failed: 0x%X", res);
        ComPtr<IDXGIAdapter> dxgi_adapter;
        res = dxgi_device->GetAdapter(&dxgi_adapter);
        if (FAILED(res))
            Panic("GetAdapter failed : 0x%X", res);
        res = dxgi_adapter->GetParent(IID_PPV_ARGS(&dxgi_factory));
        if (FAILED(res))
            Panic("GetParent failed : 0x%X", res);
    }

    // Create swapchain
    DXGI_SWAP_CHAIN_DESC1 sd = {
        .Width = WINDOW_WIDTH,
        .Height = WINDOW_HEIGHT,
        .Format = DXGI_FORMAT_R8G8B8A8_UNORM,
        .SampleDesc = {
            .Count = 1,
            .Quality = 0,
        },
        .BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT,
        .BufferCount = 1,
        // Should be using DXGI_SWAP_EFFECT_FLIP_DISCARD, but this is just example code
        .SwapEffect = DXGI_SWAP_EFFECT_DISCARD,
    };
    res = dxgi_factory->CreateSwapChainForHwnd(m_pDevice.Get(), m_hWnd, &sd, nullptr, nullptr, &m_pSwapChain);
    if (FAILED(res))
        Panic("CreateSwapChainForHwnd failed: 0x%X", res);

    // Disable ALT+ENTER to fullscreen
    dxgi_factory->MakeWindowAssociation(m_hWnd, DXGI_MWA_NO_ALT_ENTER);

    // Set up backbuffer render target view
    ComPtr<ID3D11Texture2D> backbuffer_tex;
    res = m_pSwapChain->GetBuffer(0, IID_PPV_ARGS(&backbuffer_tex));
    if (FAILED(res))
        Panic("GetBuffer failed: 0x%X", res);
    res = m_pDevice->CreateRenderTargetView(backbuffer_tex.Get(), NULL, &m_pRTV);
    if (FAILED(res))
        Panic("CreateRenderTargetView failed: 0x%X", res);
    m_pContext->OMSetRenderTargets(1, m_pRTV.GetAddressOf(), nullptr);

    // Set up viewport
    D3D11_VIEWPORT viewport = {
        .TopLeftX = 0.0f,
        .TopLeftY = 0.0f,
        .Width = WINDOW_WIDTH,
        .Height = WINDOW_HEIGHT,
        .MinDepth = 0.0f,
        .MaxDepth = 1.0f,
    };
    m_pContext->RSSetViewports(1, &viewport);

    // Create and upload projection matrix
    D3D11_BUFFER_DESC proj_desc = {
        .ByteWidth = sizeof(ConstantBuffer),
        .Usage = D3D11_USAGE_IMMUTABLE,
        .BindFlags = D3D11_BIND_CONSTANT_BUFFER,
        .CPUAccessFlags = 0,
        .MiscFlags = 0,
    };
    float L = 0.0f;
    float R = WINDOW_WIDTH;
    float T = 0.0f;
    float B = WINDOW_HEIGHT;
    auto proj = ConstantBuffer {
        .mvp = {
            { 2.0f/(R-L),   0.0f,           0.0f,       0.0f },
            { 0.0f,         2.0f/(T-B),     0.0f,       0.0f },
            { 0.0f,         0.0f,           0.5f,       0.0f },
            { (R+L)/(L-R),  (T+B)/(B-T),    0.5f,       1.0f },
        }
    };
    D3D11_SUBRESOURCE_DATA proj_data = {
        .pSysMem = &proj
    };
    res = m_pDevice->CreateBuffer(&proj_desc, &proj_data, &m_pConstantBuffer);
    if (FAILED(res))
        Panic("CreateBuffer failed: 0x%X", res);

    // Create alpha blend state
    D3D11_BLEND_DESC blend_desc = {
        .RenderTarget = {{
            .BlendEnable = TRUE,
            .SrcBlend = D3D11_BLEND_SRC_ALPHA,
            .DestBlend = D3D11_BLEND_INV_SRC_ALPHA,
            .BlendOp = D3D11_BLEND_OP_ADD,
            .SrcBlendAlpha = D3D11_BLEND_ONE,
            .DestBlendAlpha = D3D11_BLEND_ZERO,
            .BlendOpAlpha = D3D11_BLEND_OP_ADD,
            .RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL,
        }}
    };
    res = m_pDevice->CreateBlendState(&blend_desc, &m_pBlendState);
    if (FAILED(res))
        Panic("CreateBuffer failed: 0x%X", res);

    // Upload bullet texture atlas
    D3D11_TEXTURE2D_DESC tex_desc = {
        .Width = 256,
        .Height = 256,
        .MipLevels = 1,
        .ArraySize = 1,
        .Format = DXGI_FORMAT_R8G8B8A8_UNORM,
        .SampleDesc = {
            .Count = 1,
            .Quality = 0,
        },
        .Usage = D3D11_USAGE_IMMUTABLE,
        .BindFlags = D3D11_BIND_SHADER_RESOURCE,
        .CPUAccessFlags = 0,
        .MiscFlags = 0,
    };
    D3D11_SUBRESOURCE_DATA tex_data = {
        // WHY ISN'T #embed A THING YET WTF
        .pSysMem = g_pEtama3,
        .SysMemPitch = 256 * 4,
        .SysMemSlicePitch = sizeof(g_pEtama3),
    };
    res = m_pDevice->CreateTexture2D(&tex_desc, &tex_data, &m_pEtamaTex);
    if (FAILED(res))
        Panic("CreateTexture2D failed: 0x%X", res);

    // Create bullet texture shader resource view
    D3D11_SHADER_RESOURCE_VIEW_DESC tex_srv = {
        .Format = DXGI_FORMAT_R8G8B8A8_UNORM,
        .ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D,
        .Texture2D = {
            .MostDetailedMip = 0,
            .MipLevels = 1,
        }
    };
    res = m_pDevice->CreateShaderResourceView(m_pEtamaTex.Get(), &tex_srv, &m_pEtamaSRV);
    if (FAILED(res))
        Panic("CreateTexture2D failed: 0x%X", res);

    // Create bullet texture sampler
    D3D11_SAMPLER_DESC sampler_desc = {
        .Filter = D3D11_FILTER_MIN_MAG_MIP_POINT,
        .AddressU = D3D11_TEXTURE_ADDRESS_WRAP,
        .AddressV = D3D11_TEXTURE_ADDRESS_WRAP,
        .AddressW = D3D11_TEXTURE_ADDRESS_WRAP,
        .MipLODBias = 0.0f,
        .ComparisonFunc = D3D11_COMPARISON_ALWAYS,
        .MinLOD = 0.0f,
        .MaxLOD = 0.0f,
    };
    m_pDevice->CreateSamplerState(&sampler_desc, &m_pEtamaSampler);
    if (FAILED(res))
        Panic("CreateSamplerState failed: 0x%X", res);

    // Create sprite renderer
    switch (renderer) {
        case SpriteRendererType::CPUTransform:
            m_pSpriteRenderer = std::unique_ptr<D3D11SpriteRenderer>(new CPUTransformSpriteRenderer(m_pDevice, m_pContext));
            break;
        case SpriteRendererType::Instance:
            m_pSpriteRenderer = std::unique_ptr<D3D11SpriteRenderer>(new InstanceSpriteRenderer(m_pDevice, m_pContext));
            break;
        case SpriteRendererType::GeometryShader:
            m_pSpriteRenderer = std::unique_ptr<D3D11SpriteRenderer>(new GeometryShaderSpriteRenderer(m_pDevice, m_pContext));
            break;
    }
}

void D3D11Renderer::Draw(std::vector<SpriteInstance>& sprites) {
    float clear_color[4] = { 0.0f, 0.0f, 0.0f, 0.0f };
    m_pContext->ClearRenderTargetView(m_pRTV.Get(), clear_color);
    m_pContext->VSSetConstantBuffers(0, 1, m_pConstantBuffer.GetAddressOf());
    m_pContext->GSSetConstantBuffers(0, 1, m_pConstantBuffer.GetAddressOf());
    m_pContext->PSSetShaderResources(0, 1, m_pEtamaSRV.GetAddressOf());
    m_pContext->PSSetSamplers(0, 1, m_pEtamaSampler.GetAddressOf());
    m_pContext->OMSetBlendState(m_pBlendState.Get(), 0, 0xFFFFFFFF);

    if (m_pSpriteRenderer)
        m_pSpriteRenderer->Draw(sprites);

    m_pSwapChain->Present(0, 0);
}
