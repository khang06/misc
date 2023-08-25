// clang++ -m32 -fno-builtin -O2 -c sample2.cpp -o sample2.obj

#define WINBASEAPI
#include <Windows.h>

class AsciiInf {
public:
    void drawText(float* pos, const char* format, ...);
};

extern "C" AsciiInf* g_ascii;

extern "C" void hook_entry() {
    float pos[3] = {0.0f, 0.0f, 0.0f};
    g_ascii->drawText(pos, "Hello from C++!\nImage base: %p", GetModuleHandleA(NULL));
}
