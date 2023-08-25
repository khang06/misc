// clang++ -m32 -fno-builtin -O2 -c sample2.cpp -o sample2.obj

#define WINBASEAPI
#include <Windows.h>

class AsciiInf {
public:
    void drawText(float* pos, const char* format, ...);
};

extern "C" AsciiInf* g_ascii;

static bool g_initialized = false;
static void* g_base = 0;

void initialize() {
    MessageBoxA(NULL, "Testing imports", "coff2binhack sample", 0);
    g_base = (void*)GetModuleHandleA(NULL);
    g_initialized = true;
}

extern "C" void hook_entry() {
    if (!g_initialized)
        initialize();

    float pos[3] = {0.0f, 0.0f, 0.0f};
    g_ascii->drawText(pos, "Hello from C++!\nImage base: %p", g_base);
}
