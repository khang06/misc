# File extractor for the NFA0 file format, seen in the "Nemea" engine(?)
# Specifically for 不思議の幻想郷CHRONICLE -クロニクル-, but should work on other stuff too

import sys
import os

if len(sys.argv) < 3:
    print(f"Usage: {sys.argv[0]} [bin file] [output dir]")

# XOR key is hardcoded to 0xE6 in game code, where it then gets XORed with 0xEE in engine code, ending up with a key of 0x08
def xor_buf(buf):
    ret = bytearray(buf)
    for i in range(len(buf)):
        ret[i] ^= 0x08
    return ret

files = []
with open(sys.argv[1], "rb") as bin:
    assert bin.read(4) == b"NFA0"
    assert bin.read(4) == b"\x01\x00\x00\x00"
    file_count = int.from_bytes(bin.read(4), "little")
    assert bin.read(4) == b"\x01\x00\x00\x00"

    for _ in range(file_count):
        bin.read(8) # checksum?
        size = int.from_bytes(xor_buf(bin.read(4)), "little")
        offset = int.from_bytes(xor_buf(bin.read(4)), "little")
        bin.read(4) # unknown
        name = (xor_buf(bin.read(128)).split(b"\x00\x00")[0] + b"\x00").decode("utf-16-le") # pretty cursed
        files.append({
            "name": name,
            "size": size,
            "offset": offset,
        })
    
    for file in files:
        print(file["name"])
        target_path = os.path.join(sys.argv[2], file["name"])
        os.makedirs(os.path.dirname(target_path), exist_ok=True)
        with open(target_path, "wb") as target:
            bin.seek(file["offset"])
            target.write(xor_buf(bin.read(file["size"])))
