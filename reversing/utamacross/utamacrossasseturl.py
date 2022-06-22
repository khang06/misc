import os

BASE_URL = "https://assets-sakasho.cdn-dena.com/1246/20220620194610"
HASH_KEY = 0x20619cad

# fnv32 variant?
def hash_str(input: str) -> int:
    state = 0x811C9DC5
    for x in map(ord, input):
        state = (0x1000193 * (state ^ x)) & 0xFFFFFFFF
    return state ^ HASH_KEY

def get_url_from_path(input: str) -> str:
    base = os.path.splitext(input)[0]
    ext = os.path.splitext(input)[1]
    return f"{BASE_URL}{base}!s{hash_str(input):x}z!{ext}"

print(get_url_from_path("/android/ct/dv/me/01/01_01.xab"))