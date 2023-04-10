#include <Windows.h>
#include <stdio.h>
#define _USE_MATH_DEFINES
#include <math.h>
#include <format>
#include "D3D11Renderer.h"
#include "Common.h"

SpriteRendererType selected_renderer = SpriteRendererType::CPUTransform;

LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {
    PAINTSTRUCT ps;

    switch (message) {
        case WM_PAINT:
            BeginPaint(hWnd, &ps);
            EndPaint(hWnd, &ps);
            break;

        case WM_KEYDOWN:
            if (wParam == 'S') {
                switch (selected_renderer) {
                    case SpriteRendererType::CPUTransform:
                        selected_renderer = SpriteRendererType::Instance;
                        break;
                    case SpriteRendererType::Instance:
                        selected_renderer = SpriteRendererType::GeometryShader;
                        break;
                    case SpriteRendererType::GeometryShader:
                        selected_renderer = SpriteRendererType::CPUTransform;
                        break;
                }
            }
            break;

        case WM_DESTROY:
            PostQuitMessage(0);
            break;

        default:
            return DefWindowProc(hWnd, message, wParam, lParam);
    }

    return 0;
}

HWND CreateMainWindow(HINSTANCE hInstance, int nShowCmd) {
    // Register the main window class
    WNDCLASSEX wcex = {
        .cbSize = sizeof(WNDCLASSEX),
        .style = CS_HREDRAW | CS_VREDRAW,
        .lpfnWndProc = WndProc,
        .cbClsExtra = 0,
        .cbWndExtra = 0,
        .hInstance = hInstance,
        .hIcon = NULL,
        .hCursor = NULL,
        .hbrBackground = (HBRUSH)(COLOR_WINDOW + 1),
        .lpszMenuName = NULL,
        .lpszClassName = TEXT("SpriteBenchMain"),
        .hIconSm = NULL,
    };
    if (!RegisterClassEx(&wcex))
        return NULL;

    // Adjust the target window size to account for the window border and title
    RECT rc = { 0, 0, WINDOW_WIDTH, WINDOW_HEIGHT };
    AdjustWindowRect(&rc, WS_OVERLAPPEDWINDOW, FALSE);

    // Create the main window
    auto ret = CreateWindow(TEXT("SpriteBenchMain"), TEXT("spritebench"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT, CW_USEDEFAULT, rc.right - rc.left, rc.bottom - rc.top, nullptr, nullptr, hInstance,
        nullptr);
    if (ret == NULL)
        return NULL;

    // Show the window
    ShowWindow(ret, nShowCmd);

    return ret;
}

std::string GetSpriteRendererName(SpriteRendererType renderer) {
    switch (renderer) {
        case SpriteRendererType::CPUTransform:
            return "CPU Transform";
        case SpriteRendererType::Instance:
            return "Instance";
        case SpriteRendererType::GeometryShader:
            return "Geometry";
    }
}

int WINAPI WinMain(
    _In_ HINSTANCE hInstance,
    _In_opt_ HINSTANCE hPrevInstance,
    _In_ LPSTR lpCmdLine,
    _In_ int nShowCmd
) {
    HWND window = CreateMainWindow(hInstance, nShowCmd);
    if (window == NULL)
        Panic("Failed to create main window! GLE: 0x%X", GetLastError());

    // Make a bunch of bullets to render
    size_t sprites_to_draw = MAX_SPRITES;
    std::vector<SpriteInstance> sprites;
    sprites.resize(sprites_to_draw);
    srand(0);
    for (int i = 0; i < sprites_to_draw; i++) {
        sprites[i] = {
            .x = (float)rand() / RAND_MAX * WINDOW_WIDTH,
            .y = (float)rand() / RAND_MAX * WINDOW_HEIGHT,
            .width = 16.0f,
            .height = 16.0f,
            .angle = (float)rand() / RAND_MAX * (float)M_PI * 2.0f,
            .tex_index = rand() % 16,
        };
    }

    timeBeginPeriod(1);
    MSG msg = {};
    SpriteRendererType cur_renderer = selected_renderer;
    auto renderer = new D3D11Renderer(window, cur_renderer);
    SetWindowTextA(window, std::format("spritebench | {} | FPS: N/A", GetSpriteRendererName(cur_renderer)).c_str());
    DWORD last_fps_update = timeGetTime();
    int frames = 0;
    while (msg.message != WM_QUIT) {
        if (PeekMessage(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        } else {
            /*
            // Too slow...
            for (int i = 0; i < sprites_to_draw; i++) {
                sprites[i].x += cosf(sprites[i].angle);
                sprites[i].y += sinf(sprites[i].angle);
                sprites[i].x = fmodf(sprites[i].x, WINDOW_WIDTH);
                sprites[i].y = fmodf(sprites[i].y, WINDOW_HEIGHT);
            }
            */
            if (cur_renderer != selected_renderer) {
                cur_renderer = selected_renderer;
                delete renderer;
                renderer = new D3D11Renderer(window, cur_renderer);
                last_fps_update = timeGetTime();
                frames = 0;
                SetWindowTextA(window, std::format("spritebench | {} | FPS: N/A", GetSpriteRendererName(cur_renderer)).c_str());
            }

            DWORD cur_time = timeGetTime();
            if (last_fps_update + 1000 <= cur_time) {
                SetWindowTextA(window, std::format("spritebench | {} | FPS: {}", GetSpriteRendererName(cur_renderer), frames).c_str());
                last_fps_update = cur_time;
                frames = 0;
            }
            renderer->Draw(sprites);
            frames++;
        }
    }
    timeEndPeriod(1);
    delete renderer;

    return (int)msg.wParam;
}