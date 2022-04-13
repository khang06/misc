#include <Windows.h>
#include <cstdio>
#include <MinHook.h>
#include <regex>
#include "Common.h"
#include "HookHelpers.h"

// redirect image integrity check to original exe instead of noaslr
auto orig_GetModuleFileNameW = (DWORD(WINAPI*)(HMODULE, LPWSTR, DWORD))nullptr;
DWORD WINAPI custom_GetModuleFileNameW(HMODULE hModule, LPWSTR lpFilename, DWORD nSize) {
    auto ret = orig_GetModuleFileNameW(hModule, lpFilename, nSize);
    printf("GetModuleFileNameW: %ws\n", lpFilename);

    std::wstring new_path = std::regex_replace(lpFilename, std::wregex(L"-noaslr\\.exe"), L".exe");
    printf("New path: %ws\n", new_path.c_str());
    wcsncpy(lpFilename, new_path.c_str(), nSize);
    return wcslen(lpFilename);
}

// try to suspend the process during security cookie initialization
/*
.text:0000000180056960 ; unsigned __int64 __fastcall _get_entropy()
.text:0000000180056960 __get_entropy   proc near               ; CODE XREF: __security_init_cookie:loc_180056A7A↓p
.text:0000000180056960                                         ; DATA XREF: .pdata:000000018007798C↓o
.text:0000000180056960
.text:0000000180056960 var_28          = qword ptr -28h
.text:0000000180056960 SystemTimeAsFileTime= _FILETIME ptr -20h
.text:0000000180056960 PerformanceCount= LARGE_INTEGER ptr -18h
.text:0000000180056960
.text:0000000180056960                 push    rdi
.text:0000000180056962                 sub     rsp, 40h
.text:0000000180056966                 lea     rax, [rsp+48h+SystemTimeAsFileTime]
.text:000000018005696B                 mov     rdi, rax
.text:000000018005696E                 xor     eax, eax
.text:0000000180056970                 mov     ecx, 8
.text:0000000180056975                 rep stosb
.text:0000000180056977                 lea     rcx, [rsp+48h+SystemTimeAsFileTime] ; lpSystemTimeAsFileTime
.text:000000018005697C                 call    cs:__imp_GetSystemTimeAsFileTime
.text:0000000180056982                 mov     rax, qword ptr [rsp+48h+SystemTimeAsFileTime.dwLowDateTime]
.text:0000000180056987                 mov     [rsp+48h+var_28], rax
.text:000000018005698C                 call    cs:__imp_GetCurrentThreadId
.text:0000000180056992                 mov     eax, eax
.text:0000000180056994                 mov     rcx, [rsp+48h+var_28]
.text:0000000180056999                 xor     rcx, rax
.text:000000018005699C                 mov     rax, rcx
.text:000000018005699F                 mov     [rsp+48h+var_28], rax
.text:00000001800569A4                 call    cs:__imp_GetCurrentProcessId
.text:00000001800569AA                 mov     eax, eax
.text:00000001800569AC                 mov     rcx, [rsp+48h+var_28]
.text:00000001800569B1                 xor     rcx, rax
.text:00000001800569B4                 mov     rax, rcx
.text:00000001800569B7                 mov     [rsp+48h+var_28], rax
.text:00000001800569BC                 lea     rcx, [rsp+48h+PerformanceCount] ; lpPerformanceCount
.text:00000001800569C1                 call    cs:__imp_QueryPerformanceCounter
.text:00000001800569C7                 mov     eax, dword ptr [rsp+48h+PerformanceCount]
*/
static int cookie_init_counter = 0;

auto orig_GetSystemTimeAsFileTime = (void(WINAPI*)(LPFILETIME))nullptr;
void WINAPI custom_GetSystemTimeAsFileTime(LPFILETIME lpSystemTimeAsFileTime) {
    cookie_init_counter = 1;
    orig_GetSystemTimeAsFileTime(lpSystemTimeAsFileTime);
}

auto orig_GetCurrentThreadId = (DWORD(WINAPI*)())nullptr;
DWORD WINAPI custom_GetCurrentThreadId() {
    if (cookie_init_counter != 1)
        cookie_init_counter = 0;
    else
        cookie_init_counter++;
    return orig_GetCurrentThreadId();
}

auto orig_GetCurrentProcessId = (DWORD(WINAPI*)())nullptr;
DWORD WINAPI custom_GetCurrentProcessId() {
    if (cookie_init_counter != 2)
        cookie_init_counter = 0;
    else
        cookie_init_counter++;
    return orig_GetCurrentProcessId();
}

auto orig_QueryPerformanceCounter = (BOOL(WINAPI*)(LARGE_INTEGER*))nullptr;
DWORD WINAPI custom_QueryPerformanceCounter(LARGE_INTEGER* lpPerformanceCount) {
    if (cookie_init_counter != 3) {
        cookie_init_counter = 0;
    } else {
        printf("GOT IT!!!!!!!! suspending current thread\n");
        SuspendThread(GetCurrentThread());
    }
    return orig_QueryPerformanceCounter(lpPerformanceCount);
}

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH: {
        if (!AllocConsole())
            Common::Panic("Failed to create a console window");
        freopen("CONOUT$", "w", stdout);

        char* image_base = (char*)GetModuleHandleW(NULL);
        printf("Hi\n");
        printf("Image base: %p\n", image_base);

        // fix the aslr flag (i know this sucks!)
        DWORD old_prot = 0;
        VirtualProtect(&image_base[0x17E], 1, PAGE_READWRITE, &old_prot);
        image_base[0x17E] = 0x60;
        VirtualProtect(&image_base[0x17E], 1, old_prot, &old_prot);

        if (MH_Initialize() != MH_OK)
            Common::Panic("Failed to initialize MinHook");

        MH_STATUS ret;
        MAKE_HOOK(GetModuleFileNameW);
        MAKE_HOOK(GetSystemTimeAsFileTime);
        MAKE_HOOK(GetCurrentThreadId);
        MAKE_HOOK(GetCurrentProcessId);
        MAKE_HOOK(QueryPerformanceCounter);
        break;
    }
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

