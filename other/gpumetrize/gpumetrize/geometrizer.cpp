#include <algorithm>
#include <stdint.h>
#define _USE_MATH_DEFINES
#include <math.h>
#include "consts.h"
#include "util.h"
#include "lodepng.h"
#include "geometrizer.h"

#include "MainVertexShader.h"
#include "MainGeometryShader.h"
#include "StencilPixelShader.h"
#include "SpritePixelShader.h"
#include "AvgColorComputeShader.h"
#include "ErrorComputeShader.h"
#include "FullscreenVertexShader.h"
#include "FullscreenPixelShader.h"

#include "bo3atlas.h"

#define LOAD_SHADER(type, name) \
    res = m_pDevice->Create##type##Shader(g_p##name##type##Shader, sizeof(g_p##name##type##Shader), nullptr, &m_p##name##type##Shader); \
    if (FAILED(res)) \
        Panic("Create" #type "Shader (" #name ") failed: 0x%X", res);

Texture::Texture(ID3D11Device1* device, const char* type, DXGI_FORMAT format, UINT width, UINT height, bool with_uav, bool filtering, const void* data) {
    UINT bpp = format == DXGI_FORMAT_R8_UNORM ? 1 : 4;

    // Create texture and upload data if needed
    D3D11_TEXTURE2D_DESC tex_desc = {
        .Width = width,
        .Height = height,
        .MipLevels = 1,
        .ArraySize = 1,
        .Format = format,
        .SampleDesc = {
            .Count = 1,
            .Quality = 0,
        },
        .Usage = data ? D3D11_USAGE_IMMUTABLE : D3D11_USAGE_DEFAULT,
        .BindFlags = D3D11_BIND_SHADER_RESOURCE | (data ? 0u : (with_uav ? D3D11_BIND_UNORDERED_ACCESS : D3D11_BIND_RENDER_TARGET)),
        .CPUAccessFlags = 0,
        .MiscFlags = 0,
    };
    D3D11_SUBRESOURCE_DATA tex_data = {
        .pSysMem = data,
        .SysMemPitch = width * bpp,
        .SysMemSlicePitch = width * height * bpp,
    };
    HRESULT res = device->CreateTexture2D(&tex_desc, data ? &tex_data : nullptr, &tex);
    if (FAILED(res))
        Panic("CreateTexture2D for %s failed: 0x%X", type, res);

    // Create shader resource view
    D3D11_SHADER_RESOURCE_VIEW_DESC tex_srv = {
        .Format = format,
        .ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D,
        .Texture2D = {
            .MostDetailedMip = 0,
            .MipLevels = 1,
        }
    };
    res = device->CreateShaderResourceView(tex.Get(), &tex_srv, &srv);
    if (FAILED(res))
        Panic("CreateShaderResourceView for %s failed: 0x%X", type, res);

    // Create sampler
    D3D11_SAMPLER_DESC sampler_desc = {
        .Filter = filtering ? D3D11_FILTER_MIN_MAG_MIP_LINEAR : D3D11_FILTER_MIN_MAG_MIP_POINT,
        .AddressU = D3D11_TEXTURE_ADDRESS_WRAP,
        .AddressV = D3D11_TEXTURE_ADDRESS_WRAP,
        .AddressW = D3D11_TEXTURE_ADDRESS_WRAP,
        .MipLODBias = 0.0f,
        .ComparisonFunc = D3D11_COMPARISON_ALWAYS,
        .MinLOD = 0.0f,
        .MaxLOD = 0.0f,
    };
    device->CreateSamplerState(&sampler_desc, &sampler);
    if (FAILED(res))
        Panic("CreateSamplerState for %s failed: 0x%X", type, res);

    if (!data && !with_uav) {
        // Create render target view
        res = device->CreateRenderTargetView(tex.Get(), NULL, &rtv);
        if (FAILED(res))
            Panic("CreateRenderTargetView for %s failed: 0x%X", type, res);
    } else if (with_uav) {
        // Create unordered access view
        res = device->CreateUnorderedAccessView(tex.Get(), NULL, &uav);
        if (FAILED(res))
            Panic("CreateUnorderedAccessView for %s failed: 0x%X", type, res);
    }
}

Geometrizer::Geometrizer(HWND hwnd, const char* image) {
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

    // Create and upload projection matrix
    D3D11_BUFFER_DESC proj_desc = {
        .ByteWidth = sizeof(ConstantBuffer),
        .Usage = D3D11_USAGE_DYNAMIC,
        .BindFlags = D3D11_BIND_CONSTANT_BUFFER,
        .CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
        .MiscFlags = 0,
    };
    float L = 0.0f;
    float R = TARGET_WIDTH * GRID_SIZE;
    float T = 0.0f;
    float B = TARGET_HEIGHT * GRID_SIZE;
    m_pConstantBufferCPU = ConstantBuffer{
        .mvp = {
            { 2.0f / (R - L),    0.0f,              0.0f, 0.0f },
            { 0.0f,              2.0f / (T - B),    0.0f, 0.0f },
            { 0.0f,              0.0f,              0.5f, 0.0f },
            { (R + L) / (L - R), (T + B) / (B - T), 0.5f, 1.0f },
        }
    };
    D3D11_SUBRESOURCE_DATA proj_data = {
        .pSysMem = &m_pConstantBufferCPU
    };
    res = m_pDevice->CreateBuffer(&proj_desc, &proj_data, &m_pConstantBuffer);
    if (FAILED(res))
        Panic("CreateBuffer (projection matrix) failed: 0x%X", res);

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
        Panic("CreateBlendState failed: 0x%X", res);

    // Create a vertex buffer that holds the sprite data
    D3D11_BUFFER_DESC vertex_desc = {
        .ByteWidth = sizeof(SpriteInstance) * GRID_SIZE * GRID_SIZE,
        .Usage = D3D11_USAGE_DYNAMIC,
        .BindFlags = D3D11_BIND_VERTEX_BUFFER,
        .CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
        .MiscFlags = 0,
        .StructureByteStride = 0,
    };

    res = m_pDevice->CreateBuffer(&vertex_desc, NULL, &m_pVertexBuffer);
    if (FAILED(res))
        Panic("CreateBuffer (vertex) failed: 0x%X", res);

    // Create a buffer for the sprite bounding boxes
    D3D11_BUFFER_DESC bounds_desc = {
        .ByteWidth = GRID_SIZE * GRID_SIZE * 4 * sizeof(float),
        .Usage = D3D11_USAGE_DYNAMIC,
        .BindFlags = D3D11_BIND_SHADER_RESOURCE,
        .CPUAccessFlags = D3D11_CPU_ACCESS_WRITE,
        .MiscFlags = 0,
        .StructureByteStride = 0,
    };

    res = m_pDevice->CreateBuffer(&bounds_desc, NULL, &m_pBoundsBuffer);
    if (FAILED(res))
        Panic("CreateBuffer (bounds) failed: 0x%X", res);

    // Create shader resource view for sprite bounding boxes
    D3D11_SHADER_RESOURCE_VIEW_DESC bounds_srv = {
        .Format = DXGI_FORMAT_R32G32B32A32_FLOAT,
        .ViewDimension = D3D11_SRV_DIMENSION_BUFFER,
        .Buffer = {
            .FirstElement = 0,
            .NumElements = GRID_SIZE * GRID_SIZE,
        }
    };
    res = device->CreateShaderResourceView(m_pBoundsBuffer.Get(), &bounds_srv, &m_pBoundsSRV);
    if (FAILED(res))
        Panic("CreateShaderResourceView for bounds failed: 0x%X", res);

    // Load target image
    uint8_t* img;
    unsigned w;
    unsigned h;
    auto ret = lodepng_decode32_file(&img, &w, &h, image);
    if (ret)
        Panic("Failed to load target image: %s", lodepng_error_text(ret));
    if (w != TARGET_WIDTH || h != TARGET_HEIGHT)
        Panic("Target image must be %dx%d", TARGET_WIDTH, TARGET_HEIGHT);

    // Create work textures
    m_TargetTex = Texture(m_pDevice.Get(), "target", DXGI_FORMAT_R8G8B8A8_UNORM, TARGET_WIDTH, TARGET_HEIGHT, false, false, img);
    m_StencilTex = Texture(m_pDevice.Get(), "stencil", DXGI_FORMAT_R8G8B8A8_UNORM, TARGET_WIDTH * GRID_SIZE / 2, TARGET_HEIGHT * GRID_SIZE / 2);
    m_CurAvgColorTex = Texture(m_pDevice.Get(), "current average color", DXGI_FORMAT_R8G8B8A8_UNORM, GRID_SIZE, GRID_SIZE, true);
    m_BestAvgColorTex = Texture(m_pDevice.Get(), "best average color", DXGI_FORMAT_R8G8B8A8_UNORM, GRID_SIZE, GRID_SIZE, true);
    m_CurCandidatesTex = Texture(m_pDevice.Get(), "current candidates", DXGI_FORMAT_R8G8B8A8_UNORM, TARGET_WIDTH * GRID_SIZE, TARGET_HEIGHT * GRID_SIZE);
    m_LocalBestCandidatesTex = Texture(m_pDevice.Get(), "local best candidates", DXGI_FORMAT_R8G8B8A8_UNORM, TARGET_WIDTH * GRID_SIZE, TARGET_HEIGHT * GRID_SIZE);
    m_GlobalBestCandidateTex = Texture(m_pDevice.Get(), "global best candidate", DXGI_FORMAT_R8G8B8A8_UNORM, TARGET_WIDTH, TARGET_HEIGHT);
    m_ErrorTex = Texture(m_pDevice.Get(), "error", DXGI_FORMAT_R32_FLOAT, GRID_SIZE, GRID_SIZE, true);
    m_BO3Atlas = Texture(m_pDevice.Get(), "bo3 atlas", DXGI_FORMAT_R8_UNORM, 1024, 1024, false, true, g_BO3Atlas);

    // Create staging texture to read error from CPU
    D3D11_TEXTURE2D_DESC staging_tex_desc = {
        .Width = GRID_SIZE,
        .Height = GRID_SIZE,
        .MipLevels = 1,
        .ArraySize = 1,
        .Format = DXGI_FORMAT_R32_FLOAT,
        .SampleDesc = {
            .Count = 1,
            .Quality = 0,
        },
        .Usage = D3D11_USAGE_STAGING,
        .BindFlags = 0,
        .CPUAccessFlags = D3D11_CPU_ACCESS_READ,
        .MiscFlags = 0,
    };
    res = device->CreateTexture2D(&staging_tex_desc, nullptr, &m_pErrorStagingTex);
    if (FAILED(res))
        Panic("CreateTexture2D for error staging failed: 0x%X", res);

    // Create staging texture to read average color from CPU
    D3D11_TEXTURE2D_DESC avg_color_tex_desc = {
        .Width = GRID_SIZE,
        .Height = GRID_SIZE,
        .MipLevels = 1,
        .ArraySize = 1,
        .Format = DXGI_FORMAT_R8G8B8A8_UNORM,
        .SampleDesc = {
            .Count = 1,
            .Quality = 0,
        },
        .Usage = D3D11_USAGE_STAGING,
        .BindFlags = 0,
        .CPUAccessFlags = D3D11_CPU_ACCESS_READ,
        .MiscFlags = 0,
    };
    res = device->CreateTexture2D(&avg_color_tex_desc, nullptr, &m_pAvgColorStagingTex);
    if (FAILED(res))
        Panic("CreateTexture2D for average color staging failed: 0x%X", res);

    // Clean up target image
    free(img);

    // Load the main vertex shader
    res = m_pDevice->CreateVertexShader(g_pMainVertexShader, sizeof(g_pMainVertexShader), nullptr, &m_pMainVertexShader);
    if (FAILED(res))
        Panic("CreateVertexShader (main) failed: 0x%X", res);

    // Define the main input layout
    D3D11_INPUT_ELEMENT_DESC main_input_layout[] = {
        { "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD", 1, DXGI_FORMAT_R32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD", 2, DXGI_FORMAT_R32_FLOAT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD", 3, DXGI_FORMAT_R32_UINT, 0, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_PER_VERTEX_DATA, 0 },
    };
    res = m_pDevice->CreateInputLayout(main_input_layout, _countof(main_input_layout), g_pMainVertexShader, sizeof(g_pMainVertexShader), &m_pMainVertexLayout);
    if (FAILED(res))
        Panic("CreateInputLayout (main) failed: 0x%X", res);

    LOAD_SHADER(Vertex, Main);
    LOAD_SHADER(Geometry, Main);
    LOAD_SHADER(Pixel, Stencil);
    LOAD_SHADER(Pixel, Sprite);
    LOAD_SHADER(Compute, AvgColor);
    LOAD_SHADER(Compute, Error);
    LOAD_SHADER(Vertex, Fullscreen);
    LOAD_SHADER(Pixel, Fullscreen);
}

DWORD __stdcall Geometrizer::ThreadProc(void* self) {
    ((Geometrizer*)self)->ThreadProcInner();
    return 0;
}

void Geometrizer::ThreadProcInner() {
    // D3D11 is dumb
    void* nullptrs[4] = {};
    float clear_color[4] = { 0.0f, 0.0f, 0.0f, 0.0f };

    // Initialize FPS counter
    uint64_t qpc_freq;
    uint64_t last_fps_update;
    size_t frames = 0;
    QueryPerformanceFrequency((LARGE_INTEGER*)&qpc_freq);
    QueryPerformanceCounter((LARGE_INTEGER*)&last_fps_update);
    
    // Seed RNG
    //Rand::Seed(0xDEADBEEF);
    Rand::Seed((uint32_t)last_fps_update);

    // Initialize best candidates textures
    m_pContext->ClearRenderTargetView(m_LocalBestCandidatesTex.rtv.Get(), clear_color);
    m_pContext->ClearRenderTargetView(m_GlobalBestCandidateTex.rtv.Get(), clear_color);

    size_t limit = 64;
    size_t iterations = 0;

    // Generate random sprites
    for (auto& inst : m_BestInstances) {
        inst.x = Rand::NextFloat() * ASPECT_RATIO;
        inst.y = Rand::NextFloat();
        inst.width = Rand::RangeFloat(-1.0f, 1.0f);
        inst.height = Rand::RangeFloat(-1.0f, 1.0f);
        inst.angle = Rand::NextFloat() * 6.2831855f;
        inst.alpha = Rand::RangeFloat(0.75f, 1.0f);
        inst.atlas_id = Rand::Range(0, 63);
    }

    // Main loop
    while (!m_bExitRequested) {
        // Initialize candidate errors
        for (size_t i = 0; i < _countof(m_BestScores); i++)
            m_BestScores[i] = 0.0f;

        for (int i = 0; i < MUTATION_ITERS; i++) {
            // Initialize current candidates texture
            D3D11_VIEWPORT candidates_viewport = {
                .TopLeftX = 0.0f,
                .TopLeftY = 0.0f,
                .Width = TARGET_WIDTH * GRID_SIZE,
                .Height = TARGET_HEIGHT * GRID_SIZE,
                .MinDepth = 0.0f,
                .MaxDepth = 1.0f,
            };
            m_pContext->RSSetViewports(1, &candidates_viewport);
            m_pConstantBufferCPU.fullscreen_uvs[0] = m_pConstantBufferCPU.fullscreen_uvs[1] = 0.0f;
            m_pConstantBufferCPU.fullscreen_uvs[2] = m_pConstantBufferCPU.fullscreen_uvs[3] = GRID_SIZE;
            UpdateConstantBuffer();
            m_pContext->OMSetBlendState(nullptr, 0, 0xFFFFFFFF);
            m_pContext->VSSetConstantBuffers(0, 1, m_pConstantBuffer.GetAddressOf());
            m_pContext->PSSetShaderResources(0, 1, m_GlobalBestCandidateTex.srv.GetAddressOf());
            m_pContext->PSSetSamplers(0, 1, m_GlobalBestCandidateTex.sampler.GetAddressOf());
            m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            m_pContext->VSSetShader(m_pFullscreenVertexShader.Get(), nullptr, 0);
            m_pContext->GSSetShader(nullptr, nullptr, 0);
            m_pContext->PSSetShader(m_pFullscreenPixelShader.Get(), nullptr, 0);
            m_pContext->OMSetRenderTargets(1, m_CurCandidatesTex.rtv.GetAddressOf(), nullptr);
            m_pContext->Draw(3, 0);
            m_pContext->OMSetRenderTargets(1, (ID3D11RenderTargetView**)&nullptrs, nullptr);

            // Mutate current best instances
            memcpy(m_CurInstances, m_BestInstances, sizeof(m_CurInstances));
            for (int j = 0; j < _countof(m_CurInstances); j++) {
                auto& inst = m_CurInstances[j];
                switch (Rand::Range(0, 6)) {
                    case 0:
                        inst.x += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        inst.y += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        break;
                    case 1:
                        inst.width += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        inst.height += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        //inst.width = max(1.0f / 32.0f, inst.width);
                        //inst.height = max(1.0f / 32.0f, inst.height);
                        break;
                    case 2:
                        inst.angle += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        inst.angle = fmodf(inst.angle, 6.2831855f);
                        break;
                    case 3:
                        inst.alpha += Rand::RangeFloat(-1.0f / 16.0f, 1.0f / 16.0f);
                        inst.alpha = max(0.0f, min(inst.alpha, 1.0f));
                        break;
                    case 4:
                        inst.width = -inst.width;
                        break;
                    case 5:
                        inst.height = -inst.height;
                        break;
                    case 6:
                        inst.atlas_id = Rand::Range(0, 63);
                        break;
                }

                float pcos = cosf(inst.angle);
                float psin = sinf(inst.angle);
                float bounds[4] = {inst.x, inst.y, inst.x, inst.y};
                for (int k = 0; k < 4; k++) {
                    float x = ((k / 2 == 0) ? -0.5f : 0.5f) * inst.width;
                    float y = ((k % 2 == 0) ? -0.5f : 0.5f) * inst.height;
                    float rot_x = pcos * x - psin * y;
                    float rot_y = psin * x + pcos * y;
                    bounds[0] = min(bounds[0], rot_x);
                    bounds[1] = min(bounds[1], rot_y);
                    bounds[2] = max(bounds[2], rot_x);
                    bounds[3] = max(bounds[3], rot_y);
                }
                m_CurBounds[j * 4 + 0] = max(0.0f, (inst.x + bounds[0]) / ASPECT_RATIO);
                m_CurBounds[j * 4 + 1] = max(0.0f, inst.y + bounds[1]);
                m_CurBounds[j * 4 + 2] = min((inst.x + bounds[2]) / ASPECT_RATIO, 1.0f);
                m_CurBounds[j * 4 + 3] = min(inst.y + bounds[3], 1.0f);
            }

            // Write candidate sprites to vertex buffer
            D3D11_MAPPED_SUBRESOURCE resource;
            HRESULT res = m_pContext->Map(m_pVertexBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
            if (FAILED(res))
                Panic("Failed to map the vertex buffer: 0x%X", res);
            memcpy(resource.pData, m_CurInstances, sizeof(m_CurInstances));
            m_pContext->Unmap(m_pVertexBuffer.Get(), 0);

            // Write sprite bounding boxes to bounds buffer
            res = m_pContext->Map(m_pBoundsBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &resource);
            if (FAILED(res))
                Panic("Failed to map the bounds buffer: 0x%X", res);
            memcpy(resource.pData, m_CurBounds, sizeof(m_CurBounds));
            m_pContext->Unmap(m_pBoundsBuffer.Get(), 0);

            // Draw candidate sprites as stencil to the average color underneath them
            D3D11_VIEWPORT stencil_viewport = {
                .TopLeftX = 0.0f,
                .TopLeftY = 0.0f,
                .Width = TARGET_WIDTH * GRID_SIZE / 2,
                .Height = TARGET_HEIGHT * GRID_SIZE / 2,
                .MinDepth = 0.0f,
                .MaxDepth = 1.0f,
            };
            ID3D11ShaderResourceView* stencil_srvs[] = { m_TargetTex.srv.Get(), m_BO3Atlas.srv.Get() };
            ID3D11SamplerState* stencil_samplers[] = { m_TargetTex.sampler.Get(), m_BO3Atlas.sampler.Get() };
            UINT stride = sizeof(SpriteInstance);
            UINT offset = 0;
            m_pContext->RSSetViewports(1, &stencil_viewport);
            m_pContext->GSSetConstantBuffers(0, 1, m_pConstantBuffer.GetAddressOf());
            m_pContext->PSSetShaderResources(0, 2, stencil_srvs);
            m_pContext->PSSetSamplers(0, 2, stencil_samplers);
            m_pContext->OMSetBlendState(m_pBlendState.Get(), 0, 0xFFFFFFFF);
            m_pContext->IASetVertexBuffers(0, 1, m_pVertexBuffer.GetAddressOf(), &stride, &offset);
            m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_POINTLIST);
            m_pContext->IASetInputLayout(m_pMainVertexLayout.Get());
            m_pContext->VSSetShader(m_pMainVertexShader.Get(), nullptr, 0);
            m_pContext->GSSetShader(m_pMainGeometryShader.Get(), nullptr, 0);
            m_pContext->PSSetShader(m_pStencilPixelShader.Get(), nullptr, 0);
            m_pContext->OMSetRenderTargets(1, m_StencilTex.rtv.GetAddressOf(), nullptr);
            m_pContext->ClearRenderTargetView(m_StencilTex.rtv.Get(), clear_color);
            m_pContext->Draw(GRID_SIZE * GRID_SIZE, 0);
            m_pContext->OMSetRenderTargets(1, (ID3D11RenderTargetView**)&nullptrs, nullptr);

            // Calculate average shape color from stencil
            ID3D11ShaderResourceView* avg_color_srvs[2] = {m_StencilTex.srv.Get(), m_pBoundsSRV.Get()};
            m_pContext->CSSetShaderResources(0, 2, avg_color_srvs);
            m_pContext->CSSetUnorderedAccessViews(0, 1, m_CurAvgColorTex.uav.GetAddressOf(), nullptr);
            m_pContext->CSSetShader(m_pAvgColorComputeShader.Get(), nullptr, 0);
            m_pContext->Dispatch(GRID_SIZE, GRID_SIZE, 1);
            m_pContext->CSSetShaderResources(0, 2, (ID3D11ShaderResourceView**)&nullptrs);
            m_pContext->CSSetUnorderedAccessViews(0, 1, (ID3D11UnorderedAccessView**)&nullptrs, nullptr);

            // Draw candidate sprites for real this time
            D3D11_VIEWPORT sprite_viewport = {
                .TopLeftX = 0.0f,
                .TopLeftY = 0.0f,
                .Width = TARGET_WIDTH * GRID_SIZE,
                .Height = TARGET_HEIGHT * GRID_SIZE,
                .MinDepth = 0.0f,
                .MaxDepth = 1.0f,
            };
            ID3D11ShaderResourceView* sprite_srvs[] = { m_CurAvgColorTex.srv.Get(), m_BO3Atlas.srv.Get() };
            m_pContext->RSSetViewports(1, &sprite_viewport);
            m_pContext->OMSetBlendState(m_pBlendState.Get(), 0, 0xFFFFFFFF);
            m_pContext->PSSetShaderResources(0, 2, sprite_srvs);
            m_pContext->PSSetSamplers(0, 1, m_BO3Atlas.sampler.GetAddressOf());
            m_pContext->PSSetShader(m_pSpritePixelShader.Get(), nullptr, 0);
            m_pContext->OMSetRenderTargets(1, m_CurCandidatesTex.rtv.GetAddressOf(), nullptr);
            m_pContext->Draw(GRID_SIZE * GRID_SIZE, 0);
            m_pContext->PSSetShaderResources(0, 1, (ID3D11ShaderResourceView**)&nullptrs);
            m_pContext->OMSetRenderTargets(1, (ID3D11RenderTargetView**)&nullptrs, nullptr);

            // Calculate error
            ID3D11ShaderResourceView* error_srvs[] = { m_TargetTex.srv.Get(), m_CurCandidatesTex.srv.Get(), m_GlobalBestCandidateTex.srv.Get(), m_pBoundsSRV.Get() };
            m_pContext->CSSetShaderResources(0, 4, error_srvs);
            m_pContext->CSSetUnorderedAccessViews(0, 1, m_ErrorTex.uav.GetAddressOf(), nullptr);
            m_pContext->CSSetShader(m_pErrorComputeShader.Get(), nullptr, 0);
            m_pContext->Dispatch(GRID_SIZE, GRID_SIZE, 1);
            m_pContext->CSSetShaderResources(0, 4, (ID3D11ShaderResourceView**)&nullptrs);
            m_pContext->CSSetUnorderedAccessViews(0, 1, (ID3D11UnorderedAccessView**)&nullptrs, nullptr);

            // Read local candidate scores
            m_pContext->CopyResource(m_pErrorStagingTex.Get(), m_ErrorTex.tex.Get());
            res = m_pContext->Map(m_pErrorStagingTex.Get(), 0, D3D11_MAP_READ, 0, &resource);
            if (FAILED(res))
                Panic("Failed to map the vertex buffer: 0x%X", res);
            for (int j = 0; j < GRID_SIZE; j++)
                memcpy(&m_CurScores[j * GRID_SIZE], &((uint8_t*)resource.pData)[j * resource.RowPitch], GRID_SIZE * sizeof(float));
            m_pContext->Unmap(m_pErrorStagingTex.Get(), 0);

            // Update local best candidates if there was an improvement
            for (size_t j = 0; j < _countof(m_CurScores); j++) {
                if (m_CurScores[j] > m_BestScores[j]) {
                    UINT cell_x = j % GRID_SIZE;
                    UINT cell_y = j / GRID_SIZE;
                    D3D11_BOX box = {
                        .left = cell_x * TARGET_WIDTH,
                        .top = cell_y * TARGET_HEIGHT,
                        .front = 0,
                        .right = (cell_x + 1) * TARGET_WIDTH,
                        .bottom = (cell_y + 1) * TARGET_HEIGHT,
                        .back = 1,
                    };
                    m_pContext->CopySubresourceRegion(m_LocalBestCandidatesTex.tex.Get(), 0, cell_x * TARGET_WIDTH, cell_y * TARGET_HEIGHT, 0, m_CurCandidatesTex.tex.Get(), 0, &box);
                    box.left = cell_x;
                    box.top = cell_y;
                    box.right = cell_x + 1;
                    box.bottom = cell_y + 1;
                    m_pContext->CopySubresourceRegion(m_BestAvgColorTex.tex.Get(), 0, cell_x, cell_y, 0, m_CurAvgColorTex.tex.Get(), 0, &box);
                    memcpy(&m_BestInstances[j], &m_CurInstances[j], sizeof(SpriteInstance));
                    m_BestScores[j] = m_CurScores[j];
                }
            }

            // Reroll the worst 1% of candidates
            for (size_t j = 0; j < _countof(m_BestScores); j++)
                m_PruneScores[j] = std::make_pair(j, m_BestScores[j]);
            std::nth_element(&m_PruneScores[0], &m_PruneScores[_countof(m_PruneScores) / 100], &m_PruneScores[_countof(m_PruneScores)],
                [](const auto& a, const auto& b) {
                    return a.second < b.second;
                }
            );
            //printf("underperformer threshold %f\n", m_PruneScores[_countof(m_PruneScores) / 100].second);
            for (size_t j = 0; j < _countof(m_PruneScores) / 100; j++) {
                auto& inst = m_BestInstances[m_PruneScores[j].first];
                inst.x = Rand::NextFloat() * ASPECT_RATIO;
                inst.y = Rand::NextFloat();
                inst.width = Rand::RangeFloat(-1.0f, 1.0f);
                inst.height = Rand::RangeFloat(-1.0f, 1.0f);
                inst.angle = Rand::NextFloat() * 6.2831855f;
                inst.alpha = Rand::RangeFloat(0.75f, 1.0f);
                inst.atlas_id = Rand::Range(0, 63);
                m_BestScores[m_PruneScores[j].first] = 0.0f;
            }

            // Update framerate
            uint64_t cur_qpc;
            QueryPerformanceCounter((LARGE_INTEGER*)&cur_qpc);
            if (cur_qpc >= last_fps_update + qpc_freq) {
                printf("%llu checks per second\n", frames);
                last_fps_update = cur_qpc;
                frames = 0;
            }
            else {
                frames++;
            }
        }

        // Find the global best candidate
        float best_candidate_score = m_BestScores[0];
        size_t best_candidate_score_idx = 0;
        for (size_t i = 1; i < _countof(m_BestScores); i++) {
            if (m_BestScores[i] > best_candidate_score) {
                best_candidate_score = m_BestScores[i];
                best_candidate_score_idx = i;
            }
        }

        // If the best candidate was better than the current global best, use it
        if (best_candidate_score > 0.0f) {
            // Copy local best to global best
            UINT cell_x = best_candidate_score_idx % GRID_SIZE;
            UINT cell_y = best_candidate_score_idx / GRID_SIZE;
            D3D11_BOX box = {
                .left = cell_x * TARGET_WIDTH,
                .top = cell_y * TARGET_HEIGHT,
                .front = 0,
                .right = (cell_x + 1) * TARGET_WIDTH,
                .bottom = (cell_y + 1) * TARGET_HEIGHT,
                .back = 1,
            };
            m_pContext->CopySubresourceRegion(m_GlobalBestCandidateTex.tex.Get(), 0, 0, 0, 0, m_LocalBestCandidatesTex.tex.Get(), 0, &box);

            D3D11_MAPPED_SUBRESOURCE resource;
            m_ChosenInstances.push_back(m_BestInstances[best_candidate_score_idx]);
            m_pContext->CopyResource(m_pAvgColorStagingTex.Get(), m_BestAvgColorTex.tex.Get());
            HRESULT res = m_pContext->Map(m_pAvgColorStagingTex.Get(), 0, D3D11_MAP_READ, 0, &resource);
            if (FAILED(res))
                Panic("Failed to map the average color staging texture: 0x%X", res);
            m_ChosenColors.push_back(((uint32_t*)resource.pData)[cell_y * resource.RowPitch / 4 + cell_x]);
            m_pContext->Unmap(m_pAvgColorStagingTex.Get(), 0);

            printf("%llu/%llu best score: %.8f\n", iterations + 1, limit, best_candidate_score);

            if (++iterations == limit) {
                printf("%llu sprite limit reached\n", limit);
                for (size_t i = 0; i < m_ChosenInstances.size(); i++) {
                    auto& inst = m_ChosenInstances[i];
                    //printf("%u %f %f %f %f %f %u %f\n", inst.atlas_id, inst.x, inst.y, inst.width, inst.height, inst.angle, m_ChosenColors[i], inst.alpha);

                    float r = (m_ChosenColors[i] & 0xFF) / 255.0f;
                    float g = ((m_ChosenColors[i] >> 8) & 0xFF) / 255.0f;
                    float b = ((m_ChosenColors[i] >> 16) & 0xFF) / 255.0f;

                    bool flip = false;
                    float angle = inst.angle * (180.0f / M_PI);
                    if (inst.height < 0.0f) {
                        angle += 180.0f;
                        flip = inst.width > 0.0f;
                    } else if (inst.width < 0.0f) {
                        flip = true;
                    }

                    float width = inst.width < 1.0f ? log2f(abs(inst.width)) : (inst.width - 1.0f);
                    float height = inst.height < 1.0f ? log2f(abs(inst.height)) : (inst.height - 1.0f);

                    printf(
                        "emblemSelect %llu 3;"
                        "emblemSetSelectedLayerIconId %u;"
                        "emblemSetPosition %f %f;"
                        "emblemSetRotation %f;"
                        "emblemScale %f %f;"
                        "emblemLayerColor1 %f %f %f %f;"
                        "emblemLayerColor2 0 0 0 0 %f %f %f %f;"
                        "%s",

                        i,
                        inst.atlas_id + 134,
                        inst.x - 0.825f, inst.y - 0.5f,
                        angle,
                        width, height,
                        r, g, b, inst.alpha,
                        r, g, b, inst.alpha,
                        flip ? "emblemToggleFlip;" : ""
                    );
                }
                m_bExitRequested = true;
            }
        } else {
            printf("no improvement!!!\n");
        }

        // Copy to debug window
        D3D11_VIEWPORT preview_viewport = {
            .TopLeftX = 0.0f,
            .TopLeftY = 0.0f,
            .Width = WINDOW_WIDTH,
            .Height = WINDOW_HEIGHT,
            //.Width = TARGET_WIDTH * GRID_SIZE,
            //.Height = TARGET_HEIGHT * GRID_SIZE,
            .MinDepth = 0.0f,
            .MaxDepth = 1.0f,
        };
        m_pContext->RSSetViewports(1, &preview_viewport);
        m_pConstantBufferCPU.fullscreen_uvs[0] = m_pConstantBufferCPU.fullscreen_uvs[1] = 0.0f;
        m_pConstantBufferCPU.fullscreen_uvs[2] = m_pConstantBufferCPU.fullscreen_uvs[3] = 1.0f;
        UpdateConstantBuffer();
        m_pContext->OMSetBlendState(nullptr, 0, 0xFFFFFFFF);
        m_pContext->PSSetShaderResources(0, 1, m_GlobalBestCandidateTex.srv.GetAddressOf());
        m_pContext->PSSetSamplers(0, 1, m_GlobalBestCandidateTex.sampler.GetAddressOf());
        m_pContext->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        m_pContext->VSSetShader(m_pFullscreenVertexShader.Get(), nullptr, 0);
        m_pContext->GSSetShader(nullptr, nullptr, 0);
        m_pContext->PSSetShader(m_pFullscreenPixelShader.Get(), nullptr, 0);
        m_pContext->OMSetRenderTargets(1, m_pRTV.GetAddressOf(), nullptr);

        m_pContext->ClearRenderTargetView(m_pRTV.Get(), clear_color);
        m_pContext->Draw(3, 0);
        m_pSwapChain->Present(0, 0);
    }
}
