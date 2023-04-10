#pragma once

#define WINDOW_WIDTH 384
#define WINDOW_HEIGHT 448
#ifdef _DEBUG
#define MAX_SPRITES 100
#else
#define MAX_SPRITES 1000000
#endif

__declspec(noreturn) void Panic(const char* msg, ...);