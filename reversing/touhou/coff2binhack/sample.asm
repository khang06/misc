    global  ping_code
    extern  g_networkLogicInf
    extern  g_ascii
    extern  draw_debug_text

    section .text
ping_code:
    mov     eax, [g_networkLogicInf]
    test    eax, eax
    jz      done
    add     eax, 0x10          ; g_networkLogicInf->client
    mov     eax, [eax+0xC]     ; g_networkLogicInf->client->mpPeer
    test    eax, eax
    jz      done
    mov     eax, [eax+0x2C]    ; g_networkLogicInf->client->mpPeer->mpPeerBase
    test    eax, eax
    jz      done
    push    dword [eax+0x40]   ; g_networkLogicInf->client->mpPeer->mpPeerBase->roundTripTime
    push    ping_format
    push    ping_pos
    push    dword [g_ascii]
    call    draw_debug_text
    add     esp, 0x10
done:
    mov     eax, 1
    ret

    section .rdata
ping_format db  "Ping: %d ms", 0
ping_pos    dd  16.0, 470.0, 0.0
