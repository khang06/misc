// clang++ -m32 -fno-builtin -O2 -c sample2.cpp -o sample2.obj

#include <Windows.h>

class AsciiInf {
public:
    void drawText(float* pos, const char* format, ...);
};

extern "C" AsciiInf* g_ascii;

static void* g_base = 0;

extern "C" void coff2binhack_init() {
    MessageBoxA(NULL, "This is being called from the coff2binhack initializer!", "coff2binhack sample", 0);
    MessageBoxW(NULL, L"これはひどく翻訳された日本語の文字列である！", L"coff2binhack sample (wide)", 0);
    g_base = (void*)GetModuleHandleA(NULL);
}

extern "C" void hook_entry() {
    float pos[3] = {0.0f, 0.0f, 0.0f};
    g_ascii->drawText(pos, "Hello from C++!\nImage base: %p", g_base);
}
