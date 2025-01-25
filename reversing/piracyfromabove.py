# Old versions of Piano From Above (pre-1.1.0) had a premium version, which was later merged into the free version.
# I'm sure keygens for software that's already free are in high demand!

# z3 is way slower
#from z3 import *
from cvc5.pythonic import *

LICENSE_NAME = "khangaroo"

def rshash(string):
    ret = 0
    state = 0xF8C9
    for x in string:
        ret = (ord(x) + state * ret) & 0xFFFFFFFF
        state = (state * 0x5C6B7) & 0xFFFFFFFF
    return ret

def bit_mix(dst, dst_bit, src, src_bit):
    dst_idx = dst_bit.bit_length() - 1
    src_idx = src_bit.bit_length() - 1
    bit = Extract(src_idx, src_idx, src)
    if dst_idx == 7:
        return Concat(bit, Extract(6, 0, dst))
    elif dst_idx == 0:
        return Concat(Extract(7, 1, dst), bit)
    else:
        return Concat(Extract(7, dst_idx + 1, dst), bit, Extract(dst_idx - 1, 0, dst))

def char_to_int(src):
    return SignExt(24, src)

def bit_to_int(src, src_idx):
    return ZeroExt(31, Extract(src_idx, src_idx, src))

s = Solver()
key = [BitVec(f"key_{i}", 8) for i in range(13)]
decoded = [BitVecVal(0, 8) for _ in range(6)]

SHUFFLE_TABLE = [3, 88, 46, 35, 85, 74, 36, 13, 58, 82, 10, 8, 90, 73, 43, 27, 72, 69, 76, 15, 20, 6, 40, 71, 99, 1, 50, 29, 21, 63, 16, 31, 59, 54, 91, 56, 51, 78, 93, 37, 65, 87, 67, 12, 52, 23, 25, 75]

cur_idx = 0
for x in SHUFFLE_TABLE:
    decoded[cur_idx >> 3] = bit_mix(decoded[cur_idx >> 3], 1 << (cur_idx & 7), key[x >> 3], 1 << (x & 7))
    cur_idx += 1

expected_hash = rshash(LICENSE_NAME.lower())
name_hash = bit_mix(key[4] >> 7, 2, key[5], 0x10)
name_hash = bit_mix(name_hash, 4, key[4], 4)
name_hash = bit_mix(name_hash, 8, key[1], 2)
name_hash = bit_mix(name_hash, 0x10, key[10], 8)
name_hash = bit_mix(name_hash, 0x20, key[3], 4)
name_hash = bit_mix(name_hash, 0x40, key[0], 4)
name_hash = bit_mix(name_hash, 0x80, key[8], 0x10)
s.add(name_hash == ((expected_hash >> 24) ^ (expected_hash >> 16) ^ (expected_hash >> 8) ^ expected_hash) & 0xFF)

