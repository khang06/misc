#include <Windows.h>
#include <stdio.h>
#include "consts.h"
#include "geometrizer.h"
#include "util.h"

LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
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
        .lpszClassName = TEXT("GPUmetrizeMain"),
        .hIconSm = NULL,
    };
    if (!RegisterClassEx(&wcex))
        return NULL;

    // Adjust the target window size to account for the window border and title
    RECT rc = { 0, 0, (LONG)(WINDOW_WIDTH * ASPECT_RATIO), WINDOW_HEIGHT };
    AdjustWindowRect(&rc, WS_OVERLAPPEDWINDOW, FALSE);

    // Create the main window
    auto ret = CreateWindow(TEXT("GPUmetrizeMain"), TEXT("GPUmetrize"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT, CW_USEDEFAULT, rc.right - rc.left, rc.bottom - rc.top, nullptr, nullptr, hInstance,
        nullptr);
    if (ret == NULL)
        return NULL;

    // Show the window
    ShowWindow(ret, nShowCmd);

    return ret;
}

int main(int argc, char** argv) {
	if (argc != 2) {
		printf("Usage: %s <image>\n", argv[0]);
		return 1;
	}

    HWND window = CreateMainWindow((HINSTANCE)GetModuleHandle(NULL), SW_NORMAL);
    if (window == NULL)
        Panic("Failed to create main window! GLE: 0x%X", GetLastError());

    auto geom = new Geometrizer(window, argv[1]);
    geom->SpawnThread();

    MSG msg = {};
    while (GetMessage(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    delete geom;

    return 0;
}