    ; nasm -f bin load_imports.asm -o load_imports.bin
    bits    32
    section .text
start:
    push    ebx
    push    ebp
    push    esi
    push    edi
    mov     ebx, 0x41414141             ; ebx = init data
    mov     ebp, 0x42424242             ; ebp = import output data

dll_loop:
    ; Break if DLL name offset is -1
    mov     eax, dword [ebx]
    cmp     eax, -1
    jz      end

    ; Get DLL handle and import count
    add     eax, 0x43434343             ; 0x43434343 = string data address
    push    eax                         ; lpModuleName
    call    0x44444444                  ; GetModuleHandleA
    mov     esi, eax                    ; esi = DLL handle
    mov     edi, dword [ebx + 4]        ; edi = import count
    add     ebx, 8

import_loop:
    ; Load the import
    push    dword [ebx]                 ; lpProcName
    add     dword [esp], 0x43434343
    push    esi                         ; hModule
    call    0x45454545                  ; GetProcAddress

    ; Write to import array
    mov     dword [ebp], eax
    add     ebp, 4
    add     ebx, 4
    sub     edi, 1
    jz      dll_loop
    jmp     import_loop

end:
    dd      0x46464646                  ; Marker for inserting other init code
    pop     edi
    pop     esi
    pop     ebp
    pop     ebx
    ret
