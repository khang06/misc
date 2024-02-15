import binascii
import zlib
import sys

STATIC_KEY = binascii.unhexlify("6fb2884d47ef54fcd6cc05855adeb1c455d9b28a8277a313ce745eaa317ff40739ce260b95f29a0b3f57c3de0fc9c157808e84135590cb3f22fa23eb1ec4b4903f18c21aa36821208644712031acc38957b4e1c4215aa8a852609c6515022c6b27244882c9")
CRYPT_MAGIC = b"DTH_FILE_00"
FILE_KEY_LEN = 0x0B
CRYPT_HEADER_LEN = len(CRYPT_MAGIC) + FILE_KEY_LEN
COMPRESS_MAGIC = b"DTH_FILE_COMPRESSED_ENCRYPTED_10"
COMPRESS_HEADER_LEN = len(COMPRESS_MAGIC) + 4

def dth_crypt(buf, file_key, file_key_offset, static_key_offset):
    ret = bytearray(buf)
    for i in range(len(buf)):
        ret[i] ^= file_key[(i + file_key_offset) % len(file_key)] ^ STATIC_KEY[(i + static_key_offset) % len(STATIC_KEY)]
    return ret

if __name__ == "__main__":
    if len(sys.argv) != 4 or (sys.argv[1] != "d" and sys.argv[1] != "e"):
        print(f"Usage: {sys.argv[0]} [d|e] [in] [out]")
        sys.exit(1)

    if sys.argv[1] == "d":
        with open(sys.argv[2], "rb") as file:
            buf = file.read()

        file_key = buf[len(CRYPT_MAGIC):CRYPT_HEADER_LEN]
        magic = dth_crypt(buf[:len(CRYPT_MAGIC)], file_key, file_key[0], file_key[1])
        assert magic == CRYPT_MAGIC

        # Yes, this uses parts of the magic as the offsets
        compress_header = dth_crypt(buf[CRYPT_HEADER_LEN:(CRYPT_HEADER_LEN + COMPRESS_HEADER_LEN)], file_key, magic[2], magic[3])
        assert compress_header[:len(COMPRESS_MAGIC)] == COMPRESS_MAGIC
        out_buf = zlib.decompress(dth_crypt(buf[(CRYPT_HEADER_LEN + COMPRESS_HEADER_LEN):], file_key, magic[2] + COMPRESS_HEADER_LEN, magic[3] + COMPRESS_HEADER_LEN),
                                bufsize=int.from_bytes(compress_header[len(COMPRESS_MAGIC):], "little"))

        with open(sys.argv[3], "wb") as file:
            file.write(out_buf)
    elif sys.argv[1] == "e":
        with open(sys.argv[2], "rb") as file:
            input = file.read()
        with open(sys.argv[3], "wb") as file:
            compressed = zlib.compress(input)
            file_key = b"A" * FILE_KEY_LEN
            file.write(dth_crypt(CRYPT_MAGIC, file_key, file_key[0], file_key[1]))
            file.write(file_key)
            file.write(dth_crypt(COMPRESS_MAGIC, file_key, CRYPT_MAGIC[2], CRYPT_MAGIC[3]))
            file.write(dth_crypt(len(input).to_bytes(4, "little"), file_key, CRYPT_MAGIC[2] + len(COMPRESS_MAGIC), CRYPT_MAGIC[3] + len(COMPRESS_MAGIC)))
            file.write(dth_crypt(compressed, file_key, CRYPT_MAGIC[2] + COMPRESS_HEADER_LEN, CRYPT_MAGIC[3] + COMPRESS_HEADER_LEN))
