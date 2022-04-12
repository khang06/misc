import sys
import pefile

# A while ago, I downloaded a "150 Gamehouse Games Pack" off of archive.org.
# It included a bunch of classic tiny PC games like Hamsterball and Bejeweled 2.
# They run perfectly fine, but they all seemed to have some kind of "protection" on them.
# It isn't very complex and seems to be for some kind of demo time limit enforcement thing from GameHouse, which isn't even activated in this case.
# However, the demo timer activates if I modify the executable in any way, which is quite annoying.
# At the time, I just dumped the executable with Scylla and called it a day,
# but trying to statically strip the protection from this executable is a nice exercise to warm up with reverse engineering again.
# Enjoy :)


def lstrlenA(data: bytes, offset: int) -> int:
    ret = 0
    while data[offset + ret] != 0:
        ret += 1
    return ret


def c_to_str(data: bytes, offset: int) -> str:
    return data[offset : offset + lstrlenA(data, offset)].decode()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} [exe path] [res*.dll path]")
        sys.exit(1)

    # res*.dll is required because it contains the
    pe = pefile.PE(sys.argv[1])
    pe_mmap = pe.get_memory_mapped_image()
    res_pe = pefile.PE(sys.argv[2])
    res_pe_mmap = res_pe.get_memory_mapped_image()

    # GARR is added by GameHouse
    # It contains the modified import table and almost immediately jumps into the "res" dll
    garr = None
    for x in pe.sections:
        if x.Name == b"GARR\x00\x00\x00\x00" or x.Name == b".garr\x00\x00\x00":
            garr = x
            break
    if garr is None:
        print("GARR section not found, not a supported file!")
        sys.exit(1)

    # Verify that the entrypoint actually points to GARR
    ep = pe.OPTIONAL_HEADER.AddressOfEntryPoint
    if not garr.contains(ep):
        print("Entrypoint doesn't point into GARR, executable was probably modified!")
        sys.exit(1)

    # Get the location of the game info struct
    # It's probably the same between all games, but this is cleaner
    # GARR:00599000                 public start
    # GARR:00599000 start:
    # GARR:00599000                 push    offset dword_599080
    # GARR:00599005                 push    0
    # GARR:00599007                 call    ds:GetModuleHandleA_0
    # GARR:0059900D                 push    eax
    # GARR:0059900E                 call    ds:DoMessage
    first_instr = pe_mmap[ep : ep + 5]
    if first_instr[0] != 0x68:
        print("First instruction of the entrypoint wasn't a push [imm] instruction!")
        sys.exit(1)
    game_info_addr = int.from_bytes(first_instr[1:], "little")
    print(f"Game info address: {hex(game_info_addr)}")
    if not garr.contains(game_info_addr - pe.OPTIONAL_HEADER.ImageBase):
        print("Game info doesn't point into GARR, something went wrong!")
        sys.exit(1)

    # Parse the game info struct
    # Unfortunately, I can't just have hardcoded offsets for this stuff
    game_info = pe_mmap[game_info_addr - pe.OPTIONAL_HEADER.ImageBase :]
    cur_offset = 4
    print(f"Game name: {c_to_str(game_info, cur_offset)}")
    cur_offset += lstrlenA(game_info, cur_offset) + 1
    print(f"Game purchase URL: {c_to_str(game_info, cur_offset)}")
    cur_offset += lstrlenA(game_info, cur_offset) + 1
    # print(f"Useless shit: {c_to_str(game_info, cur_offset)}")
    while game_info[cur_offset] != 0:
        cur_offset += lstrlenA(game_info, cur_offset) + 1
    cur_offset += 1
    while game_info[cur_offset] != 0:
        cur_offset += lstrlenA(game_info, cur_offset) + 1
    encrypted_oep = int.from_bytes(
        game_info[cur_offset + 9 : cur_offset + 9 + 4], "little"
    )

    # Find the OEP decryption key
    # Luckily, the key is always right at the start of the .lock section, so no weird pattern matching is required
    lock = None
    for x in res_pe.sections:
        if x.Name == b".lock\x00\x00\x00":
            lock = x
            break
    if lock is None:
        print("_lock section not found, not a supported res dll!")
        sys.exit(1)
    lock_data = res_pe_mmap[lock.VirtualAddress :]
    oep_key_str = c_to_str(lock_data, 0)
    print(f"OEP decryption key: {oep_key_str}")

    # Decrypt the OEP
    oep_key = 0
    for x in oep_key_str:
        oep_key = (ord(x) | (oep_key << 8) | (oep_key >> 24)) & 0xFFFFFFFF
    print(hex(oep_key))
    oep = encrypted_oep ^ oep_key
    print(f"OEP: {hex(oep)}")

    # Save the OEP
    # The game should work perfectly fine at this point, but I want to completely remove the DRM for style!
    pe.OPTIONAL_HEADER.AddressOfEntryPoint = oep - pe.OPTIONAL_HEADER.ImageBase

    # Find the original import directory
    # The original import directory is completely intact and can be found via a simple byte search
    IMAGE_DIRECTORY_ENTRY_IMPORT = 1
    IMPORT_DESCRIPTOR_SIZE = 0x14
    garr_imports = pe.OPTIONAL_HEADER.DATA_DIRECTORY[IMAGE_DIRECTORY_ENTRY_IMPORT]
    orig_imports_addr = pe_mmap.find(
        pe_mmap[
            garr_imports.VirtualAddress : garr_imports.VirtualAddress
            + IMPORT_DESCRIPTOR_SIZE
        ]
    )
    print(f"Original import directory: {hex(orig_imports_addr)}")

    # Patch the executable to point to the original import directory
    pe.OPTIONAL_HEADER.DATA_DIRECTORY[
        IMAGE_DIRECTORY_ENTRY_IMPORT
    ].VirtualAddress = orig_imports_addr
    pe.OPTIONAL_HEADER.DATA_DIRECTORY[IMAGE_DIRECTORY_ENTRY_IMPORT].Size -= (
        IMPORT_DESCRIPTOR_SIZE * 2
    )

    # TODO: Actually delete the GARR section! I can't figure out how to properly do it with pefile...
    # But it works if you manually delete it with CFF Explorer

    fixed_filename = sys.argv[1][:-4] + "-fixed.exe"
    print(f"Saving to {fixed_filename}")
    pe.write(fixed_filename)
