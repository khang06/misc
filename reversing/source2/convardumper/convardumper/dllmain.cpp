#include <Windows.h>
#include <stdio.h>
#include <stdint.h>
#include <unordered_set>
#include <string>
#include <algorithm>
#include <format>

//#define DUMP

__declspec(noreturn) void Panic(const char* msg, ...) {
    va_list args;
    va_start(args, msg);

    char buf[1024] = {};
    vsnprintf(buf, sizeof(buf) - 1, msg, args);

    MessageBoxA(NULL, buf, "PANIC!!!!!!", 0);

    va_end(args);
    exit(1);
}

struct ConVarInfo {
    char* name;       // 0x00
    char* desc;       // 0x08
    uint64_t flags;   // 0x10
    char pad18[0x40]; // 0x18
    uint64_t type;    // 0x58
};

struct ConCommandInfo {
    char* name;       // 0x00
    char* desc;       // 0x08
    uint64_t flags;   // 0x10
};

#ifdef DUMP
const char* ConVarTypeToString(uint64_t type) {
    const char* table[] = {
        "int16",
        "uint16",
        "int32",
        "uint32",
        "int64",
        "uint64",
        "float32",
        "float64",
        "string",
        "color",
        "vector2",
        "vector3",
        "vector4",
        "qangle",
        "cmd", // doesn't actually exist
    };
    if (type < _countof(table))
        return table[type];
    else
        return "unknown";
}

std::string ConVarFlagBitToString(uint32_t index) {
    // Names from cvarlist
    const char* table[32] = {
        "linked",               // 0x1
        "devonly",              // 0x2
        "sv",                   // 0x4
        "cl",                   // 0x8
        "hidden",               // 0x10
        "prot",                 // 0x20
        "sp",                   // 0x40
        "a",                    // 0x80
        "nf",                   // 0x100
        "user",                 // 0x200
        "unk400",               // 0x400
        "nolog",                // 0x800
        "unk1000",              // 0x1000
        "rep",                  // 0x2000
        "cheat",                // 0x4000
        "per_user",             // 0x8000
        "demo",                 // 0x10000
        "norecord",             // 0x20000
        "unk40000",             // 0x40000
        "release",              // 0x80000
        "menubar_item",         // 0x100000
        "sv, cl",               // 0x200000
        "disconnected",         // 0x400000
        "vconsole_fuzzy",       // 0x800000
        "server_can_execute",   // 0x1000000
        "unk2000000",           // 0x2000000
        "server_cant_query",    // 0x4000000
        "vconsole_set_focus",   // 0x8000000
        "clientcmd_can_execute",// 0x10000000
        "execute_per_tick",     // 0x20000000
        "unk40000000",          // 0x40000000
        "unk80000000"           // 0x80000000
    };
    return table[index];
}

static std::vector<ConVarInfo> seen_convars;
static std::unordered_set<std::string> seen_convars_set;
#endif

void(__fastcall* RegisterConVarOrig)(void*, ConVarInfo*, void*, void*, void*) = 0;
void RegisterConVarHook(void* self, ConVarInfo* info, void* unk1, void* unk2, void* unk3) {
#ifdef DUMP
    // Print out added convars and push to seen_convars if it's unseen
    if (seen_convars_set.count(std::string(info->name)) == 0) {
        printf("%s (%s, 0x%llX): %s\n", info->name, ConVarTypeToString(info->type), info->flags, info->desc ? info->desc : "(n/a)");
        seen_convars_set.insert(std::string(info->name));
        seen_convars.push_back(*info);
    }
#endif

    // Unhide hidden and devonly convars
    info->flags &= ~0x12;

    RegisterConVarOrig(self, info, unk1, unk2, unk3);
}

