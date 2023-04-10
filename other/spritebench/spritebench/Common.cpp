#include <Windows.h>
#include <stdarg.h>
#include <stdio.h>
#include "Common.h"

__declspec(noreturn) void Panic(const char* msg, ...) {
    va_list args;
    va_start(args, msg);

    char buf[1024] = {};
    vsnprintf(buf, sizeof(buf) - 1, msg, args);

    MessageBoxA(NULL, buf, "PANIC!!!!!!", 0);

    va_end(args);
    exit(1);
}