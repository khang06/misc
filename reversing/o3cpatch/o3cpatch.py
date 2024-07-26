from Crypto.Cipher import AES
import re

# This gets loaded into memory at 0x4000
LOAD_OFFSET = 0x4000

# Pycryptodome is annoying and doesn't let me reuse ciphers
def new_cipher():
    KEY = b"\xc4\x05\x3d\xdf\x22\x5e\x89\xf7\x48\x68\xc1\xe1\xf4\xc0\x0d\x51\x4f\x02\xa8\xa8\x69\x2f\x99\x78\x69\xab\xeb\x15\x52\x50\x15\x0c"
    return AES.new(KEY, AES.MODE_CBC, b"\x00" * 16)

def encode_j(src, dst):
    offset = dst - src
    if offset < -(1 << 20) or offset > (1 << 20) - 1:
        raise Exception("Jump is too large")
    offset &= (1 << 21) - 1
    return (0x6F | (offset & 0xFF000) | ((offset & 0x800) << 9) | ((offset & 0x7FE) << 20) | ((offset & 0x100000) << 11)).to_bytes(4, "little")

def encode_jal_ra(src, dst):
    offset = dst - src
    if offset < -(1 << 20) or offset > (1 << 20) - 1:
        raise Exception("Jump is too large")
    offset &= (1 << 21) - 1
    return (0xEF | (offset & 0xFF000) | ((offset & 0x800) << 9) | ((offset & 0x7FE) << 20) | ((offset & 0x100000) << 11)).to_bytes(4, "little")

def encode_li(reg, addr):
    lui = 0x37 | (reg << 7) | (addr & 0xFFFFF000)
    addi = 0x13 | (reg << 7) | (reg << 15) | ((addr & 0x7FF) << 20)
    return (lui | (addi << 32)).to_bytes(8, "little")

# Not used right now but I'm sure as hell not rewriting this when I do need it
def encode_bne(src, dst, rs1, rs2):
    offset = dst - src
    if offset < -(1 << 12) or offset > (1 << 12) - 1:
        raise Exception(f"Jump is too large ({hex(offset)}, max is {hex((1 << 12) - 1)})")
    offset &= (1 << 13) - 1
    return (
        0x63 |      # BRANCH opcode
        ((offset & 0x800) >> 4) |
        ((offset & 0x1E) << 7) |
        0x1000 |    # BNE
        (rs1 << 15) |
        (rs2 << 20) |
        ((offset & 0x7E0) << 20) |
        ((offset & 0x1000) << 19)
    ).to_bytes(4, "little")

with open("firmware/app_O3C_v1.5_20240709.bin", "rb") as file:
    fw = bytearray(new_cipher().decrypt(file.read()))

def patch(addr, data):
    addr -= LOAD_OFFSET
    fw[addr:(addr + len(data))] = data

def patch_j(src, dst):
    patch(src, encode_j(src, dst))

def patch_jal_ra(src, dst):
    patch(src, encode_jal_ra(src, dst))

# Let's hope that I won't have to deal with C++ mangling here
with open("main.map", "r") as file:
    matches = re.finditer(r" {16}0x([0-9a-z]{16})\s+([A-Za-z_][A-Za-z0-9_]+)", file.read())
    syms = {x.group(2):int(x.group(1), 16) for x in matches}

# Neuter MD5 hash check by forcing it to check 0 bytes
# I think this should be version-agnostic?
print(f"orig size: {hex(int.from_bytes(fw[0x29F84:0x29F88], 'little'))}")
patch(0x2DF84, b"\x00" * 4)
patch(0x2DFA0, b"\xd4\x1d\x8c\xd9\x8f\x00\xb2\x04\xe9\x80\x09\x98\xec\xf8\x42\x7e")

# Patch reset vector to run custom init code
patch_j(0x4000, syms["handle_reset_custom"])

# Patch device info to run custom menu instead
patch_j(syms["menu_device_info"], syms["menu_device_info_custom"])

# Patch command handler to inject custom USB command
patch_j(syms["handle_usb_cmd_2_hook"], syms["get_analog_key"])

# Patch screen layer update to inject custom layer types and allow drawing in menus
patch_j(syms["screen_layer_update_hook"], syms["screen_layer_update_custom"])
patch_j(syms["screen_layer_update_menu_hook"], syms["screen_layer_update_menu_custom"])

# Patch menu tick to not clear the screen
patch_j(syms["menu_tick_fb_clear"], syms["menu_tick_fb_clear"] + 4)

# Patch TIM6's IRQ handler for keys per second display
patch_jal_ra(syms["ms_callback_hook"], syms["ms_callback_custom"])

# Inject custom code
with open("main.bin", "rb") as main:
    patch(syms["custom_flash_base"], main.read())

with open("firmware/app_O3C.bin", "wb") as file:
    file.write(new_cipher().encrypt(fw))