void(__fastcall* RegisterConCommandOrig)(void*, void*, ConCommandInfo*, void*, void*) = 0;
void RegisterConCommandHook(void* self, void* unk1, ConCommandInfo* info, void* unk2, void* unk3) {
#ifdef DUMP
    // Print out added commands and push to seen_convars if it's unseen
    if (seen_convars_set.count(std::string(info->name)) == 0) {
        printf("%s (cmd, 0x%llX): %s\n", info->name, info->flags, info->desc ? info->desc : "(n/a)");
        seen_convars_set.insert(std::string(info->name));
        seen_convars.push_back(ConVarInfo{
            .name = info->name,
            .desc = info->desc,
            .flags = info->flags,
            .type = 14,
        });
    }
#endif

    // Unhide hidden and devonly commands
    info->flags &= ~0x12;

    RegisterConCommandOrig(self, unk1, info, unk2, unk3);
}

#ifdef DUMP
DWORD OutputThread(void*) {
    // I don't feel like writing anything to automatically detect when the game's done loading
    MessageBoxA(NULL, "Click OK after the game finishes loading", "convardumper", 0);

    // Sort convars alphabetically for output
    std::sort(seen_convars.begin(), seen_convars.end(), [](const ConVarInfo& a, const ConVarInfo& b) {
        return std::string(a.name) < std::string(b.name);
    });

    // Dump them out to a HTML table
    FILE* output = fopen("output.html", "w");
    if (!output)
        Panic("Failed to open output.html");
    fprintf(output, "<table><tr><td>Name</td><td>Type</td><td>Flags</td><td>Description</td></tr>\n");
    for (auto& convar : seen_convars) {
        // Collect flags as a comma-separated list
        std::string flags;
        for (int i = 0; i < 32; i++) {
            if ((convar.flags >> i) & 1)
                flags += ConVarFlagBitToString(i) + ", ";
        }

        // Remove the trailing comma
        if (!flags.empty())
            flags.resize(flags.length() - 2);

        // Write row
        auto line = std::format(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            convar.name,
            ConVarTypeToString(convar.type),
            flags,
            (convar.desc && strlen(convar.desc)) ? std::string(convar.desc) : ""
        );
        fprintf(output, "%s", line.c_str());
    }
    fprintf(output, "</table>");
    fclose(output);

    // We're done here
    MessageBoxA(NULL, "Done", "convardumper", 0);
    ExitProcess(0);

    return 0;
}
#endif

void Attach() {
#ifdef DUMP
    // Open a console
    AllocConsole();
    freopen("CONOUT$", "w", stdout);
#endif

    // Find tier0
    auto tier0 = (char*)LoadLibraryA("tier0");
    if (!tier0)
        Panic("Failed to get tier0.dll's address");

    // Patch RegisterConVar in CCvar's vtable
    auto register_convar_ptr = (uint64_t*)(tier0 + 0x2CB5D8);
    DWORD old_prot;
    VirtualProtect(register_convar_ptr, 8, PAGE_EXECUTE_READWRITE, &old_prot);
    RegisterConVarOrig = (void(__fastcall*)(void*, ConVarInfo*, void*, void*, void*))*register_convar_ptr;
    *register_convar_ptr = (uint64_t)&RegisterConVarHook;
    VirtualProtect(register_convar_ptr, 8, old_prot, &old_prot);
    printf("Hooked RegisterConVar\n");

    // Patch RegisterConCommand in CCvar's vtable
    auto register_concommand_ptr = (uint64_t*)(tier0 + 0x2CB5F0);
    VirtualProtect(register_concommand_ptr, 8, PAGE_EXECUTE_READWRITE, &old_prot);
    RegisterConCommandOrig = (void(__fastcall*)(void*, void*, ConCommandInfo*, void*, void*))*register_concommand_ptr;
    *register_concommand_ptr = (uint64_t)&RegisterConCommandHook;
    VirtualProtect(register_concommand_ptr, 8, old_prot, &old_prot);
    printf("Hooked RegisterCommandVar\n");

#ifdef DUMP
    // Spawn a thread to wait for user input before dumping convars to table
    CreateThread(NULL, 0, OutputThread, NULL, 0, NULL);
#endif
}

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH: {
        Attach();
        break;
    }
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

