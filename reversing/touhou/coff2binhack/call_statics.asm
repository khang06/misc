    ; nasm -f bin call_statics.asm -o call_statics.bin
    %use smartalign
    alignmode p6
    bits 32
call_statics:
    push    ebx
    xor     ebx, ebx
align 16
call_loop:
    call    dword [0x41414141+ebx*4]
    inc     ebx
    cmp     ebx, 0x42424242
    jnz     call_loop

    pop     ebx
    ; ret will be added by the script