s.add(
    char_to_int(decoded[5]) + 0xD64C650F * char_to_int(decoded[4]) - 0x18A808D9 *
        (char_to_int(decoded[3]) + 0x77F8F5DF *
            (char_to_int(decoded[2]) - 0x41221DE7 * (char_to_int(decoded[1]) - 0x62F5B251 * char_to_int(decoded[0]))))
    == 0xAAAAAAAA
)
s.add(Extract(0, 0, decoded[0]) == 1)
s.add(
    ((
        bit_to_int(decoded[0], 0) +
        bit_to_int(decoded[0], 5) +
        bit_to_int(decoded[1], 1) +
        bit_to_int(decoded[4], 5) +
        bit_to_int(decoded[5], 1) +
        bit_to_int(decoded[5], 6) +
        bit_to_int(decoded[2], 6) +
        bit_to_int(decoded[3], 3) +
        bit_to_int(decoded[3], 4) +
        bit_to_int(decoded[3], 6) +
        bit_to_int(decoded[0], 3) +
        bit_to_int(decoded[0], 4) +
        bit_to_int(decoded[0], 6) +
        bit_to_int(decoded[2], 7) +
        (
            bit_to_int(decoded[1], 3) +
            bit_to_int(decoded[4], 1) +
            bit_to_int(decoded[5], 7)
        ) * 2
    ) & 3) != 0
)
s.add(
    ((
        bit_to_int(decoded[0], 6) * 2 +
        bit_to_int(decoded[1], 6) * 4 +
        bit_to_int(decoded[3], 2) +
        bit_to_int(decoded[1], 7) * 2 +
        bit_to_int(decoded[1], 0) * 4 * bit_to_int(decoded[4], 0)
    ) % 3) != 0
)
s.add(((decoded[1] ^ ((decoded[3] ^ decoded[2] ^ ((decoded[1] ^ ((decoded[0] ^ decoded[4] ^ decoded[2] ^ ((decoded[3] ^ decoded[2] ^ ((decoded[4] ^ decoded[3] ^ decoded[1] ^ ((decoded[1] ^ (decoded[4] >> 1)) >> 1)) >> 1)) >> 1)) >> 1)) >> 1)) >> 1)) & 1) != 0)
s.add(
    ((
        bit_to_int(key[10], 0) +
        bit_to_int(key[5], 7) +
        bit_to_int(key[10], 4) +
        bit_to_int(key[12], 1) +
        bit_to_int(key[12], 2) +
        bit_to_int(key[5], 5) +
        bit_to_int(key[2], 1) +
        bit_to_int(key[2], 3) +
        bit_to_int(key[4], 0) +
        bit_to_int(key[3], 0)
    ) % 3) != 0
)
s.add(((key[8] & 4) != 0) == ((key[3] & 0x10) != 0))
s.add(((key[8] & 1) != 0) == ((key[12] & 1) != 0))
s.add(((key[10] & 2) != 0) == ((key[1] & 0x40) != 0))
s.add(((key[3] & 0x40) != 0) != ((key[0] & 0x10) != 0))
s.add(((LShR(key[7], 1) & 1) + (LShR(key[7], 4) & 4) + (LShR(key[6], 4) & 2)) > ((LShR(key[5], 3) & 4) + (LShR(key[12], 1) & 1) + (key[2] & 2) - 1))
s.add(LShR(key[5], 7) != ((key[10] ^ (key[4] ^ key[3] ^ ((key[2] ^ key[12] ^ ((key[12] ^ ((key[2] ^ ((key[10] ^ ((key[5] ^ (key[11] >> 1)) >> 1)) >> 1)) >> 1)) >> 1)) >> 1))) & 1))
s.add((key[6] & 1) != 0)

checkvar1 = bit_mix((key[3] & 0x10) >> 4, 2, key[4], 2)
checkvar1 = bit_mix(checkvar1, 4, key[2], 4)
checkvar1 = bit_mix(checkvar1, 8, key[5], 4)
checkvar1 = bit_mix(checkvar1, 0x10, key[7], 0x10)
checkvar1 = bit_mix(checkvar1, 0x20, key[1], 0x40)
checkvar1 = bit_mix(checkvar1, 0x40, key[11], 0x80)
checkvar1 = bit_mix(checkvar1, 0x80, key[8], 0x40)
s.add(checkvar1 == 162)

checkvar2 = bit_mix((key[0] & 0x10) >> 4, 2, key[10], 0x40)
checkvar2 = bit_mix(checkvar2, 4, key[9], 0x20)
checkvar2 = bit_mix(checkvar2, 8, key[10], 2)
checkvar2 = bit_mix(checkvar2, 0x10, key[5], 2)
checkvar2 = bit_mix(checkvar2, 0x20, key[11], 2)
checkvar2 = bit_mix(checkvar2, 0x40, key[1], 8)
checkvar2 = bit_mix(checkvar2, 0x80, key[6], 2)
s.add(checkvar2 == 75)

checkvar3 = bit_mix((key[2] & 0x40) >> 6, 2, key[11], 0x10)
checkvar3 = bit_mix(checkvar3, 4, key[8], 4)
checkvar3 = bit_mix(checkvar3, 8, key[8], 1)
checkvar3 = bit_mix(checkvar3, 0x10, key[12], 1)
checkvar3 = bit_mix(checkvar3, 0x20, key[0], 0x20)
checkvar3 = bit_mix(checkvar3, 0x40, key[9], 0x80)
checkvar3 = bit_mix(checkvar3, 0x80, key[3], 0x40)
s.add(checkvar3 == 33)

res = s.check()
if res == sat:
    model = s.model()
    for k in reversed(key):
        if model[k] is None:
            print("00", end="")
        else:
            print(f"{model[k].as_long():02X}", end="")
    print()
else:
    print("unsat!")
