import sys
import json5 # pip install json5
import binascii
import itertools
import coff
from coff import Coff # pip install coff
from coff import IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE, IMAGE_SCN_CNT_UNINITIALIZED_DATA

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
}

class Extern:
    def __init__(self, addr: str, offset: int):
        self.addr = addr
        self.offset = offset
    
    def __repr__(self):
        return f"{self.addr}+{hex(self.offset)}"

class Config:
    def __init__(self, data: dict):
        self.input = data["input"]
        self.output = data["output"]
        self.prefix = data["prefix"]
        self.externs = {x: Extern(y["addr"], y.get("offset", 0)) for x, y in data.get("externs", {}).items()}
        self.binhacks = data.get("binhacks", {})
        self.imports = data.get("imports", {})

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} [input json]")
        sys.exit(1)
    
    with open(sys.argv[1], "r") as file:
        config = Config(json5.load(file))
    with open(config.input, "rb") as file:
        raw_obj = file.read()
    obj = Coff(config.input)

    codecaves = {}
    section_merges = dict()
    for i, section in enumerate(obj.sections):
        if section.size == 0 or section.name.startswith("/") or section.name in [".drectve"]:
            continue
        prot = ""
        if section.flags & IMAGE_SCN_MEM_READ:
            prot += "r"
        if section.flags & IMAGE_SCN_MEM_WRITE:
            prot += "w"
        if section.flags & IMAGE_SCN_MEM_EXECUTE:
            prot += "x"
        if config.prefix + section.name in codecaves:
            section_merges[i] = len(codecaves[config.prefix + section.name]["code"]) // 2
            codecaves[config.prefix + section.name]["code"] += binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
        else:
            if section.flags & IMAGE_SCN_CNT_UNINITIALIZED_DATA:
                codecaves[config.prefix + section.name] = {
                    "prot": prot,
                    "size": section.size
                }
            else:
                codecaves[config.prefix + section.name] = {
                    "prot": prot,
                    "code": binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
                }
            config.externs[section.name] = Extern(f"codecave:{config.prefix}{section.name}", 0)

    config.externs.update({k: Extern(v, 0) for k, v in COMMON_IMPORTS.items()})
    for sym in itertools.chain(*obj.symtables.values()):
        config.externs[sym.name] = Extern(f"codecave:{config.prefix}{obj.sections[sym.sectnum - 1].name}", sym.value + section_merges.get(sym.sectnum - 1, 0))

    # TODO: handle import errors
    if len(config.imports) != 0 or "_coff2binhack_init" in config.externs:
        init_data = bytearray()
        init_strs = bytearray()
        import_count = 0
        str_offset = 0
        for dll, imports in config.imports.items():
            init_strs += dll.encode(encoding="ascii") + b"\x00"
            init_data += str_offset.to_bytes(4, 'little') + len(imports).to_bytes(4, 'little')
            str_offset += len(dll) + 1
            for imp, imp_data in imports.items():
                init_strs += imp.encode(encoding="ascii") + b"\x00"
                init_data += str_offset.to_bytes(4, 'little')
                str_offset += len(imp) + 1

                config.externs["__imp_" + imp_data.get("alias", imp)] = Extern(f"codecave:{config.prefix}_imports", import_count * 4)
                import_count += 1
        init_data += b"\xFF\xFF\xFF\xFF"

        str_base = len(init_data)
        init_data += init_strs

        # See load_imports.asm
        init_code = "53555657bb41414141bd424242428b0383f8ff7432054343434350e82444444489c68b7b0483c308ff338104244343434356e80e45454589450083c50483c30483ef0174c9ebe1464646465f5e5d5bc3"
        init_code, init_end = init_code.split("46464646")
        init_code = init_code.replace("41414141", f"<codecave:{config.prefix}_init_data>")
        init_code = init_code.replace("42424242", f"<codecave:{config.prefix}_imports>")
        init_code = init_code.replace("43434343", f"(<codecave:{config.prefix}_init_data>+{hex(str_base)})")
        init_code = init_code.replace("24444444", "[th_GetModuleHandleA]") # Remember that these are relative!!!
        init_code = init_code.replace("0e454545", "[th_GetProcAddress]")

        if "_coff2binhack_init" in config.externs:
            # call _coff2binhack_init
            extern = config.externs["_coff2binhack_init"]
            init_code += f"e8([{extern.addr}]+{hex(extern.offset)})"
        init_code += init_end

        codecaves[config.prefix + "_patch_init"] = {
            "prot": "rx",
            "code": init_code,
            "export": True
        }
        if len(config.imports) != 0:
            codecaves.update({
                config.prefix + "_init_data": {
                    "prot": "r",
                    "code": binascii.hexlify(init_data).decode()
                },
                config.prefix + "_imports": {
                    "prot": "rw",
                    "size": import_count * 4
                }
            })

    for seckey in obj.relocs.keys():
        section = obj.sections[seckey]
        relocs = sorted(obj.relocs[seckey], key=lambda rel: rel.vaddr, reverse=True)
        for reloc in relocs:
            if reloc.name in config.externs:
                extern = config.externs[reloc.name]
                assert reloc.size == 4
                offset = int.from_bytes(raw_obj[(section.offdata + reloc.vaddr):(section.offdata + reloc.vaddr + 4)], byteorder="little") + extern.offset
                match reloc.type:
                    case coff.IMAGE_REL_I386_DIR32:
                        if offset == 0:
                            replacement = f"<{extern.addr}>"
                        else:
                            replacement = f"(<{extern.addr}>+{hex(offset)})"
                    case coff.IMAGE_REL_I386_REL32:
                        if offset == 0:
                            replacement = f"[{extern.addr}]"
                        else:
                            replacement = f"([{extern.addr}]+{hex(offset)})"
                    case _:
                        raise KeyError(f"Unhandled reloc type {hex(reloc.type)}")
                code = codecaves[config.prefix + section.name]["code"]
                codecaves[config.prefix + section.name]["code"] = code[:(reloc.vaddr * 2)] + replacement + code[(reloc.vaddr * 2 + 8):]
            else:
                raise KeyError(f"Unhandled reloc {reloc}")

    # TODO: this is really jank and badly implemented
    for binhack in config.binhacks.values():
        code = binhack["code"]
        obj_pos = code.find("obj:")
        while obj_pos != -1:
            SEPARATORS = [' ', ')', ']', '}', '+']
            obj_end = obj_pos + len("obj:")
            while not code[obj_end] in SEPARATORS:
                obj_end += 1

            # TODO: figure out how to implement this with the newer codecave reference syntax
            extern = config.externs[code[(obj_pos + len("obj:")):obj_end]]
            replacement = extern.addr
            if extern.offset != 0:
                replacement += f"+{hex(extern.offset)}"
            code = code[:obj_pos] + replacement + code[obj_end:]
            
            obj_pos = code.find("obj:")
        binhack["code"] = code

    output_dict = {
        "codecaves": codecaves,
        "binhacks": config.binhacks
    }
    with open(config.output, "w") as output:
        json5.dump(output_dict, output, indent=4)
