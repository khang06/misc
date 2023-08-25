import sys
import json5 # pip install json5
import binascii
import itertools
import coff
from coff import Coff # pip install coff
from coff import IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE

# Common functions provided by thcrap
COMMON_IMPORTS = {
    "_malloc": "th_malloc",
    "_calloc": "th_calloc",
    "_realloc": "th_realloc",
    "_free": "th_free",
    "__msize": "th_msize",
    "__expand": "th_expand",
    "__aligned_malloc": "th_aligned_malloc",
    "__aligned_realloc": "th_aligned_realloc",
    "__aligned_free": "th_aligned_free",
    "__aligned_msize": "th_aligned_msize",
    "_memcpy": "th_memcpy",
    "_memmove": "th_memmove",
    "_memcmp": "th_memcmp",
    "_memset": "th_memset",
    "_memccpy": "th_memccpy",
    "_strdup": "th_strdup",
    "_strndup": "th_strndup",
    "_strdup_size": "th_strdup_size",
    "_strcmp": "th_strcmp",
    "_strncmp": "th_strncmp",
    "_stricmp": "th_stricmp",
    "_strnicmp": "th_strnicmp",
    "_strcpy": "th_strcpy",
    "_strncpy": "th_strncpy",
    "_strcat": "th_strcat",
    "_strncat": "th_strncat",
    "_strlen": "th_strlen",
    "_strnlen_s": "th_strnlen_s",
    "_sprintf": "th_sprintf",
    "_snprintf": "th_snprintf",
    "_sscanf": "th_sscanf",
    "_GetLastError@4": "th_GetLastError",
    "_GetProcAddress@4": "th_GetProcAddress",
    "_GetModuleHandleA@4": "th_GetModuleHandleA",
    "_GetModuleHandleW@4": "th_GetModuleHandleW",
}

class Config:
    def __init__(self, data: dict):
        self.input = data["input"]
        self.output = data["output"]

        if data["externs"] is None:
            self.externs = {}
        else:
            self.externs = {x: y["addr"] for x, y in data["externs"].items()}

        if data["binhacks"] is None:
            self.binhacks = {}
        else:
            self.binhacks = data["binhacks"]

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} [input json]")
        sys.exit(1)
    
    with open(sys.argv[1], "r") as file:
        config = Config(json5.load(file))
    with open(config.input, "rb") as file:
        raw_obj = file.read()
    obj = Coff(config.input)

    config.externs.update(COMMON_IMPORTS)
    for sym in itertools.chain(*obj.symtables.values()):
        config.externs[sym.name] = f"codecave:{obj.sections[sym.sectnum - 1].name}+{hex(sym.value)}"

    codecaves = {}
    for seckey in obj.relocs.keys():
        # TODO: support bss
        section = obj.sections[seckey]
        if section.size == 0:
            continue
        prot = ""
        if section.flags & IMAGE_SCN_MEM_READ:
            prot += "r"
        if section.flags & IMAGE_SCN_MEM_WRITE:
            prot += "w"
        if section.flags & IMAGE_SCN_MEM_EXECUTE:
            prot += "x"

        # TODO: support more reloc types
        code = binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
        relocs = sorted(obj.relocs[seckey], key=lambda rel: rel.vaddr, reverse=True)
        for reloc in relocs:
            if reloc.name in config.externs:
                assert reloc.size == 4
                offset = int.from_bytes(raw_obj[(section.offdata + reloc.vaddr):(section.offdata + reloc.vaddr + 4)], byteorder="little")
                match reloc.type:
                    case coff.IMAGE_REL_I386_DIR32:
                        replacement = f"<{config.externs[reloc.name]}>"
                    case coff.IMAGE_REL_I386_REL32:
                        replacement = f"[{config.externs[reloc.name]}]"
                    case _:
                        raise KeyError(f"Unhandled reloc type {hex(reloc.type)}")
                if offset != 0:
                    replacement += f"+{hex(offset)}"
                code = code[:(reloc.vaddr * 2)] + f"({replacement})" + code[(reloc.vaddr * 2 + 8):]
            else:
                raise KeyError(f"Unhandled reloc {reloc}")

        codecaves[section.name] = {
            "prot": prot,
            "code": code
        }

    # TODO: this is really jank and badly implemented
    for binhack in config.binhacks.values():
        code = binhack["code"]
        obj_pos = code.find("obj:")
        while obj_pos != -1:
            SEPARATORS = [' ', ')', ']', '}', '+']
            obj_end = obj_pos + len("obj:")
            while not code[obj_end] in SEPARATORS:
                obj_end += 1

            obj_name = code[(obj_pos + len("obj:")):obj_end]
            code = code[:obj_pos] + config.externs[obj_name] + code[obj_end:]
            
            obj_pos = code.find("obj:")
        binhack["code"] = code

    output_dict = {
        "codecaves": codecaves,
        "binhacks": config.binhacks
    }
    with open(config.output, "w") as output:
        json5.dump(output_dict, output, indent=4)
