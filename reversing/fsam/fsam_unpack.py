# Unpacks the compressed ELF from the individual FSAM binaries
# ...after extracting them with fsam_extract.py, because I guess one layer wasn't enough for them :p

import sys
import lzma

with open(sys.argv[1], "rb") as f:
    data = f.read()

decompressed_len = int.from_bytes(data[-8:-4], "big")
compressed_len = int.from_bytes(data[-4:], "big")
print(f"compressed_len: {hex(compressed_len)}, decompressed_len: {hex(decompressed_len)}")

compressed = bytearray(data[-8 - compressed_len:-8])
if compressed_len == decompressed_len:
    decompressed = compressed
else:
    compressed[:6] = bytes.fromhex("FD377A585A00")
    compressed[-2:] = b"YZ"
    decompressed = lzma.decompress(compressed, format=lzma.FORMAT_XZ)

with open(sys.argv[1] + ".unpacked", "wb") as f:
    f.write(decompressed)
print(f"unpacked to {sys.argv[1]}.unpacked")
