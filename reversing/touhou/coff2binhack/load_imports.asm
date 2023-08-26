    ; nasm -f bin load_imports.asm -o load_imports.bin
    ; zero318 wrote 99% of this because I'm bad at ASM
    %use smartalign
    alignmode p6
    bits 32
make_iat:
    push    ebp
    push    ebx
    push    edi
    push    esi
    push    eax
    push    0x41414141                      ; 0x41414141 = "kernel32.dll"
    call    0x42424242                      ; 0x42424242 = th_GetModuleHandleA
    push    0x43434343                      ; 0x43434343 = "LoadLibraryA"
    push    eax
    call    0x44444444                      ; 0x44444444 = th_GetProcAddress
    mov     dword [esp], eax
    xor     ebp, ebp
    mov     ebx, 0x45454545                 ; 0x45454545 = dll_array
    mov     esi, dword [0x45454545]
align 16
load_dll:
    push    esi
    call    0x42424242
    mov     edi, eax
    test    eax, eax
    jnz     module_is_loaded
    push    esi
    call    dword [esp+4]
    mov     edi, eax
    test    eax, eax
    jz      cannot_find_dll_error
module_is_loaded:
    mov     esi, dword [ebx+4]
    cmp     ebp, esi
    je      next_dll
align 16
next_func:
    push    dword [ebp*4+0x46464646]        ; 0x46464646 = func_strings
    push    edi
    call    0x44444444
    test    eax, eax
    jz      cannot_find_func_error
    mov     dword [ebp*4+0x47474747], eax   ; 0x47474747 = iat_array
    inc     ebp
    cmp     esi, ebp
    jne     next_func
next_dll:
    mov     esi, dword [ebx+8]
    add     ebx, 8
    test    esi, esi
    jnz     load_dll

    add     esp, 4
    pop     esi
    pop     edi
    pop     ebx
    pop     ebp
    db      "DUMMY"                         ; Will be patched to either ret or jmp instruction
cannot_find_dll_error:
    push    eax
    push    esi
    push    0x48484848                      ; 0x48484848 = "Failed to load DLL %s (code: 0x%x)"
    jmp     fatal_mbox
cannot_find_func_error:
    push    eax
    push    dword [ebp*4+0x46464646]
    push    0x49494949                      ; 0x49494949 = "Failed to load import %s (code: 0x%x)"
fatal_mbox:
    call    0x4C4C4C4C                      ; 0x4C4C4C4C = th_GetLastError
    mov     dword [esp+8], eax
    push    0x10                            ; MB_ICONERROR
    push    0x4A4A4A4A                      ; 0x4A4A4A4A = patch prefix
    call    0x4B4B4B4B                      ; 0x4B4B4B4B = log_mboxf
    int3
