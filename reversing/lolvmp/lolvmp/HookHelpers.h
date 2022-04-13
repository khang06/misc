#pragma once

#include <MinHook.h>

// helpers
template <typename T>
inline MH_STATUS MH_CreateHookEx(LPVOID pTarget, LPVOID pDetour, T** ppOriginal)
{
    return MH_CreateHook(pTarget, pDetour, reinterpret_cast<LPVOID*>(ppOriginal));
}

// why do i even have to make 3 macros for 1???
#define MAKE_HOOK(x) ret = MH_CreateHookEx(x, MAKE_HOOK_HIDDEN1(x), MAKE_HOOK_HIDDEN2(x)); \
                     if (ret != MH_OK) \
                         Common::Panic("Failed to install "#x" hook (%d)", ret); \
                     ret = MH_EnableHook(x); \
                     if (ret != MH_OK) \
                         Common::Panic("Failed to enable "#x" hook (%d)", ret); \
                     printf("Installed hook for "#x"\n");
#define MAKE_HOOK_HIDDEN1(x) &custom_##x
#define MAKE_HOOK_HIDDEN2(x) &orig_##x